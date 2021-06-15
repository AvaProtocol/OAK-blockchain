// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use sc_chain_spec::ChainSpecExtension;
use sp_core::{Pair, Public, crypto::UncheckedInto, sr25519};
use serde::{Serialize, Deserialize};
use node_runtime::{
	AuthorityDiscoveryConfig, BabeConfig, BalancesConfig, ContractsConfig, CouncilConfig,
	DemocracyConfig, GrandpaConfig, ImOnlineConfig, SessionConfig, SessionKeys, StakerStatus,
	StakingConfig, ElectionsConfig, IndicesConfig, SocietyConfig, SudoConfig, SystemConfig,
	TechnicalCommitteeConfig, QuadraticFundingConfig, wasm_binary_unwrap, MAX_NOMINATIONS,
};
use node_runtime::Block;
use node_runtime::constants::currency::*;
use sc_service::ChainType;
use hex_literal::hex;
use sc_telemetry::TelemetryEndpoints;
use grandpa_primitives::{AuthorityId as GrandpaId};
use sp_consensus_babe::{AuthorityId as BabeId};
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_runtime::{Perbill, traits::{Verify, IdentifyAccount}};
use serde_json::json;

pub use node_primitives::{AccountId, Balance, Signature};
pub use node_runtime::GenesisConfig;

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<
	GenesisConfig,
	Extensions,
>;
/// Flaming Fir testnet generator
pub fn flaming_fir_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../res/flaming-fir.json")[..])
}

/// Generate a json file as configuration template, which is called staging in Polkadot
pub fn oak_testnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../res/oak-testnet.json")[..])
}

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys { grandpa, babe, im_online, authority_discovery }
}

fn oak_testnet_staging_genesis() -> GenesisConfig {
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	// and
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
		// 5EjoVYN5HbPw2QSwYj9XYwtNNsrVF1TU6bnrbmdfzWLaAFxU
		hex!["764a025efb302d208cae1fbb51ffa65c2b7abc484368a15eb6b6be69ef94122a"].into(),
		// 5GF6qAfN4pV7NQMSxzuZdKk4qxA3ZQT1EVK9oZ9W6V6WYNvW
		hex!["b8deb9c2297ee63058e42e8e4952eef6d8bbe2eb6e7e6fdfe430772d158b8b63"].into(),
		// 5FUMe39FEpf4jj6tTejngTWmXVt4QBMkJi5oB4YKmrfZfbyG
		hex!["96be87ed33633afda69554ed5328b5f5d6044d0a06aeda02cc4eafebd8ece262"].unchecked_into(),
		// 5FCFNaghf2EntpLUaeTbw7tzSPLKEb4jLhkWLog1wM3CRdfZ
		hex!["8a7582769b817bcd095e3026fafb2224da12ac7fb9548874f964455695d86e24"].unchecked_into(),
		// 5GH9U2sgkoooh9q1K9h2FdTDQQeYP11L8xijVAMhRn2kaMfz
		hex!["ba6e16b5304ae48b9f54a8bef3fb35e7dd765bfc0fe395196b0782c3421c0267"].unchecked_into(),
		// 5H9hkdJ9c4Q2kc9MHPTz2kEV7qE5FwkWuKT68TTw4wu9rjgu
		hex!["e0fd0b2988f9bba881637050752751dfaaaac2c48467845b61e27ec033373154"].unchecked_into(),
	),
	(
		// 5Di4wgCb89dzsBdAUJ7EuHG1iqig4YwsZPanvgGvHWJgrm5q
		hex!["48bb4bdf2c6161eb2769d9e890d59d767e756f96208f5c968fc55493df692410"].into(),
		// 5CrJX7C8BKqXk2xysoLAdMT2Tvp8evzCLStEC5vvVzHcY4P4
		hex!["22c6ba76c451212e126839e8ea9ad22b3986f6f49b732a35b8e63b9c7d7b0b2d"].into(),
		// 5GrQiMws4P55KM28BGTxbgTXNSaeA5dYhhUCpyDFH1HrUqCz
		hex!["d3cc836042416bfc390a549138e246e5fd79cfa3fb108c6865997ebff5748caf"].unchecked_into(),
		// 5HZDAgdzhpT1WScY657Lrb5brWxvfRwu3mHxRDsjsJjaxV2L
		hex!["f2eab4f77aea2094bae843fcce05f6bcb3a6b74b5f364c606f13ad796d24b830"].unchecked_into(),
		// 5CGThHkLQoyKjeKRc9hb4kY14p3XnUG6aox827BUriDKUv8W
		hex!["08f745b48d623fe49504ae3ca5eb050f750be69367aa4990ae9b48943b319c17"].unchecked_into(),
		// 5HN7qTx9icR6VAAaWH9ogXkGCZPBioGkcvWGJmp3cN192Vfu
		hex!["ea750f7d2ccddae33b24f70b59de52539c124e55808ee08f0a869a88413bae30"].unchecked_into(),
	),
	(
		// 5CK3GsFHv7MV1VzuZUSGw2konT6s5ZePZnfejzfEjLof53dX
		hex!["0aeeccc18bfead22d07b6fd52fcf7491afa85a63955947fc2000898ac141ec2e"].into(),
		// 5ECaDi1LLj58Keyd9SJFZ2AFV1M7ctpTGxvU4SknsL3E94R4
		hex!["5e78b888fa1ae04301a097df022f7a14d344516ca35eddf269716c766bcc384e"].into(),
		// 5F6xW5Qizba9fy96zz3HCQU7tyvYVXXDU9HUTxXmoNcz8gs4
		hex!["866c7d92c89897fbbbd0edaa41035d51c58553b5d45c25cbd409a469fa4d0acb"].unchecked_into(),
		// 5Ft6DKmDdd4cjenkn655Bwz2hgtdU8JwAodAF8HyTYd6P283
		hex!["a8d87de70df910726c74c548c5440a4b1e5ddc388ae005f86db924f072143d35"].unchecked_into(),
		// 5Fn2LFErwRaUoxVeUs1Wwhq2apEh8sHP5zeDQUGYVJwGwkHY
		hex!["a437f4b7ed497b252402fed1d6d6f8c49a2e0c3f5e444bc188a717174508ea22"].unchecked_into(),
		// 5FHuEDajibNoESKEqmJ2cm49RJUtqojGS98JJ7rpU7Hem5ZK
		hex!["8ec52bad42084ec427402792e5c2b44676c4286b66da71b006daca456ed9273f"].unchecked_into(),
	),];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// 5GcD1vPdWzBd3VPTPgVFWL9K7b27A2tPYcVTJoGwKcLjdG5w
		"c8f7b3791290f2d0f66a08b6ae1ebafe8d1efff56e31b0bb14e8d98157379028"
	].into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	testnet_genesis(initial_authorities, vec![], root_key, Some(endowed_accounts), false)
}

/// oak testnet config.
pub fn oak_testnet_staging_config() -> ChainSpec {
	let boot_nodes = vec!["/dns/testnet.oak.tech/tcp/30333/p2p/12D3KooWBpDWtYunHni9Bz3f4fZt9PZ6Cw8uNKLdiBF5NJbQPPVt".parse().unwrap()];
	ChainSpec::from_genesis(
		"OAK Testnet Staging",
		"oak_testnet_staging",
		ChainType::Live,
		oak_testnet_staging_genesis,
		boot_nodes,
		Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Staging telemetry url is valid; qed")),
		None,
		Some(json!({"tokenDecimals": 10, "tokenSymbol": "OAK"}).as_object().expect("network properties generation can not fail; qed").to_owned()),
		Default::default(),
	)
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(seed: &str) -> (
	AccountId,
	AccountId,
	GrandpaId,
	BabeId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	initial_nominators: Vec<AccountId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
	enable_println: bool,
) -> GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
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
		]
	});
	// endow all authorities and nominators.
	initial_authorities.iter().map(|x| &x.0).chain(initial_nominators.iter()).for_each(|x| {
		if !endowed_accounts.contains(&x) {
			endowed_accounts.push(x.clone())
		}
	});

	// stakers: all validators and nominators.
	let mut rng = rand::thread_rng();
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|x| {
			use rand::{seq::SliceRandom, Rng};
			let limit = (MAX_NOMINATIONS as usize).min(initial_authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = initial_authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.into_iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	const STASH: Balance = ENDOWMENT / 1000;

	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|x| (x, ENDOWMENT))
				.collect()
		},
		pallet_indices: IndicesConfig {
			indices: vec![],
		},
		pallet_session: SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), session_keys(
					x.2.clone(),
					x.3.clone(),
					x.4.clone(),
					x.5.clone(),
				))
			}).collect::<Vec<_>>(),
		},
		pallet_staking: StakingConfig {
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			.. Default::default()
		},
		pallet_democracy: DemocracyConfig::default(),
		pallet_elections_phragmen: ElectionsConfig {
			members: endowed_accounts.iter()
						.take((num_endowed_accounts + 1) / 2)
						.cloned()
						.map(|member| (member, STASH))
						.collect(),
		},
		pallet_collective_Instance1: CouncilConfig::default(),
		pallet_collective_Instance2: TechnicalCommitteeConfig {
			members: endowed_accounts.iter()
						.take((num_endowed_accounts + 1) / 2)
						.cloned()
						.collect(),
			phantom: Default::default(),
		},
		pallet_contracts: ContractsConfig {
			// println should only be enabled on development chains
			current_schedule: pallet_contracts::Schedule::default()
				.enable_println(enable_println),
		},
		pallet_sudo: SudoConfig {
			key: root_key,
		},
		pallet_babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(node_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		pallet_im_online: ImOnlineConfig {
			keys: vec![],
		},
		pallet_authority_discovery: AuthorityDiscoveryConfig {
			keys: vec![],
		},
		pallet_grandpa: GrandpaConfig {
			authorities: vec![],
		},
		pallet_membership_Instance1: Default::default(),
		pallet_treasury: Default::default(),
		pallet_society: SocietyConfig {
			members: endowed_accounts.iter()
						.take((num_endowed_accounts + 1) / 2)
						.cloned()
						.collect(),
			pot: 0,
			max_members: 999,
		},
		pallet_vesting: Default::default(),
		pallet_gilt: Default::default(),
		pallet_quadratic_funding: QuadraticFundingConfig {
			init_max_grant_count_per_round: 60,
			init_withdrawal_expiration: 1000,
			init_is_identity_required: false,
		},
	}
}

fn development_config_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![
			authority_keys_from_seed("Alice"),
		],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
		true,
	)
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		development_config_genesis,
		vec![],
		None,
		None,
		None,
		Default::default(),
	)
}

fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
		],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
		false,
	)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		local_testnet_genesis,
		vec![],
		None,
		None,
		None,
		Default::default(),
	)
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use crate::service::{new_full_base, new_light_base, NewFullBase};
	use sc_service_test;
	use sp_runtime::BuildStorage;

	fn local_testnet_genesis_instant_single() -> GenesisConfig {
		testnet_genesis(
			vec![
				authority_keys_from_seed("Alice"),
			],
			vec![],
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			None,
			false,
		)
	}

	/// Local testnet config (single validator - Alice)
	pub fn integration_test_config_with_single_authority() -> ChainSpec {
		ChainSpec::from_genesis(
			"Integration Test",
			"test",
			ChainType::Development,
			local_testnet_genesis_instant_single,
			vec![],
			None,
			None,
			None,
			Default::default(),
		)
	}

	/// Local testnet config (multivalidator Alice + Bob)
	pub fn integration_test_config_with_two_authorities() -> ChainSpec {
		ChainSpec::from_genesis(
			"Integration Test",
			"test",
			ChainType::Development,
			local_testnet_genesis,
			vec![],
			None,
			None,
			None,
			Default::default(),
		)
	}

	#[test]
	#[ignore]
	fn test_connectivity() {
		sc_service_test::connectivity(
			integration_test_config_with_two_authorities(),
			|config| {
				let NewFullBase { task_manager, client, network, transaction_pool, .. }
					= new_full_base(config,|_, _| ())?;
				Ok(sc_service_test::TestNetComponents::new(task_manager, client, network, transaction_pool))
			},
			|config| {
				let (keep_alive, _, client, network, transaction_pool) = new_light_base(config)?;
				Ok(sc_service_test::TestNetComponents::new(keep_alive, client, network, transaction_pool))
			}
		);
	}

	#[test]
	fn test_create_development_chain_spec() {
		development_config().build_storage().unwrap();
	}

	#[test]
	fn test_create_local_testnet_chain_spec() {
		local_testnet_config().build_storage().unwrap();
	}

	#[test]
	fn test_staging_test_net_chain_spec() {
		testnet_config().build_storage().unwrap();
	}
}
