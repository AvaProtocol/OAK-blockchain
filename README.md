<div><a href="https://github.com/w3f/Open-Grants-Program/pull/268"><img src="https://user-images.githubusercontent.com/1693789/156277834-ed433b60-9e82-4267-8b4f-e30438dbec54.png" alt="oak-web3-open-grant" style="width:40%" /></a></div>
OAK(Onchain Autonomous Framework) is a unique blockchain built on Substrate framework with event-driven execution model, autonomous transactions, and on-chain scheduling.

Documentation
----------

* [Website](https://oak.tech/)
* [Documentation](https://docs.oak.tech/)

Community
--------- 

* General discussion: [Telegram](https://t.me/OAK_Announcements)
* Technical discussion: [Discord](https://discord.gg/7W9UDvsbwh)
* Subscribe on [OAK Twitter](https://twitter.com/oak_network)
* Subscribe on [Founder's Twitter](https://twitter.com/chrisli2046)

Introduction
============

**OAK, or Onchain Automation Framework, is equipped with a novel smart contract virtual machine that supports an event-driven execution model, enabling developers to build fully autonomous decentralized application.** By extending the current set of atomic operations, namely, opcodes of EVM, OAK introduces an innovative way for contracts to interact with each other. Contracts can emit signal events, on which other contracts can listen. Once an event is triggered, corresponding handler functions are automatically executed as a new type of transaction, signal transaction. Applications implemented with the new approach will eliminate the dependency of unreliable mechanisms like off-chain relay servers, and in return, to significantly simplify the execution flow of the application and can avoid security risks such as relay manipulation attacks.

Based on the above, OAK has some features.
- **OAK Virtual Machine**
- **Autonomous Transactions**
- **On-chain Scheduler**
- **Collator Staking**

Live Networks
============
- `turing-staging`: rococo parachain (May 2022)
- `turing`: kusama parachain (April 2022)
- `oak`: polkadot parachain (Q1 2023)

Install OAK Blockchain 
=============

* OAK releases [releases](https://github.com/OAK-Foundation/OAK-blockchain/releases).

Building from source
--------------------

Ensure you have Rust and the support software (see shell.nix for the latest functional toolchain):

    curl https://sh.rustup.rs -sSf | sh
    # on Windows download and run rustup-init.exe
    # from https://rustup.rs instead

    rustup update nightly
    rustup target add wasm32-unknown-unknown --toolchain nightly

    # Make sure the rustup default is compatible with your machine, for example, if you are building using Apple M1 ARM you need to run
    rustup install stable-aarch64-apple-darwin
    rustup default stable-aarch64-apple-darwin

You will also need to install the following dependencies:

* Linux: `sudo apt install cmake git clang libclang-dev build-essential`
* Mac: `brew install cmake git llvm`
* Windows: Download and install the Pre Build Windows binaries of LLVM from http://releases.llvm.org/download.html

Install additional build tools:

    cargo +nightly install --git https://github.com/alexcrichton/wasm-gc

Install the OAK node from git source:

    git clone git@github.com:OAK-Foundation/OAK-blockchain.git    

Build your executable:
```bash
    cargo build --release --features neumann-node
    # OR
    cargo build --release --features turing-node
```

Run your Local Network
-----------

Launch a local setup including a Relay Chain and a Parachain.
Note: local PARA_ID is defaulted to 2000

### Launch the Relay Chain

Please check out paritytech/polkadot’s [Build from Source](https://github.com/paritytech/polkadot/releases/) for full instructions, but first find the latest stable tag on [Polkadot Releases](https://github.com/paritytech/polkadot/releases/), for example, using `v0.9.31` as the `<latest_stable_tag>` below.

```bash
# Compile Polkadot with the real overseer feature
git clone https://github.com/paritytech/polkadot
git checkout tags/<latest_stable_tag>
cargo build --release
```

Once the build succeeds, open up two terminal windows to run two nodes separately.

1. Run node Alice in the first terminal window
    ```bash
    ./target/release/polkadot \
    --alice \
    --validator \
    --tmp \
    --chain ../OAK-blockchain/resources/rococo-local.json \
    --port 30333 \
    --ws-port 9944
    ```
1. Run node Bob in the second terminal window
    ```bash
    # Bob (In a separate terminal)
    ./target/release/polkadot \
    --bob \
    --validator \
    --tmp \
    --chain ../OAK-blockchain/resources/rococo-local.json \
    --port 30334 \
    --ws-port 9945
    ```
At this point, your local relay chain network should be running. Next, we will connect Turing node to the relay chain network as a parachain.
### Reserve the Parachain slot

1. Navigate to [Local relay parachain screen](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/parachains/parathreads)
2. Click `+ ParaId`
3. Reserve `paraid` with the following paramaters
    - `reserve from`: `Alice`
    - `parachain id`: 2000
    - `reserved deposit`: <whatever the default is>


### Compile and run our parachain
First, make sure your `rustup default` output is compatible with your machine, and compile the code into a build.

```bash
git clone https://github.com/OAK-Foundation/OAK-blockchain
cargo build --release --features neumann-node
```
> Alternatively, to make local testing easy use a `dev-queue` flag, which will allow for putting a task directly on the task queue as opposed to waiting until the next hour to schedule a task.  This works when the `execution_times` passed to schedule a task equals `[0]`. For example, `cargo build --release --features neumann-node --features dev-queue`


Second, prepare two files, genesis-state and genesis-wasm, for parachain registration.
```bash
# Generate a genesis state file
./target/release/oak-collator export-genesis-state > genesis-state

# Generate a genesis wasm file
./target/release/oak-collator export-genesis-wasm > genesis-wasm
```

Third, start up the build.
```bash
# Collator1
./target/release/oak-collator \
--alice \
--collator \
--force-authoring \
--tmp \
--port 40333 \
--ws-port 9946 \
-- \
--execution wasm \
--chain resources/rococo-local.json \
--port 30335 \
--ws-port 9977 
```

### Register the parachain

1. Navigate to [Local relay sudo extrinsic](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/sudo)
2. Register your local parachain on the local relay chain by calling `parasSudoWrapper.sudoScheduleParaInitialize` (see the screenshot below). 
3. Parameters:
    1. id: 2000 (this is paraId of the parachain)
    2. genesisHead: the above generated `genesis-state` file.
    3. validationCode, the `genesis-wasm` file.
    4. parachain: Yes.
![image](./media/readme-parachain-registration.png)
1. Once submitted, you should be able to see the id:2000 from the [Parathread](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/parachains/parathreads) tab, and after a short period on the [Parachains](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/parachains) tab.![image](./media/readme-parachain-post-registration.png)
    


### Test the parachain

1. Navigate to [Local parachain](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9946#/explorer)


Contacts
--------
Maintainers: [OAK Development Team](https://github.com/orgs/OAK-Foundation/people)

If you have any questions, please ask our devs on [Discord](https://discord.gg/7W9UDvsbwh)

* * *

OAK blockchain is licensed under the GPLv3.0 by the OAK Network team.
