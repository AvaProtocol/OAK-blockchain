#!/bin/bash

staging_uri=wss://rpc.turing-staging.oak.tech:443
prod_uri=wss://rpc.turing.oak.tech:443

PS3="Select environment: "
select env in staging production
do
    case $env in
        staging ) uri=$staging_uri;;
        production ) uri=$prod_uri;;
    esac

    break
done

RUST_LOG=runtime=trace,try-runtime::cli=trace,executor=trace \
cargo run \
    --release \
    --features=turing-node,try-runtime \
    try-runtime \
    --chain turing-dev \
    on-runtime-upgrade \
    live \
    --uri=$uri