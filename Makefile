.PHONY: try-runtime-turing-staging
try-runtime-turing-staging:
	RUST_LOG=runtime=trace,try-runtime::cli=trace,executor=trace cargo run --release --features=turing-node,try-runtime try-runtime --chain turing-dev on-runtime-upgrade live --uri=wss://rpc.turing-staging.oak.tech:443
