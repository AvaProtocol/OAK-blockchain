.PHONY: try-runtime-turing
try-runtime-turing:
	RUST_LOG=runtime=trace,try-runtime::cli=trace,executor=trace cargo run --release --features=turing-node,try-runtime try-runtime --execution=Native --chain turing-dev on-runtime-upgrade live --uri=wss://rpc.turing-staging.oak.tech:443
