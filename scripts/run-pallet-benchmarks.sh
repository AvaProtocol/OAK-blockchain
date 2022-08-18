#!/bin/bash

PS3="Pallet name: "
select pallet_name in $(ls pallets | tr '-' '_'); do break; done 

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
