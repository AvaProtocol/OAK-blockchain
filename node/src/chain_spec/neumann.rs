use hex_literal::hex;
use std::time::SystemTime;

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::{Perbill, Percent};

use super::TELEMETRY_URL;
use crate::chain_spec::{
	get_account_id_from_seed, get_collator_keys_from_seed, inflation_config, DummyChainSpec,
	Extensions,
};
use common_runtime::constants::currency::{DOLLAR, TOKEN_DECIMALS};
use neumann_runtime::{
	CouncilConfig, PolkadotXcmConfig, SudoConfig, TechnicalMembershipConfig, ValveConfig,
	VestingConfig, XcmpHandlerConfig,
};
use pallet_xcmp_handler::XcmFlow;
use primitives::{AccountId, AuraId, Balance};

static TOKEN_SYMBOL: &str = "NEU";
const SS_58_FORMAT: u32 = 51;
static RELAY_CHAIN: &str = "rococo-local";
static NEUMANN_RELAY_CHAIN: &str = "rococo-testnet";
const DEFAULT_PARA_ID: u32 = 2000;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<neumann_runtime::GenesisConfig, Extensions>;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> neumann_runtime::SessionKeys {
	neumann_runtime::SessionKeys { aura: keys }
}

pub fn development_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"Neumann Development",
		// ID
		"neumann-dev",
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
			const ALLOC_TOKENS_TOTAL: u128 = DOLLAR * 1_000_000_000;
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
				DEFAULT_PARA_ID.into(),
				vec![],
				vec![],
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
				],
				vec![
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				],
				vec![],
			)
		},
		Vec::new(),
		None,
		None,
		None,
		Some(properties),
		Extensions {
			relay_chain: RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: DEFAULT_PARA_ID,
		},
	)
}

pub fn local_testnet_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"Neumann Local Testnet",
		// ID
		"neumann_local_testnet",
		ChainType::Local,
		move || {
			let accounts = vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Dave"),
				get_account_id_from_seed::<sr25519::Public>("Eve"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			];
			const ALLOC_TOKENS_TOTAL: u128 = DOLLAR * 1_000_000_000;
			let initial_balance: u128 = ALLOC_TOKENS_TOTAL / accounts.len() as u128;
			let endowed_accounts: Vec<(AccountId, Balance)> =
				accounts.iter().cloned().map(|k| (k, initial_balance)).collect();

			let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
			let now_to_the_hour = now - (now % 3600);
			let month_in_seconds = 2_628_000;
			let vest_amount = 1_000_000_000_000;
			let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> = vec![
				(
					now_to_the_hour + month_in_seconds,
					vec![
						(accounts[0].clone(), vest_amount),
						(accounts[1].clone(), vest_amount),
						(accounts[2].clone(), vest_amount),
					],
				),
				(
					now_to_the_hour + 2 * month_in_seconds,
					vec![
						(accounts[0].clone(), vest_amount),
						(accounts[1].clone(), vest_amount),
						(accounts[2].clone(), vest_amount),
					],
				),
			];

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
				DEFAULT_PARA_ID.into(),
				vec![],
				initial_vesting,
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
				],
				vec![
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				],
				vec![],
			)
		},
		// Bootnodes
		Vec::new(),
		// Telemetry
		None,
		// Protocol ID
		Some("template-local"),
		None,
		// Properties
		Some(properties),
		// Extensions
		Extensions {
			relay_chain: RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: DEFAULT_PARA_ID,
		},
	)
}

pub fn neumann_staging_testnet_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"Neumann Network",
		// ID
		"neumann",
		ChainType::Live,
		move || {
			let allocation_json =
				&include_bytes!("../../../distribution/neumann_vest_test_alloc.json")[..];
			let initial_allocation: Vec<(AccountId, Balance)> =
				serde_json::from_slice(allocation_json).unwrap();

			let vesting_json = &include_bytes!("../../../distribution/neumann_vesting.json")[..];
			let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
				serde_json::from_slice(vesting_json).unwrap();

			testnet_genesis(
				// initial collators.
				vec![
					(
						// 5ECasnYivb8cQ4wBrQsdjwRTW4dzJ1ZcFqJNCLJwcc2N6WGL
						hex!["5e7aee4ee53ef08d5032ba5db9f7a6fdd9eef52423ac8c1aa960236377b46610"]
							.into(),
						hex!["5e7aee4ee53ef08d5032ba5db9f7a6fdd9eef52423ac8c1aa960236377b46610"]
							.unchecked_into(),
					),
					(
						// 5D2VxzUBZBkYtLxnpZ9uAV7Vht2Jz5MwqSco2GaqyLwGDZ4J
						hex!["2a8db6ca2e0cb5679e0eff0609de708c9957f465af49abbe7ff0a3594d52933e"]
							.into(),
						hex!["2a8db6ca2e0cb5679e0eff0609de708c9957f465af49abbe7ff0a3594d52933e"]
							.unchecked_into(),
					),
				],
				// 5GcD1vPdWzBd3VPTPgVFWL9K7b27A2tPYcVTJoGwKcLjdG5w
				hex!["c8f7b3791290f2d0f66a08b6ae1ebafe8d1efff56e31b0bb14e8d98157379028"].into(),
				initial_allocation,
				DEFAULT_PARA_ID.into(),
				vec![],
				initial_vesting,
				vec![
					// 67nmVh57G9yo7sqiGLjgNNqtUd7H2CSESTyQgp5272aMibwS
					hex!["488ced7d199b4386081a52505962128da5a3f54f4665db3d78b6e9f9e89eea4d"].into(),
					// 67kgfmY6zpw1PRYpj3D5RtkzVZnVvn49XHGyR4v9MEsRRyet
					hex!["46f630b3f79c588100dc0f69845633a830e01ea09eed4f1d01314a9bf33b9c16"].into(),
					// 67D6ecyNhnAzZqgRbxr3MdGnxB9Bw8VadMhjpLAYB3wf5Pq6
					hex!["2edf0fd8948ea642f135b314b1358c77ec6d0a4af83220b6ea18136e5ce36277"].into(),
					// 6AMsXyV1CYc3LMTk155JTDGEzbgVPvsX9aXp7VXz9heC3iuP
					hex!["ba44d2c00d9528c2d1fc51cef8ce8b9c3939928ecda8f404cdc46e3a2c090627"].into(),
				],
				vec![
					// 67nmVh57G9yo7sqiGLjgNNqtUd7H2CSESTyQgp5272aMibwS
					hex!["488ced7d199b4386081a52505962128da5a3f54f4665db3d78b6e9f9e89eea4d"].into(),
					// 67kgfmY6zpw1PRYpj3D5RtkzVZnVvn49XHGyR4v9MEsRRyet
					hex!["46f630b3f79c588100dc0f69845633a830e01ea09eed4f1d01314a9bf33b9c16"].into(),
					// 6A6VuGbeUwm3J2HqLduH7VFZTvrYQs8GuqzdhopLGN2JKMAe
					hex!["ae8b51cd0aa290645e593a4f54673ae62bab95791a137b943723bb6070533830"].into(),
					// 699YyPF2uA83zsFnQU4GCAvZXzucvyS5rx8LS9UrL9kEv8PP
					hex!["84a328f5f568d82ecd91861df7eae1065c1a2f1bcfec0950d4124e9363205b4a"].into(),
					// 669ocRxey7vxUJs1TTRWe31zwrpGr8B13zRfAHB6yhhfcMud
					hex!["001fbcefa8c96f3d2e236688da5485a0af67988b78d61ea952f461255d1f4267"].into(),
				],
				vec![],
			)
		},
		// Bootnodes
		Vec::new(),
		// Telemetry
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		// Protocol ID
		Some("neumann"),
		None,
		// Properties
		Some(properties),
		// Extensions
		Extensions {
			relay_chain: NEUMANN_RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: DEFAULT_PARA_ID,
		},
	)
}

pub fn neumann_latest() -> Result<DummyChainSpec, String> {
	DummyChainSpec::from_json_bytes(&include_bytes!("../../res/neumann.json")[..])
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
	xcmp_handler_transact_info: Vec<(Vec<u8>, XcmFlow)>,
) -> neumann_runtime::GenesisConfig {
	neumann_runtime::GenesisConfig {
		system: neumann_runtime::SystemConfig {
			code: neumann_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		balances: neumann_runtime::BalancesConfig { balances: endowed_accounts },
		parachain_info: neumann_runtime::ParachainInfoConfig { parachain_id: para_id },
		session: neumann_runtime::SessionConfig {
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
		parachain_staking: neumann_runtime::ParachainStakingConfig {
			candidates: invulnerables
				.iter()
				.cloned()
				.map(|(acc, _)| (acc, neumann_runtime::MinCollatorStk::get()))
				.collect(),
			delegations: vec![],
			inflation_config: inflation_config(25, 5),
			blocks_per_round: 25,
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
		xcmp_handler: XcmpHandlerConfig { transact_info: xcmp_handler_transact_info },
		asset_registry: Default::default(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::chain_spec::test::{validate_allocation, validate_total_tokens, validate_vesting};
	use common_runtime::constants::currency::EXISTENTIAL_DEPOSIT;

	#[test]
	fn validate_neumann_allocation() {
		let allocation_json = &include_bytes!("../../../distribution/neumann_alloc.json")[..];
		let initial_allocation: Vec<(AccountId, Balance)> =
			serde_json::from_slice(allocation_json).unwrap();

		const EXPECTED_ALLOC_TOKENS_TOTAL: u128 = DOLLAR * 1_000_000_000;
		validate_allocation(initial_allocation, EXPECTED_ALLOC_TOKENS_TOTAL, EXISTENTIAL_DEPOSIT);
	}

	#[test]
	fn validate_neumann_vesting() {
		let vesting_json = &include_bytes!("../../../distribution/neumann_vesting.json")[..];
		let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
			serde_json::from_slice(vesting_json).unwrap();

		let vested_tokens = DOLLAR * 10_000_000;
		let vest_starting_time: u64 = 1647212400;
		let vest_ending_time: u64 = 1647277200;
		validate_vesting(
			initial_vesting,
			vested_tokens,
			EXISTENTIAL_DEPOSIT,
			vest_starting_time,
			vest_ending_time,
		);
	}

	#[test]
	fn validate_total_neumann_tokens() {
		let allocation_json =
			&include_bytes!("../../../distribution/neumann_vest_test_alloc.json")[..];
		let initial_allocation: Vec<(AccountId, Balance)> =
			serde_json::from_slice(allocation_json).unwrap();

		let vesting_json = &include_bytes!("../../../distribution/neumann_vesting.json")[..];
		let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
			serde_json::from_slice(vesting_json).unwrap();

		let expected_vested_tokens = DOLLAR * 10_000_000;
		let expected_allocated_tokens = DOLLAR * 990_000_000;
		let expected_total_tokens = expected_vested_tokens + expected_allocated_tokens;
		validate_total_tokens(initial_allocation, initial_vesting, expected_total_tokens);
	}
}
