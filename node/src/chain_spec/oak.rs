use hex_literal::hex;

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};

use super::TELEMETRY_URL;
use crate::chain_spec::{
	get_account_id_from_seed, get_collator_keys_from_seed, inflation_config,
	test::{validate_allocation, validate_vesting},
	Extensions,
};
use oak_runtime::{
	CouncilConfig, PolkadotXcmConfig, SudoConfig, TechnicalMembershipConfig, ValveConfig,
	VestingConfig, DOLLAR, EXISTENTIAL_DEPOSIT, TOKEN_DECIMALS,
};
use primitives::{AccountId, AuraId, Balance};

const TOKEN_SYMBOL: &str = "OAK";
const SS_58_FORMAT: u32 = 51;
static RELAY_CHAIN: &str = "rococo-local";
static STAGING_RELAY_CHAIN: &str = "rococo";
static LIVE_RELAY_CHAIN: &str = "polkadot";
const REGISTERED_PARA_ID: u32 = 2114;
const REGISTERED_STAGING_PARA_ID: u32 = 4103;
const REGISTERED_LIVE_PARA_ID: u32 = 2090;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<oak_runtime::GenesisConfig, Extensions>;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> oak_runtime::SessionKeys {
	oak_runtime::SessionKeys { aura: keys }
}

pub fn oak_development_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"OAK Development",
		// ID
		"oak-dev",
		ChainType::Development,
		move || {
			let accounts = vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Dave"),
				get_account_id_from_seed::<sr25519::Public>("Eve"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			];
			const ALLOC_TOKENS_TOTAL: u128 = DOLLAR * 58_000_000;
			let initial_balance: u128 = ALLOC_TOKENS_TOTAL / accounts.len() as u128;
			let endowed_accounts: Vec<(AccountId, Balance)> =
				accounts.iter().cloned().map(|k| (k, initial_balance)).collect();

			testnet_genesis(
				// initial collators.
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						get_collator_keys_from_seed("Alice"),
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_collator_keys_from_seed("Bob"),
					),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				endowed_accounts,
				REGISTERED_PARA_ID.into(),
				vec![],
				vec![],
				vec![get_account_id_from_seed::<sr25519::Public>("Alice")],
				vec![get_account_id_from_seed::<sr25519::Public>("Alice")],
			)
		},
		Vec::new(),
		None,
		None,
		None,
		None,
		Extensions {
			relay_chain: RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: REGISTERED_PARA_ID,
		},
	)
}

pub fn oak_staging() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"Oak Staging",
		// ID
		"oak-staging",
		ChainType::Live,
		move || {
			let allocation_json =
				&include_bytes!("../../../distribution/oak_staging_alloc.json")[..];
			let initial_allocation: Vec<(AccountId, Balance)> =
				serde_json::from_slice(allocation_json).unwrap();
			const ALLOC_TOKENS_TOTAL: u128 = DOLLAR * 1_000_000_000;
			validate_allocation(
				initial_allocation.clone(),
				ALLOC_TOKENS_TOTAL,
				EXISTENTIAL_DEPOSIT,
			);

			let vesting_json = &include_bytes!("../../../distribution/oak_vesting.json")[..];
			let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
				serde_json::from_slice(vesting_json).unwrap();

			let vested_tokens = 9_419_999_999_999_999_919;
			let vest_starting_time: u64 = 1651431600;
			let vest_ending_time: u64 = 1743534000;
			validate_vesting(
				initial_vesting.clone(),
				vested_tokens,
				EXISTENTIAL_DEPOSIT,
				vest_starting_time,
				vest_ending_time,
			);

			testnet_genesis(
				// initial collators.
				vec![
					(
						// SS58 prefix substrate: 5EvKT4iWQ5gDnRhkS3d254RVCfaQJSyXHj5mA4kQTsCjWchW
						hex!["7e4f4efde71551c83714fb3062724067e7bb6ebc3bf942813321f1583e187572"]
							.into(),
						hex!["7e4f4efde71551c83714fb3062724067e7bb6ebc3bf942813321f1583e187572"]
							.unchecked_into(),
					),
					(
						// SS58 prefix substrate: 5F4Dx9TD16awU7FeGD3Me5VDrEsL5MDPEcbNBvUgLQFawxyW
						hex!["8456c30af083c2b2d767d6950e8ee654d4eff5e69cdd9f75db5bcbf2d7b0d801"]
							.into(),
						hex!["8456c30af083c2b2d767d6950e8ee654d4eff5e69cdd9f75db5bcbf2d7b0d801"]
							.unchecked_into(),
					),
				],
				// 5C571x5GLRQwfA3aRtVcxZzD7JnzNb3JbtZEvJWfQozWE54K
				hex!["004df6aeb14c73ef5cd2c57d9028afc402c4f101a8917bbb6cd19407c8bf8307"].into(),
				initial_allocation,
				REGISTERED_STAGING_PARA_ID.into(),
				vec![],
				initial_vesting,
				vec![
					// 5C571x5GLRQwfA3aRtVcxZzD7JnzNb3JbtZEvJWfQozWE54K
					hex!["004df6aeb14c73ef5cd2c57d9028afc402c4f101a8917bbb6cd19407c8bf8307"].into(),
				],
				vec![
					// 5C571x5GLRQwfA3aRtVcxZzD7JnzNb3JbtZEvJWfQozWE54K
					hex!["004df6aeb14c73ef5cd2c57d9028afc402c4f101a8917bbb6cd19407c8bf8307"].into(),
				],
			)
		},
		// Bootnodes
		Vec::new(),
		// Telemetry
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		// Protocol ID
		Some("oak"),
		None,
		// Properties
		Some(properties),
		// Extensions
		Extensions {
			relay_chain: STAGING_RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: REGISTERED_STAGING_PARA_ID,
		},
	)
}

pub fn oak_live() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"OAK",
		// ID
		"oak",
		ChainType::Live,
		move || {
			let allocation_json =
				&include_bytes!("../../../distribution/oak_alloc.json")[..];
			let initial_allocation: Vec<(AccountId, Balance)> =
				serde_json::from_slice(allocation_json).unwrap();
			const ALLOC_TOKENS_TOTAL: u128 = DOLLAR * 1_000_000_000;
			validate_allocation(
				initial_allocation.clone(),
				ALLOC_TOKENS_TOTAL,
				EXISTENTIAL_DEPOSIT,
			);

			let vesting_json = &include_bytes!("../../../distribution/oak_vesting.json")[..];
			let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
				serde_json::from_slice(vesting_json).unwrap();

			let vested_tokens = 9_419_999_999_999_999_919;
			let vest_starting_time: u64 = 1669874400;
			let vest_ending_time: u64 = 1764568800;
			validate_vesting(
				initial_vesting.clone(),
				vested_tokens,
				EXISTENTIAL_DEPOSIT,
				vest_starting_time,
				vest_ending_time,
			);

			testnet_genesis(
				// initial collators.
				vec![
					(
						// SS58 prefix substrate: 5EvKT4iWQ5gDnRhkS3d254RVCfaQJSyXHj5mA4kQTsCjWchW
						hex!["7e4f4efde71551c83714fb3062724067e7bb6ebc3bf942813321f1583e187572"]
							.into(),
						hex!["7e4f4efde71551c83714fb3062724067e7bb6ebc3bf942813321f1583e187572"]
							.unchecked_into(),
					),
					(
						// SS58 prefix substrate: 5F4Dx9TD16awU7FeGD3Me5VDrEsL5MDPEcbNBvUgLQFawxyW
						hex!["8456c30af083c2b2d767d6950e8ee654d4eff5e69cdd9f75db5bcbf2d7b0d801"]
							.into(),
						hex!["8456c30af083c2b2d767d6950e8ee654d4eff5e69cdd9f75db5bcbf2d7b0d801"]
							.unchecked_into(),
					),
				],
				// 66MGxr9zcyJ6ka6FBQmT1VSvMqARKfwBT7589Fikii1Ci5sg
				hex!["08df8338e854d8d589dedd4305c11e589cbef994e5dd00c7bb8fb7d277705b06"].into(),
				initial_allocation,
				REGISTERED_LIVE_PARA_ID.into(),
				vec![],
				initial_vesting,
				vec![
					// 69pKU2QpgtMBT9NsaN1diyJQ8qcvrJy8KJk5aWeAXfMGjb5F
					hex!["a23443cef4fe4e7ee3f61c8248505312fa81121f2a8bd64099390b40d7a05206"].into(),
					// 69Eyxo3gFpZpcRrtx5NAFBDMcoiVQrdg7so1M9w4FWiTBLej
					hex!["88c782a383f0cfa5fbc36c9734dfc7cb6319137b48cc69cc662c84b6e687a106"].into(),
					// 67D6ecyNhnAzZqgRbxr3MdGnxB9Bw8VadMhjpLAYB3wf5Pq6
					hex!["2edf0fd8948ea642f135b314b1358c77ec6d0a4af83220b6ea18136e5ce36277"].into(),
				],
				vec![
					// 69pKU2QpgtMBT9NsaN1diyJQ8qcvrJy8KJk5aWeAXfMGjb5F
					hex!["a23443cef4fe4e7ee3f61c8248505312fa81121f2a8bd64099390b40d7a05206"].into(),
					// 67ppUQ3mGfaWDEr5VQRz4QgvNzXiGLmGjwwEcnvbJofSQyMN
					hex!["4a1d713c2f41e10db31b83a45a3a98b64088330190eac7fcef4623694fbac55e"].into(),
					// 6871Utgdq5ycWnTzm7VEwQfg4zM81JqYv8EDzKPu3fdzsXK8
					hex!["56766d3f856f2ca399ae575689a7340c2532351948f355c21bc06b170569da35"].into(),
				],
			)
		},
		// Bootnodes
		Vec::new(),
		// Telemetry
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		// Protocol ID
		Some("oak"),
		None,
		// Properties
		Some(properties),
		// Extensions
		Extensions {
			relay_chain: LIVE_RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: REGISTERED_LIVE_PARA_ID,
		},
	)
}

fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	para_id: ParaId,
	pallet_gates_closed: Vec<Vec<u8>>,
	vesting_schedule: Vec<(u64, Vec<(AccountId, Balance)>)>,
	general_councils: Vec<AccountId>,
	technical_memberships: Vec<AccountId>,
) -> oak_runtime::GenesisConfig {
	let candidate_stake =
		std::cmp::max(oak_runtime::MinCollatorStk::get(), oak_runtime::MinCandidateStk::get());
	oak_runtime::GenesisConfig {
		system: oak_runtime::SystemConfig {
			code: oak_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		balances: oak_runtime::BalancesConfig { balances: endowed_accounts },
		parachain_info: oak_runtime::ParachainInfoConfig { parachain_id: para_id },
		session: oak_runtime::SessionConfig {
			keys: invulnerables
				.iter()
				.cloned()
				.map(|(acc, aura)| {
					(
						acc.clone(),                 // account id
						acc,                         // validator id
						template_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		parachain_staking: oak_runtime::ParachainStakingConfig {
			candidates: invulnerables
				.iter()
				.cloned()
				.map(|(acc, _)| (acc, candidate_stake))
				.collect(),
			delegations: vec![],
			inflation_config: inflation_config(oak_runtime::DefaultBlocksPerRound::get()),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		council: CouncilConfig { members: general_councils, phantom: Default::default() },
		democracy: Default::default(),
		tokens: Default::default(),
		technical_committee: Default::default(),
		technical_membership: TechnicalMembershipConfig {
			members: technical_memberships.try_into().unwrap(),
			phantom: Default::default(),
		},
		parachain_system: Default::default(),
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
		sudo: SudoConfig { key: Some(root_key) },
		treasury: Default::default(),
		valve: ValveConfig { start_with_valve_closed: false, closed_gates: pallet_gates_closed },
		vesting: VestingConfig { vesting_schedule },
	}
}
