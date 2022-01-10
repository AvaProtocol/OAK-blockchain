use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use neumann_runtime::{AccountId, AuraId, CouncilConfig, Signature, SudoConfig, EXISTENTIAL_DEPOSIT, TOKEN_DECIMALS, DOLLAR};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{ crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

static TOKEN_SYMBOL: &str = "NEU";
const SS_58_FORMAT: u32 = 42;
const TOTAL_TOKENS: u128 = DOLLAR * 1_000_000_000;
static RELAY_CHAIN: &str = "rococo-local";
static NEUMANN_RELAY_CHAIN: &str = "rococo-testnet";
const DEFAULT_PARA_ID: u32 = 1000;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
	sc_service::GenericChainSpec<neumann_runtime::GenesisConfig, Extensions>;

/// Helper function to generate a crypto pair from seed
pub fn get_public_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	get_public_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_public_from_seed::<TPublic>(seed)).into_account()
}

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
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
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
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				DEFAULT_PARA_ID.into(),
			)
		},
		Vec::new(),
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
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
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
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				DEFAULT_PARA_ID.into(),
			)
		},
		// Bootnodes
		Vec::new(),
		// Telemetry
		None,
		// Protocol ID
		Some("template-local"),
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
			testnet_genesis(
				// initial collators.
				vec![
					(
						// 5Fq2CXrxNoUEb2MvaLWhU86QwCe1KzVGzodH81r5todNy8t5
						hex!["a6813e94ff1be02ee649ec51238046ec5124727f73a9b16e348b7d29d4869902"].into(),
						hex!["a6813e94ff1be02ee649ec51238046ec5124727f73a9b16e348b7d29d4869902"].unchecked_into(),
					),
					(
						// 5HEPAKfvJ5rG2N7LvYCxtq66H3tYqbiVrczSz7Erm2hGCGYH
						hex!["e48eafad4a882d37698016bb17e21beeb1da09856f210c4594a0bf8dcb5f4804"].into(),
						hex!["e48eafad4a882d37698016bb17e21beeb1da09856f210c4594a0bf8dcb5f4804"].unchecked_into(),
					),
				],
				// 5FtCDGK8KHu88V7sb2xxV6bkUYcjFpT3aiBHKUtK1jLXtP6d
				hex!["a8ecafa7dc50000365047c15c1542cc4875bec58de8b5ef7ed03e0a7111c0469"].into(),
				vec![
					// 5Cd7iTSbkuRqRJw791trBUZQq76Z4VPEuwyJwGpgW4ShzPvh
					hex!["18b82ae2626d2e644cc2aaca59c4f370359ed9ee1aa1be3a78d93d64d132f639"].into(),
					// 5CM2JyPHnbs81Cu8GzbraqHiwjeNwX3c9Rr5nXkJfwK9fwrk
					hex!["0c720beb3f580f0143f9cb18ae694cddb767161060850025a57a4f72a71bf475"].into(),
					// 5FtCDGK8KHu88V7sb2xxV6bkUYcjFpT3aiBHKUtK1jLXtP6d
					hex!["a8ecafa7dc50000365047c15c1542cc4875bec58de8b5ef7ed03e0a7111c0469"].into(),
				],
				DEFAULT_PARA_ID.into(),
			)
		},
		// Bootnodes
		Vec::new(),
		// Telemetry
		None,
		// Protocol ID
		Some("neumann"),
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
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> neumann_runtime::GenesisConfig {

	// this will result in there being slightly more tokens than TOTAL_TOKENS due to the invulnerables
	let initial_balance: u128 = TOTAL_TOKENS / endowed_accounts.len() as u128;

	neumann_runtime::GenesisConfig {
		system: neumann_runtime::SystemConfig {
			code: neumann_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		balances: neumann_runtime::BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, initial_balance)).collect(),
		},
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
		council: CouncilConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		parachain_system: Default::default(),
		sudo: SudoConfig { key: root_key },
		treasury: Default::default(),
	}
}
