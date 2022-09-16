use serde::{Deserialize, Serialize};

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainSpec;
use sp_core::{Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

use pallet_parachain_staking::{InflationInfo, Range};
use std::collections::HashMap;

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

pub mod test {
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

	/// Validate that the vested fits the following criteria:
	/// - amounts per month
	pub fn vesting_monthly_totals(
		vesting_timeslots: Vec<(u64, Vec<(AccountId, Balance)>)>,
		dollar: u128,
		base_amount: Balance,
	) {
		let mut month_total: Balance = 0;
		vesting_timeslots
			.iter()
			.for_each(|(timestamp, schedules)| {
				schedules
					.iter()
					.for_each(|(_, amount)| {
						println!("{}", month_total);
						println!("{}", amount);
						month_total = month_total
							.checked_add(*amount)
							.expect("shouldn't overflow when building genesis");
						println!("{}", month_total);
					});
				match timestamp {
					1672603200 => {
						assert_eq!(
							month_total, 697916660000000000
						);
					},
					1675281600 => {
						assert_eq!(
							month_total, 780274810000000000
						);
					},
					1677700800 => {
						assert_eq!(
							month_total, 862632960000000000
						);
					},
					1680375600 => {
						assert_eq!(
							month_total, 1021397360000000000 //40 to 36
						);
					},
					1682967600 => {
						assert_eq!(
							month_total, 1103755510000000000
						);
					},
					1685646000 => {
						assert_eq!(
							month_total, 1346113660000000000
						);
					},
					1688238000 => {
						assert_eq!(
							month_total, 1444878060000000000
						);
					},
					1690916400 => {
						assert_eq!(
							month_total, 1587236210000000000
						);
					},
					1693594800 => {
						assert_eq!(
							month_total, 1669594360000000000
						);
					},
					1696186800 => {
						assert_eq!(
							month_total, 1768358760000000000
						);
					},
					1698865200 => {
						assert_eq!(
							month_total, 1850716910000000000
						);
					},
					1701460800 => {
						assert_eq!(
							month_total, 2638075060000000000
						);
					},
					1704139200 => {
						assert_eq!(
							month_total, 2695172800000000000
						);
					},
					1706817600 => {
						assert_eq!(
							month_total, 2735864290000000000
						);
					},
					1709323200 => {
						assert_eq!(
							month_total, 3381555780000000000
						);
					},
					1711998000 => {
						assert_eq!(
							month_total, 3498653520000000000
						);
					},
					1714590000 => {
						assert_eq!(
							month_total, 3539345010000000000
						);
					},
					1717268400 => {
						assert_eq!(
							month_total, 4185036500000000000
						);
					},
					1719860400 => {
						assert_eq!(
							month_total, 4242134240000000000
						);
					},
					1722538800 => {
						assert_eq!(
							month_total, 4342825730000000000
						);
					},
					1725217200 => {
						assert_eq!(
							month_total, 4988517220000000000
						);
					},
					1727809200 => {
						assert_eq!(
							month_total, 5045614960000000000
						);
					},
					1730487600 => {
						assert_eq!(
							month_total, 5086306450000000000
						);
					},
					1733083200 => {
						assert_eq!(
							month_total, 5731997940000000000
						);
					},
					1735761600 => {
						assert_eq!(
							month_total, 5789095680000000000
						);
					},
					1738440000 => {
						assert_eq!(
							month_total, 5829787170000000000
						);
					},
					1740859200 => {
						assert_eq!(
							month_total, 5995478660000000000
						);
					},
					1743534000 => {
						assert_eq!(
							month_total, 6036170150000000000
						);
					},
					1746126000 => {
						assert_eq!(
							month_total, 6076861640000000000
						);
					},
					1748804400 => {
						assert_eq!(
							month_total, 6242553130000000000
						);
					},
					1751396400 => {
						assert_eq!(
							month_total, 6283244620000000000
						);
					},
					1754074800 => {
						assert_eq!(
							month_total, 6323936110000000000
						);
					},
					1756753200 => {
						assert_eq!(
							month_total, 6489627600000000000
						);
					},
					1759345200 => {
						assert_eq!(
							month_total, 6530319090000000000
						);
					},
					1762023600 => {
						assert_eq!(
							month_total, 6571010580000000000
						);
					},
					1764619200 => {
						assert_eq!(
							month_total, 6736702070000000000
						);
					},
					1767297600 => {
						assert_eq!(
							month_total, 6777393560000000000
						);
					},
					1769976000 => {
						assert_eq!(
							month_total, 6818085050000000000
						);
					},
					1772395200 => {
						assert_eq!(
							month_total, 6983776540000000000
						);
					},
					1775070000 => {
						assert_eq!(
							month_total, 7024468030000000000
						);
					},
					1777662000 => {
						assert_eq!(
							month_total, 7065159520000000000
						);
					},
					1780340400 => {
						assert_eq!(
							month_total, 7230851010000000000
						);
					},
					1782932400 => {
						assert_eq!(
							month_total, 7271542500000000000
						);
					},
					1785610800 => {
						assert_eq!(
							month_total, 7312233990000000000
						);
					},
					1788289200 => {
						assert_eq!(
							month_total, 7477925480000000000
						);
					},
					1790881200 => {
						assert_eq!(
							month_total, 7518616970000000000
						);
					},
					1793563200 => {
						assert_eq!(
							month_total, 7559308460000000000
						);
					},
					1796155200 => {
						assert_eq!(
							month_total, 7599999950000000000
						);
					},
					_ => {
						println!("month total: {}", month_total);
					}
				}
			});
	}


	/// Validate that the vested fits the following criteria:
	/// - amounts per address
	pub fn vesting_address_totals(
		vesting_timeslots: Vec<(u64, Vec<(AccountId, Balance)>)>,
		base_amount: Balance,
	) {
		let mut accounts: HashMap<AccountId, u128> = HashMap::new();
		vesting_timeslots
			.iter()
			.for_each(|(_timestamp, schedules)| {
				schedules
					.iter()
					.for_each(|(account_id, amount)| {
						let mut new_amount: u128 = *amount;
						let account_str = account_id.clone();
						println!("key: {}, string: {}", account_id, account_str);
						if accounts.contains_key(&account_str) {
							if let Some(old_amount) = accounts.get(&account_str) {
								new_amount = new_amount + old_amount;
							}
						}
						accounts.insert(account_str, new_amount);
					});
			});
		let mut grand_total: u128 = 0;
		accounts.iter().for_each(|(key, amount)| {
			println!("key: {} val: {}", *key, amount);
			grand_total += *amount;
		});
		println!("{grand_total}");
	// strat_part: 5DbvVzgMUaakg9ZCAsJ9Wx7XngiTUpwNLkNJaP6KrA7rkjDd val: 999999999999999972 //67grpvYywCTJ5gCNKfDMv4WMc416yUc43DMV7Xt31rUFHexc
	// dev_incentives: 5EPUXN1Unn8Zbdbue8qG5vLNtbQfRRUD7fyM1x67pfSXzDac val: 1499999999999999976 //68UQrHt7FQ171AF5nvkUV2jChxhJv58tp8xXZ6spzMnvWvFN
	// ecosystem_development: 5CamukJsXYQKhYq4h9Mm8fuMnZd66ViFuTsM4n28LrEvZdje val: 1499999999999999976 //66fiEgBVzAGs75UEqwGyXnJBbvujb9NwbvrXbvoqWYbK6Uui
	// crowdloan: 5H6LdMvqX4qqqWdNn7ZbAMm4uZ4vSz74fnvECtX2VXQtHfwb val: 1119999999999999999 //6BBGxHoTygiPF3GYvuUoZU9tivMZwdmkNFuQk3JjfDmGpbVC
	// community_dao: 5CFe6mCFDdsfAPt4PjrN4wdCoqnNgqZaTGBXp8PfgVpYWoPJ val: 1000000000000000000 //66LaRh4sgFkCZvXEYXmaU422dD52BVEG9jAiMHBNrCAw3g6h
	// earlybackers: 5HWUKbTo7J91swVmq7RgLgsWGEisiuGD8rDQx5ZBJoTHZJyP val: 800000000000000000 //6BbQeXLRZv1ZHU8wyuLtjoGL5c1XDYvtqKCbVELtUVog6MAJ
	// team: 5DsfkzRDrzL3n7jw1QJQHqGeo2AC7VAGaFzdQGVgp87X7YAg val: 1599999999999999996 //67xc5vHrKcCbBeP7ACDcgwfUcPSqc8pxGiyowRHPypTueU4d
	// parachainslotreserve: 5DFXtRzj8nAEVvFWESwsvvxCLPLhP9e6tjXHwuZP4vKRb3mF val: 500000000000000000 //67LUDMsMbQ2muStgPEs6L3M29kdLsoJnbCWUV4M6Ecfp7oVc
	// collator_fund: 5GRjwiFB7J44iYQrcPMiuPUBS8H9uyhbPfch3E8zpkBeRkzc val: 400000000000000000 //6AWgGe7oZuvc8542mBGwJVs1FVZoQdNH68bsaNvhzSY2xjYN

		// let collator_fund = "6AWgGe7oZuvc8542mBGwJVs1FVZoQdNH68bsaNvhzSY2xjYN".to_string();
		// let dev_incentives = "68UQrHt7FQ171AF5nvkUV2jChxhJv58tp8xXZ6spzMnvWvFN".to_string();
		// let ecosystem_development = "66fiEgBVzAGs75UEqwGyXnJBbvujb9NwbvrXbvoqWYbK6Uui".to_string();
		// let strat_part = "67grpvYywCTJ5gCNKfDMv4WMc416yUc43DMV7Xt31rUFHexc".to_string();
		// let crowdloan = "6BBGxHoTygiPF3GYvuUoZU9tivMZwdmkNFuQk3JjfDmGpbVC".to_string();
		// let earlybackers = "6BbQeXLRZv1ZHU8wyuLtjoGL5c1XDYvtqKCbVELtUVog6MAJ".to_string();
		// let team = "67xc5vHrKcCbBeP7ACDcgwfUcPSqc8pxGiyowRHPypTueU4d".to_string();
		// let community_dao = "66LaRh4sgFkCZvXEYXmaU422dD52BVEG9jAiMHBNrCAw3g6h".to_string();
		// let parachainslotreserve = "67LUDMsMbQ2muStgPEs6L3M29kdLsoJnbCWUV4M6Ecfp7oVc".to_string();

		// assert_eq!(1,0);
	}

	/// Validate that the vested fits the following criteria:
	/// - amounts per address
	pub fn vesting_address_monthly(
		vesting_timeslots: Vec<(u64, Vec<(AccountId, Balance)>)>,
		base_ecosystem_amount: Balance,
	) {
		let mut accounts: HashMap<String, u128> = HashMap::new();
		// accounts.insert("68AJcfjXieDqsBxk4mYVGhRBHcqJm7tzdxpkDKx8gPhkKbid".to_string(), 0);
		// accounts.insert("66aUED3BFQX1yKbTV2xfHStgo9qzfTPt5xoWYW83TyZWAbzf".to_string(), 0);
		// accounts.insert("6AsPuKdBFQBENh8XG831i9DZHeRRTWDVG5MPseCzZMhZwqBB".to_string(), 0);
		// accounts.insert("69jQ52EQgtqShE7Xc3uEk9BoTRcPLuMMUC5xTNcSA3NEoxNP".to_string(), 0);
		// accounts.insert("6B5xHoASi3J95oYmfTGcYgviA8Lou58PaRVmsLzM7M7JPGR8".to_string(), 0);
		// accounts.insert("671sSYej7EidfpnXnV2szkBg1VsSgp8zhmXWSiPcTarRJBtv".to_string(), 0);
		// accounts.insert("68ug2jNhBH9oE89JUsLrmCkoKoVyFQDqasSDee1UrMmcxGnf".to_string(), 0);
		// accounts.insert("69NVptNsycgjrUY5qG3uAdMg7Qh9LE4HmvMwi5FQUVpAKpXo".to_string(), 0);
		// accounts.insert("671hHAijZgBhwxWtABJhpzhC8QFbU4JzZJLyZiGxdgNAYVRD".to_string(), 0);
		// accounts.insert("67ZY3Qfd6qhsypsmsvzzmKjXTWg7MGUce99h45rCtzVf2NEg".to_string(), base_ecosystem_amount);
		let mut count = 0;
		vesting_timeslots
			.iter()
			.for_each(|(timestamp, schedules)| {
				count = count + 1;
				schedules
					.iter()
					.for_each(|(account_id, amount)| {
						let mut new_amount: u128 = *amount;
						let account_str = account_id.to_string();
						if accounts.contains_key(&account_str) {
							if let Some(old_amount) = accounts.get(&account_str) {
								new_amount = new_amount + old_amount;
							}
						}
						accounts.insert(account_str, new_amount);
					});
				println!("==================================");
				println!("month: {}", count);
				println!("timestamp: {}", timestamp);
				accounts.iter().for_each(|(key, amount)| {
					println!("key: {} val: {}", *key, amount);
				});
				println!("==================================");
			});
		let mut grand_total: u128 = 0;
		accounts.iter().for_each(|(key, amount)| {
			println!("key: {} val: {}", *key, amount);
			grand_total += *amount;
		});
		println!("{grand_total}");

		// assert_eq!(1,0);
	}
}