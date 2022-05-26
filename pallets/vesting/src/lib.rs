// This file is part of OAK Blockchain.

// Copyright (C) 2021 OAK Network
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Vesting pallet
//!
//! This pallet will mint tokens to the designated accounts at the designated times.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod migrations;
pub mod weights;
pub use weights::WeightInfo;

use parachain_staking::AdditionalIssuance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_runtime::traits::{SaturatedConversion, Saturating, Zero};
	use sp_std::vec::Vec;

	use frame_support::{pallet_prelude::*, traits::Currency};
	use frame_system::pallet_prelude::*;
	use pallet_timestamp::{self as timestamp};

	pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;
	type UnixTime = u64;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The Currency handler
		type Currency: Currency<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A vest occured.
		Vested {
			account: AccountOf<T>,
			amount: BalanceOf<T>,
		},
		VestFailed {
			account: AccountOf<T>,
			amount: BalanceOf<T>,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Block time not set.
		BlockTimeNotSet,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The closed pallet map. Each pallet in here will not receive transcations.
	#[pallet::storage]
	#[pallet::getter(fn get_scheduled_vest)]
	pub type VestingSchedule<T: Config> =
		StorageMap<_, Twox64Concat, UnixTime, Vec<(AccountOf<T>, BalanceOf<T>)>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_unvested_allocation)]
	pub type TotalUnvestedAllocation<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: T::BlockNumber) -> Weight {
			let vest_count = Self::vest();
			<T as Config>::WeightInfo::vest(vest_count)
		}

		fn on_runtime_upgrade() -> Weight {
			migrations::set_total_unvested_allocation::<T>()
		}
	}

	impl<T: Config> Pallet<T> {
		/// Based on the block time, return the time slot.
		///
		/// In order to do this we:
		/// * Get the most recent timestamp from the block.
		/// * Convert the ms unix timestamp to seconds.
		/// * Bring the timestamp down to the last whole hour.
		fn get_current_time_slot() -> Result<UnixTime, Error<T>> {
			let now = <timestamp::Pallet<T>>::get().saturated_into::<UnixTime>();
			if now == 0 {
				Err(Error::<T>::BlockTimeNotSet)?
			}
			let now = now / 1000;
			let diff_to_min = now % 3600;
			Ok(now.saturating_sub(diff_to_min))
		}

		/// Mint tokens for any accounts that have vested.
		pub fn vest() -> u32 {
			let mut num_vests: u32 = 0;
			if let Ok(current_time) = Self::get_current_time_slot() {
				if let Some(scheduled) = Self::get_scheduled_vest(current_time) {
					let mut vested_funds: BalanceOf<T> = Zero::zero();
					num_vests = scheduled.len().saturated_into::<u32>();
					for (account, amount) in scheduled {
						vested_funds = vested_funds.saturating_add(amount);
						<T as Config>::Currency::deposit_creating(&account, amount);
						Self::deposit_event(Event::Vested { account, amount })
					}
					let unvested_funds = Self::total_unvested_allocation();
					TotalUnvestedAllocation::<T>::set(unvested_funds.saturating_sub(vested_funds));
				}
				VestingSchedule::<T>::remove(current_time);
			}
			num_vests
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub vesting_schedule: Vec<(UnixTime, Vec<(AccountOf<T>, BalanceOf<T>)>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { vesting_schedule: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			let mut unvested_allocation: BalanceOf<T> = Zero::zero();
			for (time, schedule) in self.vesting_schedule.iter() {
				assert!(time % 3600 == 0, "Invalid time");
				let mut scheduled_vests: Vec<(AccountOf<T>, BalanceOf<T>)> = vec![];
				for (account, amount) in schedule {
					assert!(
						*amount > <T>::Currency::minimum_balance(),
						"Cannot vest less than the existential deposit"
					);
					scheduled_vests.push((account.clone(), amount.clone()));
					unvested_allocation = unvested_allocation.saturating_add(*amount);
				}
				VestingSchedule::<T>::insert(time, scheduled_vests);
			}
			TotalUnvestedAllocation::<T>::set(unvested_allocation);
		}
	}

	impl<T: Config> AdditionalIssuance<BalanceOf<T>> for Pallet<T> {
		fn additional_issuance() -> BalanceOf<T> {
			Self::total_unvested_allocation()
		}
	}
}
