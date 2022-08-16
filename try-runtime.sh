#!/usr/bin/env bash

# local:
# RUST_LOG=runtime=trace,try-runtime::cli=trace,executor=trace \
#   cargo run \
#     --release \
#     --features=all-nodes,try-runtime \
#     try-runtime \
#     --execution=Native \
#     --chain turing-dev \
#     on-runtime-upgrade live \
#     --uri=ws://localhost:9988

# staging:
RUST_LOG=runtime=trace,try-runtime::cli=trace,executor=trace \
  cargo run \
    --release \
    --features=all-nodes,try-runtime \
    try-runtime \
    --execution=Native \
    --chain turing-dev \
    on-runtime-upgrade live \
    --uri=wss://rpc.turing-staging.oak.tech:443
    #--uri="ws://54.89.28.91:9944"
    #--uri="ws://54.89.28.91:9944"
    #--uri=wss://rpc.turing-staging.oak.tech
    #--uri=wss://rpc.turing-staging.oak.tech:9944
    #--uri="ws://54.89.28.91:9933"

# RUST_LOG=runtime=trace,try-runtime::cli=trace,executor=trace \
#   cargo run \
#   --release \
#   --features=all-nodes,try-runtime \
#   try-runtime \
#   --chain=turing-dev \
#   on-runtime-upgrade \
#   live \
#   --uri ws://localhost:9988

# cargo run \
#   --release \
#   --execution Native \
#   --features all-nodes \
#   --features try-runtime \
#   -- \
#     try-runtime \
#     --chain=turing-dev \
#     on-runtime-upgrade \
#     live \
#     --uri wss://rpc.turing-staging.oak.tech
