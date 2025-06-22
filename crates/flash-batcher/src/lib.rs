use futures::{FutureExt, TryStreamExt};
use reth::core::primitives::AlloyBlockHeader;
use reth::{builder::NodeTypes, primitives::EthPrimitives};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::FullNodeComponents;
use reth_primitives_traits::Block;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, ready},
};
use tracing::info;

use crate::batcher::Batcher;
use crate::db::{BatchStatus, BlockData, DB};
use reth_primitives::SealedBlock;

pub mod batcher;
pub mod db;

fn serialize_block<B: Block>(block: &SealedBlock<B>) -> anyhow::Result<Vec<u8>>
where
    SealedBlock<B>: serde::Serialize,
{
    Ok(serde_json::to_vec(block)?)
}

pub struct BatcherExEx<Node: FullNodeComponents> {
    ctx: ExExContext<Node>,
    batcher: Batcher,
}

impl<Node: FullNodeComponents> BatcherExEx<Node> {
    pub async fn new(ctx: ExExContext<Node>, batcher: Batcher) -> eyre::Result<Self> {
        Ok(Self { ctx, batcher })
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
                        // TODO: remove unwrap?
                        let data = serialize_block(block.sealed_block()).unwrap();

                        let block_data = BlockData {
                            block_number: block.number(),
                            block_hash: block.hash().to_string(),
                            timestamp: block.timestamp(),
                            block_data: data,
                            batch_id: None,
                        };

                        // TODO: remove clone?
                        this.batcher.add_block(block_data.clone());
                        println!(
                            "pending blocks length: {:?}, batch size: {:?}",
                            this.batcher.pending_blocks().len(),
                            this.batcher.batch_size()
                        );

                        if this.batcher.pending_blocks().len() >= this.batcher.batch_size() as usize
                        {
                            // TODO: remove unwrap?
                            this.batcher.insert_batch().unwrap();
                            this.batcher.clear_queue();

                            let db = this.batcher.db();

                            // submit the batch
                            submit_batches(db)?;
                        }

                        println!("block_data: {:?}", block_data);
                    }

                    // start the batcher submission loop in the background

                    info!(committed_chain = ?new.range(), "Received commit");

                    this.ctx
                        .events
                        .send(ExExEvent::FinishedHeight(new.tip().num_hash()))?;
                }
                ExExNotification::ChainReorged { old, new } => {
                    info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
                }
                ExExNotification::ChainReverted { old } => {
                    info!(reverted_chain = ?old.range(), "Received revert");
                }
            };
        }

        Poll::Ready(Ok(()))
    }
}

fn submit_batches(db: Arc<Mutex<DB>>) -> eyre::Result<()> {
    let db = db.lock().unwrap();
    let batches = db.get_pending_batches()?;

    for batch in batches {
        // NOTE: right now we are not submitting the batches to the flash chain, we are just printing them to the console
        // ideally we would make a call via the celestia-client to submit the batches for the flash chain
        println!("batch submitted: {:?}", batch);

        db.update_batch_status(&batch.id, BatchStatus::Submitted)?;
    }

    info!("batches submitted ........");

    Ok(())
}
