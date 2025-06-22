use alloy_primitives::Sealed;
use futures::{Future, TryStreamExt};
use reth::{
    core::primitives::{AlloyBlockHeader, serde_bincode_compat::RecoveredBlock},
    rpc::eth::EthApiServer,
};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::{FullNodeComponents, NodeTypes};
use reth_primitives_traits::Block;
use serde_json::to_vec;
use tracing::info;

use crate::db::BlockData;
use reth_primitives::{EthPrimitives, SealedBlock};

pub mod batcher;
pub mod db;

fn serialize_block<B: Block>(block: &SealedBlock<B>) -> anyhow::Result<Vec<u8>>
where
    SealedBlock<B>: serde::Serialize,
{
    Ok(serde_json::to_vec(block)?)
}

pub async fn exex<Node: FullNodeComponents>(mut ctx: ExExContext<Node>) -> eyre::Result<()> {
    while let Some(notification) = ctx.notifications.try_next().await? {
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

                    // let block_data = BlockData { block_number: ,
                    //     block_hash: format!("{:?}", block.hash()),
                    //     block_data: serialize_block(block)?,
                    //     timestamp: block.timestamp() as i64,
                    //     batch_id: None,
                    // };
                    // send block to batcher
                }

                // start the batcher submission loop in the background

                info!(committed_chain = ?new.range(), "Received commit");
            }
            ExExNotification::ChainReorged { old, new } => {
                info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
            }
            ExExNotification::ChainReverted { old } => {
                info!(reverted_chain = ?old.range(), "Received revert");
            }
        };

        if let Some(committed_chain) = notification.committed_chain() {
            ctx.events
                .send(ExExEvent::FinishedHeight(committed_chain.tip().num_hash()))?;
        }
    }

    Ok(())
}
