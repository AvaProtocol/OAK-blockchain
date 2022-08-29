use serde::{Deserialize, Serialize};

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainSpec;
use sp_core::{Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

use pallet_parachain_staking::{InflationInfo, Range};

use primitives::{AccountId, AuraId, Balance, Signature};

#[cfg(feature = "neumann-node")]
pub mod neumann;
#[cfg(feature = "oak-node")]
pub mod oak;
#[cfg(feature = "turing-node")]
pub mod turing;

pub const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
pub type DummyChainSpec = sc_service::GenericChainSpec<(), Extensions>;

/// Can be called for a `Configuration` to check if it is a configuration for
/// the `OAK` network.
pub trait IdentifyVariant {
	/// Returns `true` if this is a configuration for the `Neumann` network.
	fn is_neumann(&self) -> bool;

	/// Returns `true` if this is a configuration for the `Turing` network.
	fn is_turing(&self) -> bool;

	/// Returns `true` if this is a configuration for the `OAK` network.
	fn is_oak(&self) -> bool;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_neumann(&self) -> bool {
		self.id().starts_with("neumann")
	}

	fn is_turing(&self) -> bool {
		self.id().starts_with("turing")
	}

	fn is_oak(&self) -> bool {
		self.id().starts_with("oak")
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

pub fn inflation_config(blocks_per_round: u32) -> InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>, blocks_per_round: u32) -> Range<Perbill> {
		use pallet_parachain_staking::inflation::{
			perbill_annual_to_perbill_round, BLOCKS_PER_YEAR,
		};
		perbill_annual_to_perbill_round(annual, BLOCKS_PER_YEAR / blocks_per_round)
	}

	let annual = Range {
		min: Perbill::from_percent(5),
		ideal: Perbill::from_percent(5),
		max: Perbill::from_percent(5),
	};

	InflationInfo {
		// We have no staking expectations since inflation range is a singular value
		expect: Range { min: 0, ideal: 0, max: 0 },
		annual,
		round: to_round_inflation(annual, blocks_per_round),
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sp_runtime::traits::Zero;

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
		assert_eq!(
			total_allocated, total_tokens,
			"total allocated does not equal the desired amount"
		);
	}

	/// Validate that the vested fits the following criteria:
	/// - allocated and vesting tokens add up to equal total number of expected tokens
	pub fn validate_total_tokens(
		allocated_accounts: Vec<(AccountId, Balance)>,
		vesting_timeslots: Vec<(u64, Vec<(AccountId, Balance)>)>,
		total_expected_tokens: u128,
	) {
		let mut total_tokens: Balance = Zero::zero();
		vesting_timeslots.iter().for_each(|(_, schedules)| {
			schedules.iter().for_each(|(_, amount)| {
				total_tokens = total_tokens
					.checked_add(*amount)
					.expect("shouldn't overflow when building genesis");
			});
		});
		allocated_accounts.iter().for_each(|(_, amount)| {
			total_tokens = total_tokens
				.checked_add(*amount)
				.expect("shouldn't overflow when building genesis");
		});

		assert_eq!(
			total_tokens, total_expected_tokens,
			"total vested does not equal the desired amount"
		);
	}

	/// Validate that the vested fits the following criteria:
	/// - no duplicate timestamps
	/// - no duplicate accountIds per timestamp
	/// - total amount vested is correct
	/// - times are within a given range
	pub fn validate_vesting(
		vesting_timeslots: Vec<(u64, Vec<(AccountId, Balance)>)>,
		total_tokens: u128,
		existential_deposit: Balance,
		vest_starting: u64,
		vest_ending: u64,
	) {
		let mut total_vested: Balance = Zero::zero();
		let unique_vested_slots = vesting_timeslots
			.iter()
			.map(|(timestamp, schedules)| {
				assert!(timestamp <= &vest_ending, "greater than largest expected timestamp.");
				assert!(timestamp >= &vest_starting, "smaller than smallest expected timestamp.");
				let unique_vesting_accounts = schedules
					.iter()
					.map(|(account_id, amount)| {
						assert!(*amount >= existential_deposit, "allocated amount must gte ED");
						total_vested = total_vested
							.checked_add(*amount)
							.expect("shouldn't overflow when building genesis");
						account_id
					})
					.cloned()
					.collect::<std::collections::BTreeSet<_>>();
				assert!(
					unique_vesting_accounts.len() == schedules.len(),
					"duplicate accounts per timeslot."
				);
				timestamp
			})
			.cloned()
			.collect::<std::collections::BTreeSet<_>>();
		assert!(
			unique_vested_slots.len() == vesting_timeslots.len(),
			"duplicate vesting timeslots in genesis."
		);

		assert_eq!(total_vested, total_tokens, "total vested does not equal the desired amount");
	}
}
