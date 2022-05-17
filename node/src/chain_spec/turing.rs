use hex_literal::hex;

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};

use super::TELEMETRY_URL;
use crate::chain_spec::{
	get_account_id_from_seed, get_collator_keys_from_seed, validate_allocation, validate_vesting,
	DummyChainSpec, Extensions,
};
use primitives::{AccountId, AuraId, Balance};
use turing_runtime::{
	CouncilConfig, SudoConfig, TechnicalMembershipConfig, ValveConfig, VestingConfig, DOLLAR,
	EXISTENTIAL_DEPOSIT, TOKEN_DECIMALS,
};

const TOKEN_SYMBOL: &str = "TUR";
const SS_58_FORMAT: u32 = 51;
static RELAY_CHAIN: &str = "rococo-local";
static STAGING_RELAY_CHAIN: &str = "rococo";
const DEFAULT_PARA_ID: u32 = 2000;
const REGISTERED_PARA_ID: u32 = 2114;

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
				DEFAULT_PARA_ID.into(),
				vec![b"AutomationTime".to_vec(), b"Balances".to_vec(), b"Democracy".to_vec()],
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
			para_id: DEFAULT_PARA_ID,
		},
	)
}

pub fn turing_staging() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), SS_58_FORMAT.into());

	ChainSpec::from_genesis(
		// Name
		"Turing Staging",
		// ID
		"turing",
		ChainType::Live,
		move || {
			let allocation_json = &include_bytes!("../../../distribution/turing_staging_alloc.json")[..];
			let initial_allocation: Vec<(AccountId, Balance)> =
				serde_json::from_slice(allocation_json).unwrap();
			const ALLOC_TOKENS_TOTAL: u128 = DOLLAR * 1_000_000_000;
			validate_allocation(
				initial_allocation.clone(),
				ALLOC_TOKENS_TOTAL,
				EXISTENTIAL_DEPOSIT,
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
				REGISTERED_PARA_ID.into(),
				vec![],
				vec![],
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
		Some("turing"),
		None,
		// Properties
		Some(properties),
		// Extensions
		Extensions {
			relay_chain: STAGING_RELAY_CHAIN.into(), // You MUST set this to the correct network!
			para_id: REGISTERED_PARA_ID,
		},
	)
}

pub fn turing_live() -> Result<DummyChainSpec, String> {
	DummyChainSpec::from_json_bytes(&include_bytes!("../../res/turing.json")[..])
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
) -> turing_runtime::GenesisConfig {
	turing_runtime::GenesisConfig {
		system: turing_runtime::SystemConfig {
			code: turing_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		balances: turing_runtime::BalancesConfig { balances: endowed_accounts },
		parachain_info: turing_runtime::ParachainInfoConfig { parachain_id: para_id },
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
		// Defaults to active collators from session pallet unless configured otherwise
		parachain_staking: Default::default(),
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		council: CouncilConfig { members: general_councils, phantom: Default::default() },
		democracy: Default::default(),
		tokens: Default::default(),
		technical_committee: Default::default(),
		technical_membership: TechnicalMembershipConfig {
			members: technical_memberships,
			phantom: Default::default(),
		},
		parachain_system: Default::default(),
		sudo: SudoConfig { key: Some(root_key) },
		treasury: Default::default(),
		valve: ValveConfig { start_with_valve_closed: false, closed_gates: pallet_gates_closed },
		vesting: VestingConfig { vesting_schedule },
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn validate_turing_allocation() {
		let allocation_json = &include_bytes!("../../../distribution/turing_alloc.json")[..];
		let initial_allocation: Vec<(AccountId, Balance)> =
			serde_json::from_slice(allocation_json).unwrap();
		const EXPECTED_ALLOC_TOKENS_TOTAL: u128 = DOLLAR * 58_000_000;
		validate_allocation(initial_allocation, EXPECTED_ALLOC_TOKENS_TOTAL, EXISTENTIAL_DEPOSIT);
	}

	#[test]
	fn validate_turing_vesting() {
		let vesting_json = &include_bytes!("../../../distribution/turing_vesting.json")[..];
		let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
			serde_json::from_slice(vesting_json).unwrap();

		let vested_tokens = 9_419_999_999_999_999_919;
		let vest_starting_time: u64 = 1651431600;
		let vest_ending_time: u64 = 1743534000;
		validate_vesting(
			initial_vesting,
			vested_tokens,
			EXISTENTIAL_DEPOSIT,
			vest_starting_time,
			vest_ending_time,
		);
	}

	#[test]
	fn validate_total_turing_tokens() {
		use crate::chain_spec::validate_total_tokens;
		let allocation_json = &include_bytes!("../../../distribution/turing_alloc.json")[..];
		let initial_allocation: Vec<(AccountId, Balance)> =
			serde_json::from_slice(allocation_json).unwrap();

		let vesting_json = &include_bytes!("../../../distribution/turing_vesting.json")[..];
		let initial_vesting: Vec<(u64, Vec<(AccountId, Balance)>)> =
			serde_json::from_slice(vesting_json).unwrap();

		let expected_vested_tokens = 9_419_999_999_999_999_919;
		let expected_allocated_tokens = DOLLAR * 58_000_000;
		let expected_total_tokens = expected_vested_tokens + expected_allocated_tokens;
		validate_total_tokens(initial_allocation, initial_vesting, expected_total_tokens);
	}
}
