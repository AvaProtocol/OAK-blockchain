---
title: Run a Validator
---

## Run a validator node

Then run the following command to run a validator node

### Run a validator node
```bash
docker run -d -p 9993:9933 --name <container_name> oaknetwork/oak_testnet:latest --name <node_name> --validator --rpc-cors all --rpc-methods=unsafe --rpc-external
```

### Get the rotateKeys of the node
Use cURL command from command line to send a request to the node just started up.

```
curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}' http://localhost:9933
```

The response should look like this.
```
{"jsonrpc":"2.0","result":"0xd63605b333d1a706bddfa0c79e930fbcb3db786d59181289b19d698e4c8f7b5ca2b5e55a59fa94bf0a147a4aa697c9ff90ed075923bf17d7c80ca1daccb2c31e10cb1cc74a53f5ce5fb048ac12e2d0c8d74eed658ba85dc6788e67077b5e871ad8dbf6d47a1233dd40d3b17ce6c130c274a3f83779105416c8e3e1c929f62618","id":1}
```

The output will have a hex-encoded "result" field. The result is the concatenation of the four public keys. Save this result for a later step.


## Become a validator

### Open this link and click 'Validator'
https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.testnet.oak.tech#/staking/actions

### Setup validator

#### Setup validator 1/2
![setup_validator_1](assets/images/validator/setup_validator_1.png)

There are some fields.

##### Stash account, controller account
Think of the stash as your cold wallet and the controller as your hot wallet. Funding operations are controlled by the stash, any other non-funding actions by the controller itself.


##### Value bonded
The amount placed at-stake should not be your full available available amount to allow for transaction fees.
Once bonded, it will need to be unlocked/withdrawn and will be locked for at least the bonding duration.

##### Payment destination
Rewards (once paid) can be deposited to either the stash or controller, with different effects.

After filling the form, click 'next' to the form of 'setup validator 2/2'

#### Setup validator 2/2

![setup_validator_2](assets/images/validator/setup_validator_2.png)

There are some fields.

##### Keys from rotateKeys
Fill the string generated form [Run a validator node](https://github.com/OAK-Foundation/OAK-blockchain/blob/add_validator_setup_doc/docs/validator-setup.md#run-a-validator-node)

##### Reward commission percentage
The commission is deducted from all rewards before the remainder is split with nominators.
For example, 20.

##### Allows new nominations
The validator can block any new nominations. By default it is set to allow all nominations.

After commit, you can see your accont in the stashes list.
![stashes](assets/images/validator/stashes.png)

And you can the account in the waitting list.
![waitting](assets/images/validator/waitting.png)

After an era, your account is in the validators list.

In OAK Testnet, an era is 1 hour currently.

The number of validators is limited, the top 6 will become validators currently.

![era](assets/images/validator/era.png)

![validators](assets/images/validator/validators.png)
