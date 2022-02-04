// This file is part of OAK Blockchain.

// Copyright (C) 2022 OAK Network
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

//! # Valve pallet
//!
//! When the "valve has been shut off" we filter all transactions based on the `ShutOffCallFilter`.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_support::pallet;
pub use pallet::*;

#[pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::Contains};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	/// Configuration trait of this pallet.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
		/// The pallets that we want to turn off on demand.
		type ClosedCallFilter: Contains<Self::Call>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event {
		/// The valve has been shut. This has stopped transactions from all non-critical pallets.
		ValveClosed,
		/// The chain returned to its normal operating state.
		ValveOpen,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The valve is already off.
		ValveAlreadyClosed,
		/// The valve is already open.
		ValveAlreadyOpen,
	}

	#[pallet::storage]
	#[pallet::getter(fn maintenance_mode)]
	/// Whether the valve is closed
	type ValveClosed<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Shut off the valve.
		///
		/// Weight cost is:
		/// * One DB read to ensure the valve isn't already closed.
		/// * Two DB writes - 1 for the mode and 1 for the event.
		#[pallet::weight(T::DbWeight::get().read + 2 * T::DbWeight::get().write)]
		pub fn close_valve(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			// Ensure the valve isn't already off.
			// This test is not strictly necessary, but seeing the error may help a confused chain
			// operator during an emergency.
			ensure!(!ValveClosed::<T>::get(), Error::<T>::ValveAlreadyClosed);

			ValveClosed::<T>::put(true);
			<Pallet<T>>::deposit_event(Event::ValveClosed);
			Ok(().into())
		}

		/// Return the chain to normal operating mode.
		///
		/// Weight cost is:
		/// * One DB read to ensure the valve is closed.
		/// * Two DB writes - 1 for the mode and 1 for the event.
		#[pallet::weight(T::DbWeight::get().read + 2 * T::DbWeight::get().write)]
		pub fn open_valve(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			// Ensure the valve is off.
			// This test is not strictly necessary, but seeing the error may help a confused chain
			// operator during an emergency
			ensure!(ValveClosed::<T>::get(), Error::<T>::ValveAlreadyOpen);

			ValveClosed::<T>::put(false);
			<Pallet<T>>::deposit_event(Event::ValveOpen);
			Ok(().into())
		}
	}

	#[derive(Default)]
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub start_with_valve_closed: bool,
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			if self.start_with_valve_closed {
				ValveClosed::<T>::put(true);
			}
		}
	}

	impl<T: Config> Contains<T::Call> for Pallet<T> {
		fn contains(call: &T::Call) -> bool {
			if ValveClosed::<T>::get() {
				T::ClosedCallFilter::contains(call)
			} else {
				// Could be used to filter calls we don't want people to access
				true
			}
		}
	}
}
