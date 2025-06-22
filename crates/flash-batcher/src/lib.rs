use futures::{FutureExt, TryStreamExt};
use reth::core::primitives::AlloyBlockHeader;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::FullNodeComponents;
use reth_primitives_traits::Block;
use std::sync::{Arc, Mutex};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, ready},
};
use tracing::{info, debug, error, warn};

use crate::channel_builder::ChannelBuilder;
use crate::db::{BatchStatus, BlockData, DB};
use reth_primitives::SealedBlock;

pub mod channel_builder;
pub mod db;

fn serialize_block<B: Block>(block: &SealedBlock<B>) -> anyhow::Result<Vec<u8>>
where
    SealedBlock<B>: serde::Serialize,
{
    serde_json::to_vec(block).map_err(|e| anyhow::anyhow!("Failed to serialize block: {}", e))
}

pub struct BatcherExEx<Node: FullNodeComponents> {
    ctx: ExExContext<Node>,
    channel_builder: ChannelBuilder,
}

impl<Node: FullNodeComponents> BatcherExEx<Node> {
    pub async fn new(ctx: ExExContext<Node>, channel_builder: ChannelBuilder) -> eyre::Result<Self> {
        Ok(Self { ctx, channel_builder })
    }
}

impl<Node: FullNodeComponents> Future for BatcherExEx<Node> {
    type Output = eyre::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        while let Some(notification) = ready!(this.ctx.notifications.try_next().poll_unpin(cx))? {
            match &notification {
                ExExNotification::ChainCommitted { new } => {
                    for block in new.blocks_iter() {
                        let data = match serialize_block(block.sealed_block()) {
                            Ok(data) => data,
                            Err(e) => {
                                error!("Failed to serialize block {}: {}", block.number(), e);
                                continue; // Skip this block but continue processing
                            }
                        };

                        let block_data = BlockData {
                            block_number: block.number(),
                            block_hash: block.hash().to_string(),
                            timestamp: block.timestamp(),
                            block_data: data,
                            batch_id: None,
                        };

                        this.channel_builder.add_block(block_data.clone());
                        debug!(
                            "Added block {} to queue. Pending: {}/{}", 
                            block.number(),
                            this.channel_builder.pending_blocks().len(),
                            this.channel_builder.batch_size()
                        );

                        if this.channel_builder.pending_blocks().len() >= this.channel_builder.batch_size() as usize {
                            debug!("Batch size reached, creating batch...");
                            
                            if let Err(e) = this.channel_builder.insert_batch() {
                                error!("Failed to insert batch: {}", e);
                                continue;
                            }
                            
                            this.channel_builder.clear_queue();

                            let db = this.channel_builder.db();
                            if let Err(e) = submit_batches(db) {
                                error!("Failed to submit batches: {}", e);
                            }
                        }

                        debug!("Processed block: {}", block.number());
                    }

                    info!(committed_chain = ?new.range(), "Received commit");

                    this.ctx
                        .events
                        .send(ExExEvent::FinishedHeight(new.tip().num_hash()))?;
                }
                ExExNotification::ChainReorged { old, new } => {
                    warn!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
                }
                ExExNotification::ChainReverted { old } => {
                    warn!(reverted_chain = ?old.range(), "Received revert");
                }
            };
        }

        Poll::Ready(Ok(()))
    }
}

fn submit_batches(db: Arc<Mutex<DB>>) -> eyre::Result<()> {
    let db = db.lock().map_err(|_| eyre::eyre!("Database lock poisoned"))?;
    
    let batches = db.get_pending_batches()
        .map_err(|e| eyre::eyre!("Failed to get pending batches: {}", e))?;

    debug!("Found {} pending batches to submit", batches.len());

    for batch in batches {
        // NOTE: right now we are not submitting the batches to the flash chain, we are just marking them as submitted
        // ideally we would make a call via the celestia-client to submit the batches for the flash chain
        debug!("Processing batch: {} with {} blocks", batch.id, batch.block_numbers.len());

        if let Err(e) = db.update_batch_status(&batch.id, BatchStatus::Submitted) {
            error!("Failed to update batch status for {}: {}", batch.id, e);
            continue;
        }
        
        info!("Successfully submitted batch: {}", batch.id);
    }

    info!("Batch submission completed");
    Ok(())
}
