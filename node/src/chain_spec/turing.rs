use hex_literal::hex;

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};

use super::TELEMETRY_URL;
use crate::chain_spec::{
	get_account_id_from_seed, get_collator_keys_from_seed, validate_allocation, Extensions,
};
use primitives::{AccountId, AuraId, Balance};
use turing_runtime::{
	CouncilConfig, SudoConfig, ValveConfig, DOLLAR, EXISTENTIAL_DEPOSIT, TOKEN_DECIMALS,
};

static TOKEN_SYMBOL: &str = "TUR";
const SS_58_FORMAT: u32 = 51;
const TOTAL_TOKENS: u128 = DOLLAR * 1_000_000_000;
static RELAY_CHAIN: &str = "rococo-local";
static TURING_RELAY_CHAIN: &str = "ksmcc3";
const DEFAULT_PARA_ID: u32 = 2000;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<turing_runtime::GenesisConfig, Extensions>;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> turing_runtime::SessionKeys {
	turing_runtime::SessionKeys { aura: keys }
}

pub fn turing_development_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"Turing Development",
		// ID
		"turing-dev",
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
			let initial_balance: u128 = TOTAL_TOKENS / accounts.len() as u128;
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
			)
		},
		Vec::new(),
		None,
		None,
		None,
		None,
		Extensions {
			relay_chain: RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: DEFAULT_PARA_ID,
		},
	)
}
pub fn turing_latest_latest() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"Turing Network",
		// ID
		"turing",
		ChainType::Live,
		move || {
			let allocation_json = &include_bytes!("../../../distribution/neumann_alloc.json")[..];
			let initial_allocation: Vec<(AccountId, Balance)> =
				serde_json::from_slice(allocation_json).unwrap();

			validate_allocation(initial_allocation.clone(), TOTAL_TOKENS, EXISTENTIAL_DEPOSIT);

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
			)
		},
		// Bootnodes
		Vec::new(),
		// Telemetry
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		// Protocol ID
		Some("turing"),
		None,
		// Properties
		Some(properties),
		// Extensions
		Extensions {
			relay_chain: TURING_RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: DEFAULT_PARA_ID,
		},
	)
}

fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	id: ParaId,
	pallet_gates_closed: Vec<Vec<u8>>,
) -> turing_runtime::GenesisConfig {
	turing_runtime::GenesisConfig {
		system: turing_runtime::SystemConfig {
			code: turing_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		balances: turing_runtime::BalancesConfig { balances: endowed_accounts },
		parachain_info: turing_runtime::ParachainInfoConfig { parachain_id: id },
		collator_selection: turing_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
			..Default::default()
		},
		session: turing_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                 // account id
						acc,                         // validator id
						template_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		council: CouncilConfig { members: vec![root_key.clone()], phantom: Default::default() },
		parachain_system: Default::default(),
		sudo: SudoConfig { key: Some(root_key) },
		treasury: Default::default(),
		valve: ValveConfig { start_with_valve_closed: false, closed_gates: pallet_gates_closed },
		vesting: Default::default(),
	}
}
