use std::sync::{Arc, Mutex};

use clap::Parser;
use flash_batcher::{BatcherExEx, channel_builder::ChannelBuilder, db::DB};
use flash_chainspec::FlashChainSpecParser;
use reth_optimism_cli::Cli;
use reth_optimism_node::{OpNode, args::RollupArgs};
use tracing::info;

fn main() {
    reth_cli_util::sigsegv_handler::install();

    let db = Arc::new(Mutex::new(DB::new("batcher.db")));
    db.lock().unwrap().initialize_database().unwrap();

    let channel_builder = ChannelBuilder::new(db, 10);

    if let Err(err) =
        Cli::<FlashChainSpecParser, RollupArgs>::parse().run(async move |builder, rollup_args| {
            info!(target: "reth::cli", "Launching node");

            let node = OpNode::new(rollup_args);

            let handle = builder
                .node(node)
                .install_exex(
                    "exex",
                    |ctx| async move { BatcherExEx::new(ctx, channel_builder).await },
                )
                .launch_with_debug_capabilities()
                .await?;

            handle.node_exit_future.await
        })
    {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
