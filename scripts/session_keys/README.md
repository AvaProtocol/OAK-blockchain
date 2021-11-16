# The session keys of the initial validator node.

This article describes how to set session keys for the initial validator node.

There must be at least 3 initial validator nodes in a network. You need to generate and setup session keys for each initial validator node.

The key types are audi, babe, gran, imon. They are used in these modules: authority_discovery, babe, grandpa, im_online.

| module name | key type |
| ---- | ---- |
| authority_discovery | audi |
| babe | babe |
| grandpa | gran |
| im_online | imon |

## 1. Use subkey to generate a mnemonic phrase for the initial validator node.

```
Run subkey generate
```

The output is as follows:
```
Secret phrase `<mnemonic phrase>` is account:
```

Save the `mnemonic phrase` for use in the following steps.

## 2. Generate session keys for chain spec.

### 2.1 Execute commands to generate session keys.

```
SECRET="<mnemonic phrase>" sh prepare-test-net.sh <initial validator count>
```

The output is as follows:
```
(
// <stash public key (SS58)>
hex!["<stash public key> (hex)"].into(),
// <controller public key (SS58)>
hex!["<controller public key> (hex)"].into(),
// <grandpa public key (SS58)>
hex!["<grandpa public key> (hex)"].unchecked_into(),
// <babe public key (SS58)>
hex!["<babe public key> (hex)"].unchecked_into(),
// <im_online public key (SS58)>
hex!["<im_online public key> (hex)"].unchecked_into(),
// <authority_discovery public key (SS58)>
hex!["<authority_discovery public key (hex)>"].unchecked_into(),
),
(...),
{...)
```

### 2.2 Modify chain specification

Modify `initial_authorities ` in oak_testet.rs

```
fn oak_testnet_staging_genesis() -> GenesisConfig {
	let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![
		(
		// <stash public key (SS58)>
		hex!["<stash public key> (hex)"].into(),
		// <controller public key (SS58)>
		hex!["<controller public key> (hex)"].into(),
		// <grandpa public key (SS58)>
		hex!["<grandpa public key> (hex)"].unchecked_into(),
		// <babe public key (SS58)>
		hex!["<babe public key> (hex)"].unchecked_into(),
		// <im_online public key (SS58)>
		hex!["<im_online public key> (hex)"].unchecked_into(),
		// <authority_discovery public key (SS58)>
		hex!["<authority_discovery public key (hex)>"].unchecked_into(),
		),
		(...),
		{...),
	];
}
```

## 3. Generate and modify chain specification

### 3.1 Generate plain chain specification file
```
# Compile
cargo build --release
# Generate plain chain specification file
./target/release/oak build-spec --chain oak-testnet-staging > oak-testnet-plain.json
```

### 3.2 Modify plain chain specification
#### 3.2.1 Generate node-key

The first validator node to start uses the `node-key` file to uniquely determine its `peer ID`.

```
subkey generate-node-key --file node-key
```
The `peer ID` is displayed on screen and the actual key is saved in the `node-key` file.

#### 3.2.2 Modify oak-testnet-plain.json
```
{
  "name": "OAK Testnet",
  "id": "oak_testnet",
  ...
  "bootNodes": [
    "/ip4/127.0.0.1/tcp/30333/p2p/<peer ID>"
  ],
}
```

### 3.3 Generate raw chain specification file from plain chain specification file
```
# Generate raw chain specification file
./target/release/oak build-spec --chain oak-testnet-plain.json > node/cli/src/res/oak-testnet.json
# Recompile the code to make the raw chain specification take effect
cargo build --release
```

## 4. Set the session keys of the validator node through the rpc request

### 4.1 Start validator nodes
```
# Validator 1, start with node-key file
./target/release/oak --chain oak-testnet -d <validator1-storage-folder> --validator --name <validator1-name> --port <port> --ws-port <ws-port> --rpc-port <rpc-port> --node-key-file node-key

# Validator 2
./target/release/oak --chain oak-testnet -d <validator2-storage-folder> --validator --name <validator2-name> --port <port> --ws-port <ws-port> --rpc-port <rpc-port>

# Validator 3
./target/release/oak --chain oak-testnet -d <validator3-storage-folder> --validator --name <validator3-name> --port <port> --ws-port <ws-port> --rpc-port <rpc-port>
```

### 4.2 Set the validator's session key and mnemonic phrase
Copy the corresponding public key information from the session keys generated in section 2.1 above, and send an RPC request to the node.

```
curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "author_insertKey", "params":["<key type>", "<mnemonic phrase>//<node index>//<module name>", "0x<public key> (hex)"]}' <rpc_url>
```

Take the authority_discovery session key of the first node as an example:

Run command：

```
curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "author_insertKey", "params":["audi", "<mnemonic phrase>//1//authority_discovery", "0x<authority_discovery public key (hex)>"]}' <rpc_url>
```

If the command is executed correctly, you will see the following output.
```
{"jsonrpc":"2.0","result":null,"id":1}
```

If you have 3 validator nodes, and each node needs to set 4 session keys (audi, babe, grandpa, imon), then you need to execute 12 commands. In order to facilitate execution, you can modify the `run.sh` script to set up in batches.
