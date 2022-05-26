export CARGO_INCREMENTAL=0
export SKIP_BUILD_WASM=true
export BUILD_DUMMY_WASM_BINARY=true
export LLVM_PROFILE_FILE="llvmcoveragedata-%p-%m.profraw"
export WASM_TARGET_DIRECTORY=/tmp/wasm
cargo build --features all-nodes
export RUSTFLAGS="-Zinstrument-coverage"
rm -rf target/debug
cargo test --all --features all-nodes
# grcov generate lcov.info
# https://github.com/mozilla/grcov/blob/master/README.md
# Ignore target/*, **test.rs
grcov . --binary-path ./target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore "/*" --ignore "target/*" --ignore "**tests.rs" -o lcov.info
# Fix coverage
# https://crates.io/crates/rust-covfix
rust-covfix -o lcov_correct.info lcov.info
bash <(curl -s https://codecov.io/bash) -f lcov_correct.info
