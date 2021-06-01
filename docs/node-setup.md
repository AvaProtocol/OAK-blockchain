---
title: Node Setup
---

## Run a local single development node

### Setup Rust environment

```bash
# setup Rust nightly toolchain
rustup default nightly-2021-03-01
# setup wasm toolchain
rustup target add wasm32-unknown-unknown --toolchain nightly-2021-03-01
```

### Clone the OAK blockchain (branch oak-testnet).

```bash
git clone -b oak-testnet https://github.com/OAK-Foundation/OAK-blockchain.git
```

### Build

```bash
cargo build --release
```

### Run

```bash
./target/release/oak-testnet --dev --tmp
```

#### Options:
```
-h, --help
  Prints help information

--dev
  Specify the development chain

--tmp
    Run a temporary node.

    A temporary directory will be created to store the configuration and will be deleted at the end of the
    process.

-d, --base-path <PATH>
    Specify custom base path

--rpc-external
    Listen to all RPC interfaces.

--ws-external
    Listen to all Websocket interfaces.

    Default is local. Note: not all RPC methods are safe to be exposed publicly. Use an RPC proxy server to
    filter out dangerous methods. More details: <https://github.com/paritytech/substrate/wiki/Public-RPC>. Use
    `--unsafe-ws-external` to suppress the warning if you understand the risks.

--unsafe-rpc-external
    Listen to all RPC interfaces.

    Same as `--rpc-external`.

--unsafe-ws-external
    Listen to all Websocket interfaces.
```

## Join OAK Testnet

Then run the following command to start a full node and join OAK Testnet

```bash
docker run -d --name <container_name> imstar15/oak-testnet:latest --name <node_name>
```
