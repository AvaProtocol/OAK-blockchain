use cumulus_primitives_core::ParaId;
use node_runtime::{/*AccountId, AuraId, */Signature, GenesisConfig};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

pub const TURING_PARA_ID: ParaId = ParaId::new(2001);
pub const EXISTENTIAL_DEPOSIT: u64 = 10000;

/// Specialized `ChainSpec` for the normal parachain runtime.
// TODO(irsal): point to parahcain runtime
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>; 


/// Helper function to generate a crypto pair from seed
pub fn get_pair_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
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
// pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
// 	get_pair_from_seed::<AuraId>(seed)
// }

/// Helper function to generate an account ID from seed
// pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
// where
// 	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
// {
// 	AccountPublic::from(get_pair_from_seed::<TPublic>(seed)).into_account()
// }

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
// pub fn session_keys(keys: AuraId) -> node_runtime::SessionKeys {
// 	//node_runtime::SessionKeys { aura: keys }
// }

// pub fn turing_development_config() -> ChainSpec  {
// 	development_config(TURING_PARA_ID)
// }

// fn development_config(id: ParaId) -> ChainSpec {
// 	// // Give your base currency a unit name and decimal places
// 	// let mut properties = sc_chain_spec::Properties::new();
// 	// properties.insert("tokenSymbol".into(), "TUR".into());
// 	// properties.insert("tokenDecimals".into(), 10.into());
// 	// properties.insert("ss58Format".into(), 42.into());

	
// 	// ChainSpec::from_genesis(
// 	// 	// Name
// 	// 	"Development",
// 	// 	// ID
// 	// 	"dev",
// 	// 	ChainType::Development,
// 	// 	move || {
// 	// 		testnet_genesis(
// 	// 			// initial collators.
// 	// 			vec![
// 	// 				(
// 	// 					get_account_id_from_seed::<sr25519::Public>("Alice"),
// 	// 					get_collator_keys_from_seed("Alice"),
// 	// 				),
// 	// 				(
// 	// 					get_account_id_from_seed::<sr25519::Public>("Bob"),
// 	// 					get_collator_keys_from_seed("Bob"),
// 	// 				),
// 	// 			],
// 	// 			vec![
// 	// 				get_account_id_from_seed::<sr25519::Public>("Alice"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Bob"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Charlie"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Dave"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Eve"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Ferdie"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
// 	// 				get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
// 	// 			],
// 	// 			id,
// 	// 		)
// 	// 	},
// 	// 	vec![],
// 	// 	None,
// 	// 	None,
// 	// 	None,
// 	// 	Extensions {
// 	// 		relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
// 	// 		para_id: id.into(),
// 	// 	},
// 	// )
// }

pub fn turing_local_testnet_config() -> Result<ChainSpec, String>  {
	local_testnet_config(TURING_PARA_ID)
}

fn local_testnet_config(id: ParaId) -> Result<ChainSpec, String>  {
	ChainSpec::from_json_bytes(&include_bytes!("../../res/oak-testnet.json")[..])
}


// pub fn local_testnet_config(id: ParaId) -> ChainSpec {
// 	// Give your base currency a unit name and decimal places
// 	let mut properties = sc_chain_spec::Properties::new();
// 	properties.insert("tokenSymbol".into(), "TUR".into());
// 	properties.insert("tokenDecimals".into(), 10.into());
// 	properties.insert("ss58Format".into(), 42.into());

// 	ChainSpec::from_genesis(
// 		// Name
// 		"Local Testnet",
// 		// ID
// 		"local_testnet",
// 		ChainType::Local,
// 		move || {
// 			testnet_genesis(
// 				// initial collators.
// 				vec![
// 					(
// 						get_account_id_from_seed::<sr25519::Public>("Alice"),
// 						get_collator_keys_from_seed("Alice"),
// 					),
// 					(
// 						get_account_id_from_seed::<sr25519::Public>("Bob"),
// 						get_collator_keys_from_seed("Bob"),
// 					),
// 				],
// 				vec![
// 					get_account_id_from_seed::<sr25519::Public>("Alice"),
// 					get_account_id_from_seed::<sr25519::Public>("Bob"),
// 					get_account_id_from_seed::<sr25519::Public>("Charlie"),
// 					get_account_id_from_seed::<sr25519::Public>("Dave"),
// 					get_account_id_from_seed::<sr25519::Public>("Eve"),
// 					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
// 					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
// 					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
// 					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
// 					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
// 					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
// 					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
// 				],
// 				id,
// 			)
// 		},
// 		// Bootnodes
// 		vec![],
// 		// Telemetry
// 		None,
// 		// Protocol ID
// 		Some("template-local"),
// 		// Properties
// 		Some(properties),
// 		// Extensions
// 		Extensions {
// 			relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
// 			para_id: id.into(),
// 		},
// 	)
// }

// fn testnet_genesis(
// 	invulnerables: Vec<(AccountId, AuraId)>,
// 	endowed_accounts: Vec<AccountId>,
// 	id: ParaId,
// ) -> parachain_template_runtime::GenesisConfig {
// 	parachain_template_runtime::GenesisConfig {
// 		system: parachain_template_runtime::SystemConfig {
// 			code: parachain_template_runtime::WASM_BINARY
// 				.expect("WASM binary was not build, please build it!")
// 				.to_vec(),
// 			changes_trie_config: Default::default(),
// 		},
// 		balances: parachain_template_runtime::BalancesConfig {
// 			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
// 		},
// 		parachain_info: parachain_template_runtime::ParachainInfoConfig { parachain_id: id },
// 		collator_selection: parachain_template_runtime::CollatorSelectionConfig {
// 			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
// 			candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
// 			..Default::default()
// 		},
// 		session: parachain_template_runtime::SessionConfig {
// 			keys: invulnerables
// 				.iter()
// 				.cloned()
// 				.map(|(acc, aura)| {
// 					(
// 						acc.clone(),                 // account id
// 						acc,                         // validator id
// 						template_session_keys(aura), // session keys
// 					)
// 				})
// 				.collect(),
// 		},
// 		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
// 		// of this.
// 		aura: Default::default(),
// 		aura_ext: Default::default(),
// 		parachain_system: Default::default(),
// 	}
// }
