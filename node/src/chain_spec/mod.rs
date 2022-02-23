use serde::{Deserialize, Serialize};

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainSpec;
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify, Zero};

use primitives::{AccountId, AuraId, Balance, Signature};

#[cfg(feature = "neumann-node")]
pub mod neumann;
#[cfg(feature = "turing-node")]
pub mod turing;

pub type DummyChainSpec = sc_service::GenericChainSpec<(), Extensions>;

/// Can be called for a `Configuration` to check if it is a configuration for
/// the `OAK` network.
pub trait IdentifyVariant {
	/// Returns `true` if this is a configuration for the `Neumann` network.
	fn is_neumann(&self) -> bool;

	/// Returns `true` if this is a configuration for the `Turing` network.
	fn is_turing(&self) -> bool;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_neumann(&self) -> bool {
		self.id().starts_with("neumann")
	}

	fn is_turing(&self) -> bool {
		self.id().starts_with("turing")
	}
}

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

/// Validate that the allocated fits the following criteria:
/// - no duplicate accounts
/// - total allocated is equal to total_tokens
pub fn validate_allocation(
	allocated_accounts: Vec<(AccountId, Balance)>,
	total_tokens: u128,
	existential_deposit: Balance,
) {
	let mut total_allocated: Balance = Zero::zero();
	let unique_allocated_accounts = allocated_accounts
		.iter()
		.map(|(account_id, amount)| {
			assert!(*amount >= existential_deposit, "allocated amount must gte ED");
			total_allocated = total_allocated
				.checked_add(*amount)
				.expect("shouldn't overflow when building genesis");

			account_id
		})
		.cloned()
		.collect::<std::collections::BTreeSet<_>>();
	assert!(
		unique_allocated_accounts.len() == allocated_accounts.len(),
		"duplicate allocated accounts in genesis."
	);
	assert_eq!(total_allocated, total_tokens, "total allocated does not equal the desired amount");
}
