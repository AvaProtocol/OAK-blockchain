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
	use frame_support::{
		dispatch::{CallMetadata, GetCallMetadata},
		pallet_prelude::*,
		traits::{Contains, PalletInfoAccess},
	};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;

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
		/// All the pallet's actions stoped.
		PalletTapped { pallet_name_bytes: Vec<u8> },
		/// All the pallet's actions resumed.
		PalletUntapped { pallet_name_bytes: Vec<u8> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The valve is already off.
		ValveAlreadyClosed,
		/// The valve is already open.
		ValveAlreadyOpen,
		/// Invalid character encoding.
		InvalidCharacter,
		/// The valve pallet cannot be paused.
		CannotPause,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn valve_closed)]
	/// Whether the valve is closed.
	pub type ValveClosed<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The tapped pallet map. Each pallet in here will not receive transcations or process tasks.
	#[pallet::storage]
	#[pallet::getter(fn paused_transactions)]
	pub type TappedPallets<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, (), OptionQuery>;

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
			Ok(())
		}

		/// Tap the valve.
		///
		/// If valve is closed you cannot tap a pallet.
		/// You cannot tap this paller, as then you could never untap it.
		#[pallet::weight(T::DbWeight::get().read + 2 * T::DbWeight::get().write)]
		pub fn tighten_valve(origin: OriginFor<T>, pallet_name: Vec<u8>) -> DispatchResult {
			ensure_root(origin)?;

			// Ensure the valve isn't already off.
			// If the valve is already off there is no need to tap individual pallets.
			ensure!(!ValveClosed::<T>::get(), Error::<T>::ValveAlreadyClosed);

			let pallet_name_string =
				sp_std::str::from_utf8(&pallet_name).map_err(|_| Error::<T>::InvalidCharacter)?;

			// Not allowed to pause calls of this pallet as then you could never unpause them.
			ensure!(
				pallet_name_string != <Self as PalletInfoAccess>::name(),
				Error::<T>::CannotPause
			);

			TappedPallets::<T>::mutate_exists(pallet_name.clone(), |maybe_tapped| {
				if maybe_tapped.is_none() {
					*maybe_tapped = Some(());
					Self::deposit_event(Event::PalletTapped { pallet_name_bytes: pallet_name });
				}
			});

			Ok(())
		}

		/// Return the chain to normal operating mode.
		///
		/// Weight cost is:
		/// * One DB read to ensure the valve is closed.
		/// * Three DB writes - 1 for the mode and 1 for the event.
		#[pallet::weight(T::DbWeight::get().read + 3 * T::DbWeight::get().write)]
		pub fn open_valve(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			ValveClosed::<T>::put(false);
			TappedPallets::<T>::remove_all(None);

			<Pallet<T>>::deposit_event(Event::ValveOpen);
			Ok(())
		}

		/// Untap the pallet.
		#[pallet::weight(T::DbWeight::get().read + 2 * T::DbWeight::get().write)]
		pub fn loosen_valve(origin: OriginFor<T>, pallet_name: Vec<u8>) -> DispatchResult {
			ensure_root(origin)?;

			// If the valve is off then you cannot lossen it.
			ensure!(!ValveClosed::<T>::get(), Error::<T>::ValveAlreadyClosed);

			if TappedPallets::<T>::take(&pallet_name).is_some() {
				Self::deposit_event(Event::PalletUntapped { pallet_name_bytes: pallet_name });
			};
			Ok(())
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

	impl<T: Config> Contains<T::Call> for Pallet<T>
	where
		<T as frame_system::Config>::Call: GetCallMetadata,
	{
		fn contains(call: &T::Call) -> bool {
			if ValveClosed::<T>::get() {
				T::ClosedCallFilter::contains(call)
			} else {
				let CallMetadata { function_name: _, pallet_name } = call.get_call_metadata();
				!TappedPallets::<T>::contains_key(pallet_name.as_bytes())
			}
		}
	}
}
