use futures::{FutureExt, TryStreamExt};
use reth::core::primitives::AlloyBlockHeader;
use reth::{builder::NodeTypes, primitives::EthPrimitives};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::FullNodeComponents;
use reth_primitives_traits::Block;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, ready},
};
use tracing::info;

use crate::db::BlockData;
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
}

impl<Node: FullNodeComponents> BatcherExEx<Node> {
    pub async fn new(ctx: ExExContext<Node>) -> eyre::Result<Self> {
        Ok(Self { ctx })
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

// pub async fn exex<Node: FullNodeComponents>(mut ctx: ExExContext<Node>) -> eyre::Result<()> {
//     while let Some(notification) = ctx.notifications.try_next().await? {
//         match &notification {
//             ExExNotification::ChainCommitted { new } => {
//                 for block in new.blocks_iter() {
//                     // TODO: remove unwrap?
//                     let data = serialize_block(block.sealed_block()).unwrap();

//                     let block_data = BlockData {
//                         block_number: block.number(),
//                         block_hash: block.hash().to_string(),
//                         timestamp: block.timestamp(),
//                         block_data: data,
//                         batch_id: None,
//                     };

//                     println!("block_data: {:?}", block_data);
//                 }

//                 // start the batcher submission loop in the background

//                 info!(committed_chain = ?new.range(), "Received commit");
//             }
//             ExExNotification::ChainReorged { old, new } => {
//                 info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
//             }
//             ExExNotification::ChainReverted { old } => {
//                 info!(reverted_chain = ?old.range(), "Received revert");
//             }
//         };

//         if let Some(committed_chain) = notification.committed_chain() {
//             ctx.events
//                 .send(ExExEvent::FinishedHeight(committed_chain.tip().num_hash()))?;
//         }
//     }

//     Ok(())
// }
