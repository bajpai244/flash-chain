use clap::Parser;
use flash_batcher::exex;
use flash_chainspec::FlashChainSpecParser;
use reth_optimism_cli::Cli;
use reth_optimism_node::{OpNode, args::RollupArgs};
use tracing::info;

fn main() {
    reth_cli_util::sigsegv_handler::install();

    if let Err(err) =
        Cli::<FlashChainSpecParser, RollupArgs>::parse().run(async move |builder, rollup_args| {
            info!(target: "reth::cli", "Launching node");

            let node = OpNode::new(rollup_args);

            let handle = builder
                .node(node)
                .install_exex("exex", async move |ctx| Ok(exex(ctx)))
                .launch_with_debug_capabilities()
                .await?;

            handle.node_exit_future.await
        })
    {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
