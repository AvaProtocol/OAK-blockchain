use hex_literal::hex;

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};

use crate::chain_spec::{
	get_account_id_from_seed, get_collator_keys_from_seed, validate_allocation, Extensions,
};
use neumann_runtime::{
	CouncilConfig, SudoConfig, ValveConfig, DOLLAR, EXISTENTIAL_DEPOSIT, TOKEN_DECIMALS,
};
use primitives::{AccountId, AuraId, Balance};
use super::TELEMETRY_URL;

static TOKEN_SYMBOL: &str = "NEU";
const SS_58_FORMAT: u32 = 51;
const TOTAL_TOKENS: u128 = DOLLAR * 1_000_000_000;
static RELAY_CHAIN: &str = "rococo-local";
static NEUMANN_RELAY_CHAIN: &str = "rococo-testnet";
const DEFAULT_PARA_ID: u32 = 2000;

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
			let accounts = vec![
				// 5Cd7iTSbkuRqRJw791trBUZQq76Z4VPEuwyJwGpgW4ShzPvh
				hex!["18b82ae2626d2e644cc2aaca59c4f370359ed9ee1aa1be3a78d93d64d132f639"].into(),
				// 5CM2JyPHnbs81Cu8GzbraqHiwjeNwX3c9Rr5nXkJfwK9fwrk
				hex!["0c720beb3f580f0143f9cb18ae694cddb767161060850025a57a4f72a71bf475"].into(),
				// 5GcD1vPdWzBd3VPTPgVFWL9K7b27A2tPYcVTJoGwKcLjdG5w
				hex!["c8f7b3791290f2d0f66a08b6ae1ebafe8d1efff56e31b0bb14e8d98157379028"].into(),
			];
			let initial_balance: u128 = TOTAL_TOKENS / accounts.len() as u128;
			let endowed_accounts: Vec<(AccountId, Balance)> =
				accounts.iter().cloned().map(|k| (k, initial_balance)).collect();

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
				endowed_accounts,
				DEFAULT_PARA_ID.into(),
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

pub fn neumann_latest() -> ChainSpec {
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

fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	id: ParaId,
	pallet_gates_closed: Vec<Vec<u8>>,
) -> neumann_runtime::GenesisConfig {
	neumann_runtime::GenesisConfig {
		system: neumann_runtime::SystemConfig {
			code: neumann_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		balances: neumann_runtime::BalancesConfig { balances: endowed_accounts },
		parachain_info: neumann_runtime::ParachainInfoConfig { parachain_id: id },
		collator_selection: neumann_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
			..Default::default()
		},
		session: neumann_runtime::SessionConfig {
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
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn validate_neumann_allocation() {
		let allocation_json = &include_bytes!("../../../distribution/neumann_alloc.json")[..];
		let initial_allocation: Vec<(AccountId, Balance)> =
			serde_json::from_slice(allocation_json).unwrap();

		validate_allocation(initial_allocation, TOTAL_TOKENS, EXISTENTIAL_DEPOSIT);
	}
}
