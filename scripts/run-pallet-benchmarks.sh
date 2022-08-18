#!/bin/bash

read -p 'Pallet name: ' pallet

pallet_name=$(echo "$pallet" | tr '-' '_')

cargo run \
--release \
--features runtime-benchmarks,turing-node \
benchmark \
pallet \
--chain turing-dev \
--execution wasm \
--wasm-execution compiled \
--pallet pallet_$pallet_name \
--extrinsic '*' \
--repeat 20 \
--steps 50 \
--output ./pallets/$pallet/src/raw-weights.rs