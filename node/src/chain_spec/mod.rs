use serde::{Deserialize, Serialize};

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainSpec;
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify, Zero};
use std::collections::HashMap;

use primitives::{AccountId, AuraId, Balance, Signature};

#[cfg(feature = "neumann-node")]
pub mod neumann;
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
	vest_ending: u64
) {
	let mut total_vested: Balance = Zero::zero();
	let unique_vested_slots = vesting_timeslots
		.iter()
		.map(|(timestamp, schedules)| {
			assert!(
				timestamp <= &vest_ending,
				"greater than largest expected timestamp."
			);
			assert!(
				timestamp >= &vest_starting,
				"smaller than smallest expected timestamp."
			);
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
	let mut month_total: Balance = base_amount;
	vesting_timeslots
		.iter()
		.for_each(|(timestamp, schedules)| {
			schedules
				.iter()
				.for_each(|(_, amount)| {
					month_total = month_total
						.checked_add(*amount)
						.expect("shouldn't overflow when building genesis");
				});
			match timestamp {
				1651777200 => {
					assert_eq!(
						month_total, 891111111111111109
					);
				},
				1654455600 => {
					assert_eq!(
						month_total, 1202222222222222218
					);
				},
				1657047600 => {
					assert_eq!(
						month_total, 1313333333333333327
					);
				},
				1659726000 => {
					assert_eq!(
						month_total, 1797777777777777769
					);
				},
				1662404400 => {
					assert_eq!(
						month_total, 1908888888888888878
					);
				},
				1664996400 => {
					assert_eq!(
						month_total, 2986666666666666653
					);
				},
				1667674800 => {
					assert_eq!(
						month_total, 3097777777777777762
					);
				},
				1670270400 => {
					assert_eq!(
						month_total, 3582222222222222204
					);
				},
				1672948800 => {
					assert_eq!(
						month_total, 3693333333333333313
					);
				},
				1675627200 => {
					assert_eq!(
						month_total, 3804444444444444422
					);
				},
				1678046400 => {
					assert_eq!(
						month_total, 3915555555555555531
					);
				},
				1680721200 => {
					assert_eq!(
						month_total, 5116666666666666639
					);
				},
				1683313200 => {
					assert_eq!(
						month_total, 5227777777777777748
					);
				},
				1685991600 => {
					assert_eq!(
						month_total, 5338888888888888857
					);
				},
				1688583600 => {
					assert_eq!(
						month_total, 5449999999999999966
					);
				},
				1691262000 => {
					assert_eq!(
						month_total, 5561111111111111075
					);
				},
				1693940400 => {
					assert_eq!(
						month_total, 5672222222222222184
					);
				},
				1696532400 => {
					assert_eq!(
						month_total, 6749999999999999959
					);
				},
				1699214400 => {
					assert_eq!(
						month_total, 6861111111111111068
					);
				},
				1701806400 => {
					assert_eq!(
						month_total, 6972222222222222177
					);
				},
				1704484800 => {
					assert_eq!(
						month_total, 7083333333333333286
					);
				},
				1707163200 => {
					assert_eq!(
						month_total, 7194444444444444395
					);
				},
				1709668800 => {
					assert_eq!(
						month_total, 7305555555555555504
					);
				},
				1712343600 => {
					assert_eq!(
						month_total, 8133333333333333279
					);
				},
				1714935600 => {
					assert_eq!(
						month_total, 8244444444444444388
					);
				},
				1717614000 => {
					assert_eq!(
						month_total, 8355555555555555497
					);
				},
				1720206000 => {
					assert_eq!(
						month_total, 8466666666666666606
					);
				},
				1722884400 => {
					assert_eq!(
						month_total, 8577777777777777715
					);
				},
				1725562800 => {
					assert_eq!(
						month_total, 8688888888888888824
					);
				},
				1728154800 => {
					assert_eq!(
						month_total, 9066666666666666599
					);
				},
				1730836800 => {
					assert_eq!(
						month_total, 9177777777777777708
					);
				},
				1733428800 => {
					assert_eq!(
						month_total, 9288888888888888817
					);
				},
				1736107200 => {
					assert_eq!(
						month_total, 9399999999999999926
					);
				},
				1738785600 => {
					assert_eq!(
						month_total, 9511111111111111035
					);
				},
				1741204800 => {
					assert_eq!(
						month_total, 9622222222222222144
					);
				},
				1743879600 => {
					assert_eq!(
						month_total, 9999999999999999919
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
	base_collator_amount: Balance,
	base_crowdloan_amount: Balance,
) {
	let mut accounts: HashMap<String, u128> = HashMap::new();
	accounts.insert("5DFXtRzj8nAEVvFWESwsvvxCLPLhP9e6tjXHwuZP4vKRb3mF".to_string(), 0);
	accounts.insert("5DsfkzRDrzL3n7jw1QJQHqGeo2AC7VAGaFzdQGVgp87X7YAg".to_string(), 0);
	accounts.insert("5CamukJsXYQKhYq4h9Mm8fuMnZd66ViFuTsM4n28LrEvZdje".to_string(), 0);
	accounts.insert("5EPUXN1Unn8Zbdbue8qG5vLNtbQfRRUD7fyM1x67pfSXzDac".to_string(), 0);
	accounts.insert("5DbvVzgMUaakg9ZCAsJ9Wx7XngiTUpwNLkNJaP6KrA7rkjDd".to_string(), 0);
	accounts.insert("5GRjwiFB7J44iYQrcPMiuPUBS8H9uyhbPfch3E8zpkBeRkzc".to_string(), base_collator_amount);
	accounts.insert("5CFe6mCFDdsfAPt4PjrN4wdCoqnNgqZaTGBXp8PfgVpYWoPJ".to_string(), 0);
	accounts.insert("5H6LdMvqX4qqqWdNn7ZbAMm4uZ4vSz74fnvECtX2VXQtHfwb".to_string(), base_crowdloan_amount);
	accounts.insert("5HWUKbTo7J91swVmq7RgLgsWGEisiuGD8rDQx5ZBJoTHZJyP".to_string(), 0);
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

	assert_eq!(1,0);
}
