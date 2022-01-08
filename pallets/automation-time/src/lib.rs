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

//! # The automation time pallet!
//! 
//! This pallet allows a user to schedule tasks. We currently support the following tasks.
//! 
//! * On-chain events with custom text
//! 

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;
use pallet_timestamp::{self as timestamp};
use sp_runtime:: {
	traits::{SaturatedConversion},
};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		/// Time must end in a whole minute.
		InvalidTime,
		/// Time must be in the future.
		PastTime,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		
		/// Get the relevant time slot.
		/// 
		/// In order to do this we get the most recent timestamp from the chain. Then convert
		/// the ms unix timestamp to seconds. Lastly, we bring the timestamp down to the last whole minute.
		fn get_time_slot() -> u64 {
			let now = <timestamp::Pallet<T>>::get().saturated_into::<u64>();
			let now = now / 1000;
			let diff_to_min = now % 60;
			now - diff_to_min
		}

		/// Checks to see if the scheduled time is a valid timestamp.
		/// 
		/// In order for a time to be valid it must end in a whole minute and be in the future.
		fn is_valid_time(scheduled_time: u64) -> Result<(), Error<T>> {
			let remainder = scheduled_time % 60;
			if remainder != 0 {
				Err(<Error<T>>::InvalidTime)?;
			}
		
			let now = Self::get_time_slot();
			if scheduled_time <= now  {
				Err(<Error<T>>::PastTime)?;
			}

			Ok(())
		}
	}
}
