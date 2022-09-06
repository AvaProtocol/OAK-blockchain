use hex_literal::hex;

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};

use super::TELEMETRY_URL;
use crate::chain_spec::{
	get_account_id_from_seed, get_collator_keys_from_seed, inflation_config,
	test::{validate_allocation, validate_vesting},
	DummyChainSpec, Extensions,
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
const REGISTERED_PARA_ID: u32 = 2114;
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

pub fn oak_staging() -> Result<DummyChainSpec, String> {
	DummyChainSpec::from_json_bytes(&include_bytes!("../../res/oak-staging.json")[..])
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
