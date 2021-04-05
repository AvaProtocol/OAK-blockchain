<a href="https://github.com/w3f/Open-Grants-Program/pull/268"><img src="https://user-images.githubusercontent.com/2616844/113636716-e3857f80-9627-11eb-842a-dcb1e1a96689.png" alt="oak-web3-open-grant" /></a>
OAK(Onchain Autonomous Framework) is a unique blockchain built on Substrate framework with event-driven smart contract VM, autonomous transactions, and on-chain scheduler.

Documentation
----------

* [Website](https://oak.tech/)
* [Documentation]()

<!--- 

Whitepaper
----------

* [Whitepaper]()
-->

Community
--------- 

* General discussion: [Telegram (Coming Soon)]()
* Technical discussion: [Discord (Coming Soon)]()
* Subscribe on [OAK Twitter](https://twitter.com/OAKSubstrate)
* Subscribe on [Founder's Twitter](https://twitter.com/chrisli2046)

Table of Contents
-----------------

* [Introduction]()
* [Install OAK]()
* [OAK Validator Program]()

Introduction
============

**OAK, or Onchain Automation Framework, is equipped with a novel smart contract virtual machine that supports an event-driven execution model, enabling developers to build fully autonomous decentralized application. **By extending the current set of atomic operations, namely, opcodes of EVM, OAK introduces an innovative way for contracts to interact with each other. Contracts can emit signal events, on which other contracts can listen. Once an event is triggered, corresponding handler functions are automatically executed as a new type of transaction, signal transaction. Applications implemented with the new approach will eliminate the dependency of unreliable mechanisms like off-chain relay servers, and in return, to significantly simplify the execution flow of the application and can avoid security risks such as relay manipulation attacks.

Based on the above, OAK has some features.
- **[OAK Virtual Machine]()**
- **[Autonomous Transactions]()**
- **[Onchain Relayer]()**
- **[Validator Staking]()**

Once Polkadot is launched, we will connect our root chain to Polkadot, and we aim to be one of the parachains.

Install OAK Blockchain 
=============

* OAK releases [releases]().
* Node [custom types](). 

> Latest version you can try to build from source.

Building from source
--------------------

Ensure you have Rust and the support software:

    curl https://sh.rustup.rs -sSf | sh
    # on Windows download and run rustup-init.exe
    # from https://rustup.rs instead

    rustup update nightly
    rustup target add wasm32-unknown-unknown --toolchain nightly

You will also need to install the following dependencies:

* Linux: `sudo apt install cmake git clang libclang-dev build-essential`
* Mac: `brew install cmake git llvm`
* Windows: Download and install the Pre Build Windows binaries of LLVM from http://releases.llvm.org/download.html

Install additional build tools:

    cargo +nightly install --git https://github.com/alexcrichton/wasm-gc

Install the OAK node from git source:

    

Run node on [Dusty Network](https://telemetry.polkadot.io/#list/Dusty):

    oak

Or run on your local development network:

    oak --dev

Building with Nix
-----------------

Install Nix package manager:

    curl https://nixos.org/nix/install | sh

Run on your Nix shell:

    git clone


Future Works
------------
Here are the key milestones.

1. Start the crowdloan with Kusama network
1. Become a Kusama Parachain (TBA)
1. Become a Polkadot Parachain. (TBA)

If you have any questions, please ask us on [Discord]()

Contacts
--------

**Maintainers**

* [Chris Li](https://github.com/chrisli30)
* [Xingyou Chen](https://github.com/imstar15)
* [Zhongwei Shi](https://github.com/amazingbeerbelly)

* * *

OAK blockchain is licensed under the GPLv3.0 by OAK Foundation.
