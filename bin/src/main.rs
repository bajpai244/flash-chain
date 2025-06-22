use std::sync::{Arc, Mutex};

use clap::Parser;
use flash_batcher::{BatcherExEx, channel_builder::ChannelBuilder, db::DB};
use flash_chainspec::FlashChainSpecParser;
use reth_optimism_cli::Cli;
use reth_optimism_node::{OpNode, args::RollupArgs};
use tracing::{info, error};

fn main() {
    reth_cli_util::sigsegv_handler::install();

    let db = match DB::new("batcher.db") {
        Ok(db) => Arc::new(Mutex::new(db)),
        Err(e) => {
            error!("Failed to create database: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = db.lock().unwrap().initialize_database() {
        error!("Failed to initialize database schema: {}", e);
        std::process::exit(1);
    }

    // Validate batch size
    const BATCH_SIZE: u64 = 10;
    if BATCH_SIZE == 0 {
        error!("Batch size must be greater than 0");
        std::process::exit(1);
    }

    let channel_builder = ChannelBuilder::new(db, BATCH_SIZE);
    info!("Initialized channel builder with batch size: {}", BATCH_SIZE);

    if let Err(err) =
        Cli::<FlashChainSpecParser, RollupArgs>::parse().run(async move |builder, rollup_args| {
            info!(target: "reth::cli", "Launching node with flash batcher");

            let node = OpNode::new(rollup_args);

            let handle = builder
                .node(node)
                .install_exex(
                    "flash-batcher",
                    |ctx| async move { BatcherExEx::new(ctx, channel_builder).await },
                )
                .launch_with_debug_capabilities()
                .await?;

            info!("Flash chain node started successfully");
            handle.node_exit_future.await
        })
    {
        error!("Node startup failed: {err:?}");
        std::process::exit(1);
    }
}
