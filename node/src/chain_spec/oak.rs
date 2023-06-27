use hex_literal::hex;

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::{Perbill, Percent};

use super::TELEMETRY_URL;
use crate::chain_spec::{
	get_account_id_from_seed, get_collator_keys_from_seed, inflation_config,
	test::{validate_allocation, validate_vesting},
	Extensions,
};
use common_runtime::constants::currency::{DOLLAR, EXISTENTIAL_DEPOSIT, TOKEN_DECIMALS};
use oak_runtime::{
	CouncilConfig, PolkadotXcmConfig, SudoConfig, TechnicalMembershipConfig, ValveConfig,
	VestingConfig, XcmpHandlerConfig,
};
use pallet_xcmp_handler::XcmFlow;
use primitives::{AccountId, AuraId, Balance};

const TOKEN_SYMBOL: &str = "OAK";
const SS_58_FORMAT: u32 = 51;
static RELAY_CHAIN: &str = "rococo-local";
static STAGING_RELAY_CHAIN: &str = "rococo";
static LIVE_RELAY_CHAIN: &str = "polkadot";
const REGISTERED_PARA_ID: u32 = 2090;
const REGISTERED_STAGING_PARA_ID: u32 = 4103;

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
				vec![],
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
			let allocation_json = &include_bytes!("../../../distribution/oak_alloc.json")[..];
			let initial_allocation: Vec<(AccountId, Balance)> =
				serde_json::from_slice(allocation_json).unwrap();
			const ALLOC_TOKENS_TOTAL: u128 = 80_230_000_000_000_000;
			validate_allocation(
				initial_allocation.clone(),
				ALLOC_TOKENS_TOTAL,
				EXISTENTIAL_DEPOSIT,
			);

			let vesting_json = &include_bytes!("../../../distribution/oak_vesting.json")[..];
			let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
				serde_json::from_slice(vesting_json).unwrap();

			let vested_tokens = 7_599_999_950_000_000_000;
			let vest_starting_time: u64 = 1672603200;
			let vest_ending_time: u64 = 1796155200;
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
						// SS58 prefix 51: 67He8TSn5ayD2p2ZS9WDQtuVuW7Z8RXXvBx5NyszGwcEGpzS
						hex!["325603af5d8d4e284646556835df92977d48c8100eaf92e110d4b75d4650a73c"]
							.into(),
						hex!["325603af5d8d4e284646556835df92977d48c8100eaf92e110d4b75d4650a73c"]
							.unchecked_into(),
					),
					(
						// SS58 prefix 51: 6Azd1ebort6eZUDfg9f4A75fmD1JCV6ysHaFnTEqEPEuGpbP
						hex!["d64c03d1ab36318a0d7da660670c83928261fc2a3464f92218ff5d42561e9a40"]
							.into(),
						hex!["d64c03d1ab36318a0d7da660670c83928261fc2a3464f92218ff5d42561e9a40"]
							.unchecked_into(),
					),
				],
				// 5C571x5GLRQwfA3aRtVcxZzD7JnzNb3JbtZEvJWfQozWE54K
				hex!["004df6aeb14c73ef5cd2c57d9028afc402c4f101a8917bbb6cd19407c8bf8307"].into(),
				initial_allocation,
				REGISTERED_STAGING_PARA_ID.into(),
				vec![
					b"AutomationTime".to_vec(),
					b"Balances".to_vec(),
					b"Bounties".to_vec(),
					b"Currencies".to_vec(),
					b"Democracy".to_vec(),
					b"ParachainStaking".to_vec(),
					b"PolkadotXcm".to_vec(),
					b"Treasury".to_vec(),
					b"XTokens".to_vec(),
				],
				initial_vesting,
				vec![
					// 5C571x5GLRQwfA3aRtVcxZzD7JnzNb3JbtZEvJWfQozWE54K
					hex!["004df6aeb14c73ef5cd2c57d9028afc402c4f101a8917bbb6cd19407c8bf8307"].into(),
				],
				vec![
					// 5C571x5GLRQwfA3aRtVcxZzD7JnzNb3JbtZEvJWfQozWE54K
					hex!["004df6aeb14c73ef5cd2c57d9028afc402c4f101a8917bbb6cd19407c8bf8307"].into(),
				],
				vec![],
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
		"OAK Network",
		// ID
		"oak",
		ChainType::Live,
		move || {
			let allocation_json = &include_bytes!("../../../distribution/oak_alloc.json")[..];
			let initial_allocation: Vec<(AccountId, Balance)> =
				serde_json::from_slice(allocation_json).unwrap();
			const ALLOC_TOKENS_TOTAL: u128 = 80_230_000_000_000_000;
			validate_allocation(
				initial_allocation.clone(),
				ALLOC_TOKENS_TOTAL,
				EXISTENTIAL_DEPOSIT,
			);

			let vesting_json = &include_bytes!("../../../distribution/oak_vesting.json")[..];
			let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
				serde_json::from_slice(vesting_json).unwrap();

			let vested_tokens = 7_599_999_950_000_000_000;
			let vest_starting_time: u64 = 1672603200;
			let vest_ending_time: u64 = 1796155200;
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
						// SS58 prefix 51: 67He8TSn5ayD2p2ZS9WDQtuVuW7Z8RXXvBx5NyszGwcEGpzS
						hex!["325603af5d8d4e284646556835df92977d48c8100eaf92e110d4b75d4650a73c"]
							.into(),
						hex!["325603af5d8d4e284646556835df92977d48c8100eaf92e110d4b75d4650a73c"]
							.unchecked_into(),
					),
					(
						// SS58 prefix 51: 6Azd1ebort6eZUDfg9f4A75fmD1JCV6ysHaFnTEqEPEuGpbP
						hex!["d64c03d1ab36318a0d7da660670c83928261fc2a3464f92218ff5d42561e9a40"]
							.into(),
						hex!["d64c03d1ab36318a0d7da660670c83928261fc2a3464f92218ff5d42561e9a40"]
							.unchecked_into(),
					),
					(
						// SS58 prefix 51: 6AQf3NV66CTNSsFuY1ehA1zJySYQybQwvC5Yq8ufqcZVQ3iN
						hex!["bc6480f3f236dcd444809ace67e5a64bd35d7757b46d051bf300264bfc1eea55"]
							.into(),
						hex!["bc6480f3f236dcd444809ace67e5a64bd35d7757b46d051bf300264bfc1eea55"]
							.unchecked_into(),
					),
					(
						// SS58 prefix 51: 66AJHj124JZnpKnWFoJGx41uSVqjyUz6tmVPfZWnhemBe8wG
						hex!["00804668906bbe7934ba89ef18c17c62c2dc175ca8df33621f2413e677c7144f"]
							.into(),
						hex!["00804668906bbe7934ba89ef18c17c62c2dc175ca8df33621f2413e677c7144f"]
							.unchecked_into(),
					),
				],
				// 66MGxr9zcyJ6ka6FBQmT1VSvMqARKfwBT7589Fikii1Ci5sg
				hex!["08df8338e854d8d589dedd4305c11e589cbef994e5dd00c7bb8fb7d277705b06"].into(),
				initial_allocation,
				REGISTERED_PARA_ID.into(),
				vec![
					b"AutomationTime".to_vec(),
					b"Balances".to_vec(),
					b"Bounties".to_vec(),
					b"Currencies".to_vec(),
					b"Democracy".to_vec(),
					b"ParachainStaking".to_vec(),
					b"PolkadotXcm".to_vec(),
					b"Treasury".to_vec(),
					b"XTokens".to_vec(),
				],
				initial_vesting,
				vec![
					// 69pKU2QpgtMBT9NsaN1diyJQ8qcvrJy8KJk5aWeAXfMGjb5F
					hex!["a23443cef4fe4e7ee3f61c8248505312fa81121f2a8bd64099390b40d7a05206"].into(),
					// 69Eyxo3gFpZpcRrtx5NAFBDMcoiVQrdg7so1M9w4FWiTBLej
					hex!["88c782a383f0cfa5fbc36c9734dfc7cb6319137b48cc69cc662c84b6e687a106"].into(),
					// 67D6ecyNhnAzZqgRbxr3MdGnxB9Bw8VadMhjpLAYB3wf5Pq6
					hex!["2edf0fd8948ea642f135b314b1358c77ec6d0a4af83220b6ea18136e5ce36277"].into(),
					// 6AxoUZEKEbaGVHsBkuJ6mvVgeMEixoroGiD1b3sBkroBqDXE
					hex!["d4e8bfb1c924dd64e33ecfbb35d90061bb83b2dde667e58588780068f9fc1471"].into(),
					// 67nmVh57G9yo7sqiGLjgNNqtUd7H2CSESTyQgp5272aMibwS
					hex!["488ced7d199b4386081a52505962128da5a3f54f4665db3d78b6e9f9e89eea4d"].into(),
				],
				vec![
					// 69pKU2QpgtMBT9NsaN1diyJQ8qcvrJy8KJk5aWeAXfMGjb5F
					hex!["a23443cef4fe4e7ee3f61c8248505312fa81121f2a8bd64099390b40d7a05206"].into(),
					// 67ppUQ3mGfaWDEr5VQRz4QgvNzXiGLmGjwwEcnvbJofSQyMN
					hex!["4a1d713c2f41e10db31b83a45a3a98b64088330190eac7fcef4623694fbac55e"].into(),
					// 6871Utgdq5ycWnTzm7VEwQfg4zM81JqYv8EDzKPu3fdzsXK8
					hex!["56766d3f856f2ca399ae575689a7340c2532351948f355c21bc06b170569da35"].into(),
					// 68Tn1mgDo1dvctJLZCWaYJWLSWgu4GYtL5jgrrmzidAgpoJG
					hex!["664d3e87b905d7e3fba4f459673af256166313c498972fb8cbba6e3697d7fe3b"].into(),
					// 67Za6G2i3BGkcTtYzU2hUzHrmy6quwWzyEMJB98oRv3GwDmb
					hex!["3e7c592124e2cde1808eeaa4c6799cb680506046bfc7b0304d9305bd3bd50b11"].into(),
					// 67nmVh57G9yo7sqiGLjgNNqtUd7H2CSESTyQgp5272aMibwS
					hex!["488ced7d199b4386081a52505962128da5a3f54f4665db3d78b6e9f9e89eea4d"].into(),
				],
				vec![],
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
			para_id: REGISTERED_PARA_ID,
		},
	)
}

const NUM_SELECTED_CANDIDATES: u32 = 6;
fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	para_id: ParaId,
	pallet_gates_closed: Vec<Vec<u8>>,
	vesting_schedule: Vec<(u64, Vec<(AccountId, Balance)>)>,
	general_councils: Vec<AccountId>,
	technical_memberships: Vec<AccountId>,
	xcmp_handler_asset_data: Vec<(Vec<u8>, XcmFlow)>,
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
			inflation_config: inflation_config(1800, 5),
			blocks_per_round: 1800,
			collator_commission: Perbill::from_percent(20),
			parachain_bond_reserve_percent: Percent::from_percent(30),
			num_selected_candidates: NUM_SELECTED_CANDIDATES,
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
		xcmp_handler: XcmpHandlerConfig { asset_data: xcmp_handler_asset_data },
		asset_registry: Default::default(),
	}
}
