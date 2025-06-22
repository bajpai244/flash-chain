# Flash Chain

Flash is a project, exploring integrating [op-batcher](https://github.com/ethereum-optimism/optimism/tree/develop/op-batcher), directly into a custom op-stack chains, by leveraging [reth-exex](https://www.paradigm.xyz/2024/05/reth-exex)

## How to run?

Start the flash chain execution client:
```bash
cargo run -p flash_chain node \
    --chain flash \
    --http \
    --ws \
    --authrpc.port 8551 \
    --authrpc.jwtsecret jwt.txt \
    --datadir datadir 
```

NOTE: you will need to generate a jwt, and store it as jwt.txt in the project root, so that you can connect with a [op-node](https://github.com/ethereum-optimism/optimism/tree/develop/op-node), and start producing blocks.

Start an op-node:
```bash
op-node --l2=http://localhost:8551 --l2.jwt-secret=./jwt.txt --sequencer.enabled --sequencer.l1-confs=5 --verifier.l1-confs=4 --rollup.config=./rollup.json --rpc.addr=0.0.0.0 --p2p.disable --rpc.enable-admin --p2p.sequencer.key=$GS_SEQUENCER_PRIVATE_KEY --l1=$L1_RPC_URL  --l1.beacon=https://ethereum-sepolia-beacon-api.publicnode.com
```

and, you will start seeing blocks being produced on the chain, they being batched and submitted.

The batcher right is capable of doing the following:

- A ChannelBuilder to build new channels from new blocks being produced
    - The implementation is done via a reth-exex
- Channels are written to SQL lite tables
- A simple routine mocks the behaviour of consuming a channel, and uploading it to a DA layer

What features are not included in the toy batcher:

- splitting channels into frames, and doing compression
- integration with real DA layers
- the channel upload routine needs to be a seperate service from the reth-exex so that it can run concurrently, without effecting the reth-exex
- doesn’t manages re-orgs and pruning


