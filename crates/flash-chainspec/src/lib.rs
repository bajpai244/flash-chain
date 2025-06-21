//! Odyssey chainspec parsing logic.
use reth_op::chainspec::OpChainSpec;
// OpHardfork needs to be imported directly
use reth_cli::chainspec::{ChainSpecParser, parse_genesis};
use std::sync::Arc;

use crate::chainspec::FLASH_CHAIN;

pub mod chainspec;

/// Odyssey chain specification parser.
#[derive(Debug, Clone, Default)]
pub struct FlashChainSpecParser;

impl ChainSpecParser for FlashChainSpecParser {
    type ChainSpec = OpChainSpec;

    const SUPPORTED_CHAINS: &'static [&'static str] = &["flash"];

    fn parse(s: &str) -> eyre::Result<Arc<Self::ChainSpec>> {
        Ok(match s {
            "flash" => FLASH_CHAIN.clone(),
            s => {
                let chainspec = OpChainSpec::from(parse_genesis(s)?);
                Arc::new(chainspec)
            }
        })
    }
}
