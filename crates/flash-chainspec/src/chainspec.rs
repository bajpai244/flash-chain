//! Odyssey chainspec parsing logic.
use alloy_primitives::{U256, b256};
use reth_op::{
    chainspec::{
        BaseFeeParams, BaseFeeParamsKind, Chain, ChainSpec, EthereumHardfork, Hardfork,
        OpChainSpec, make_op_genesis_header,
    },
    primitives::SealedHeader,
};
use reth_optimism_forks::{OP_SEPOLIA_HARDFORKS, OpHardfork};
use std::sync::{Arc, LazyLock};

pub const FLASH_CHAIN_ID: u64 = 421;

/// The FLASH CHAIN spec
pub static FLASH_CHAIN: LazyLock<Arc<OpChainSpec>> = LazyLock::new(|| {
    let genesis = serde_json::from_str(include_str!("../../../config/genesis.json"))
        .expect("Can't deserialize Flash genesis json");

    let hardforks = OP_SEPOLIA_HARDFORKS.clone();
    OpChainSpec {
        inner: ChainSpec {
            chain: Chain::from_id(FLASH_CHAIN_ID),
            genesis_header: SealedHeader::new(
                make_op_genesis_header(&genesis, &hardforks),
                b256!("0x2a68ab477176e988e21ea9ea6eb852a9fb2d39341debc8fb1d69a6f0a20926b7"),
            ),
            genesis,
            paris_block_and_final_difficulty: Some((0, U256::from(0))),
            hardforks,
            base_fee_params: BaseFeeParamsKind::Variable(
                vec![
                    (
                        EthereumHardfork::London.boxed(),
                        BaseFeeParams::optimism_sepolia(),
                    ),
                    (
                        OpHardfork::Canyon.boxed(),
                        BaseFeeParams::optimism_sepolia_canyon(),
                    ),
                ]
                .into(),
            ),
            prune_delete_limit: 10000,
            ..Default::default()
        },
    }
    .into()
});
