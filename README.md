<div><a href="https://github.com/w3f/Open-Grants-Program/pull/268"><img src="https://user-images.githubusercontent.com/1693789/156277834-ed433b60-9e82-4267-8b4f-e30438dbec54.png" alt="oak-web3-open-grant" style="width:40%" /></a></div>
OAK(Onchain Autonomous Framework) is a unique blockchain built on Substrate framework with an event-driven execution model, autonomous transactions, and on-chain scheduling.

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

Build from source
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
    cargo build --release --features turing-node --features dev-queue
```

Quickstart: run Local Network with Zombienet
-----------
We have configured a network of 2 relay chain nodes, 1 Turing node and 1 Substate Parachain Template node in [zombienets/turing](https://github.com/OAK-Foundation/OAK-blockchain/tree/master/zombienets/turing), so the easiest way to spin up a local network is through below steps.

1. Clone and build source of [Zombienet]
   1. Find the latest stable tag from [Releases](https://github.com/paritytech/zombienet/releases), for example, `v1.3.17`
   2. `git clone https://github.com/paritytech/zombienet.git`
   3. `cd zombienet && git fetch --all --tags`
   4. `git checkout tags/v1.3.17`
   5. `cd javascript`
   6. Make sure your node version is compatible with that in [javascript/package.json](https://github.com/paritytech/zombienet/blob/main/javascript/package.json), for example `"node": ">=16"`.
   7. `npm install`
   8. `npm run build`
2. After a successful build, you should be able to test run `npm run zombie`.
3. Create an alias to the zombie program(on MacOS). Since the actual command of `npm run zombie` is `node ./packages/cli/dist/cli.js`, we can add an alias to it by editing the `~/.bash_profile` file. Simply, run `vim ~/.bash_profile` add one line `alias zombie="node <your_absolute_path>/zombienet/javascript/packages/cli/dist/cli.js"` to it.
4. Run `source ~/.bash_profile`. This will reload your command-line.
5. Cd into OAK-blockchain folder, `cd ../../OAK-blockchain`.
6. Spawn Turing config, `zombie spawn zombienets/turing/xcmp.toml`.

The zombie spawn will run 2 relay chain nodes, 1 Turing node and 1 Substate Parachain Template node, and set up an HRMP channel between the parachains.

Run Local Network from source
-----------
Launch a local setup including a Relay Chain and a Parachain.
Note: local PARA_ID is defaulted to 2000

### Launch the Relay Chain

First, find out a compatible version of the relay chain’s code from `polkadot-parachain` reference in [runtime/turing/Cargo.toml](https://github.com/OAK-Foundation/OAK-blockchain/blob/master/runtime/turing/Cargo.toml), for example, `branch "release-v0.9.29" on "https://github.com/paritytech/polkadot"`. That is the code of the relay chain to run, and then build the source with the below commands.

```bash
# Compile Polkadot with the real overseer feature
git clone --branch release-v0.9.29 https://github.com/paritytech/polkadot
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
cargo build --release --features turing-node --features dev-queue
```
> Alternatively, to make local testing easy use a `dev-queue` flag, which will allow for putting a task directly on the task queue as opposed to waiting until the next hour to schedule a task.  This works when the `execution_times` passed to schedule a task equals `[0]`.


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
