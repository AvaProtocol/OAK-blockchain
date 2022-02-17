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
//! Close Valve -> Reject all transactions from non-critical pallets.
//! Close Pallet Gate -> Reject all transactions to the pallet.
//! Open Valve -> Resume normal chain operations. This includes allowing all non-critical pallets to receive transactions and opening all pallet gates.
//! Open Pallet Gate -> Allow the pallet to start receiving transactions again.

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
	use pallet_automation_time::{self as automation_time};
	use sp_std::vec::Vec;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_automation_time::Config {
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
		/// The pallets that we want to close on demand.
		type ClosedCallFilter: Contains<Self::Call>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event {
		/// The valve has been closed. This has stopped transactions to non-critical pallets.
		ValveClosed,
		/// The chain returned to its normal operating state.
		ValveOpen,
		/// The pallet gate has been closed. It can no longer recieve transactions.
		PalletGateClosed { pallet_name_bytes: Vec<u8> },
		/// The pallet gate has been opened. It will now start receiving transactions.
		PalletGateOpen { pallet_name_bytes: Vec<u8> },
		/// Scheduled tasks are now longer being run.
		ScheduledTasksStopped,
		/// Scheduled tasks will now start running.
		ScheduledTasksResumed,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The valve is already closed.
		ValveAlreadyClosed,
		/// The valve is already open.
		ValveAlreadyOpen,
		/// Invalid character encoding.
		InvalidCharacter,
		/// The valve pallet gate cannot be closed.
		CannotCloseGate,
		/// Scheduled tasks have already been stopped.
		ScheduledTasksAlreadyStopped,
		/// Scheduled tasks are already running.
		ScheduledTasksAlreadyRunnung,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn valve_closed)]
	/// Whether the valve is closed.
	pub type ValveClosed<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The closed pallet map. Each pallet in here will not receive transcations.
	#[pallet::storage]
	#[pallet::getter(fn paused_transactions)]
	pub type ClosedPallets<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, (), OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Close the valve.
		///
		/// This will stop all the pallets defined in `ClosedCallFilter` from receiving transactions.
		#[pallet::weight(T::DbWeight::get().read + 2 * T::DbWeight::get().write)]
		pub fn close_valve(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			// Ensure the valve isn't already closed.
			// This test is not strictly necessary, but seeing the error may help a confused chain
			// operator during an emergency.
			ensure!(!ValveClosed::<T>::get(), Error::<T>::ValveAlreadyClosed);

			ValveClosed::<T>::put(true);
			<Pallet<T>>::deposit_event(Event::ValveClosed);
			Ok(())
		}

		/// Close the pallet's gate.
		///
		/// Stop the pallet from receiving transactions.
		/// If valve is closed you cannot close a pallet.
		/// You cannot close this pallet, as then you could never open it.
		#[pallet::weight(T::DbWeight::get().read + 2 * T::DbWeight::get().write)]
		pub fn close_pallet_gate(origin: OriginFor<T>, pallet_name: Vec<u8>) -> DispatchResult {
			ensure_root(origin)?;

			// Ensure the valve isn't closed.
			// If the valve is closed there is no need to close individual pallet gates.
			ensure!(!ValveClosed::<T>::get(), Error::<T>::ValveAlreadyClosed);

			let pallet_name_string =
				sp_std::str::from_utf8(&pallet_name).map_err(|_| Error::<T>::InvalidCharacter)?;

			// Not allowed to close this pallet as then you could never open it.
			ensure!(
				pallet_name_string != <Self as PalletInfoAccess>::name(),
				Error::<T>::CannotCloseGate
			);

			ClosedPallets::<T>::mutate_exists(pallet_name.clone(), |maybe_closed| {
				if maybe_closed.is_none() {
					*maybe_closed = Some(());
					Self::deposit_event(Event::PalletGateClosed { pallet_name_bytes: pallet_name });
				}
			});

			Ok(())
		}

		/// Return the chain to normal operating mode.
		///
		/// This will open the valve and open all pallet gates.
		#[pallet::weight(T::DbWeight::get().read + 3 * T::DbWeight::get().write)]
		pub fn open_valve(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			ValveClosed::<T>::put(false);
			ClosedPallets::<T>::remove_all(None);

			<Pallet<T>>::deposit_event(Event::ValveOpen);
			Ok(())
		}

		/// Open the pallet.
		///
		/// This allows the pallet to receiving transactions.
		#[pallet::weight(T::DbWeight::get().read + 2 * T::DbWeight::get().write)]
		pub fn open_pallet_gate(origin: OriginFor<T>, pallet_name: Vec<u8>) -> DispatchResult {
			ensure_root(origin)?;

			// If the valve is closed then you cannot open a specific pallet.
			ensure!(!ValveClosed::<T>::get(), Error::<T>::ValveAlreadyClosed);

			if ClosedPallets::<T>::take(&pallet_name).is_some() {
				Self::deposit_event(Event::PalletGateOpen { pallet_name_bytes: pallet_name });
			};
			Ok(())
		}

		/// Stop all scheduled tasks from running.
		#[pallet::weight(T::DbWeight::get().read * T::DbWeight::get().write)]
		pub fn stop_scheduled_tasks(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(
				!<automation_time::Pallet<T>>::is_shutdown(),
				Error::<T>::ScheduledTasksAlreadyStopped
			);

			<automation_time::Shutdown<T>>::put(true);
			Self::deposit_event(Event::ScheduledTasksStopped);

			Ok(())
		}

		/// Allow scheduled tasks to run again.
		#[pallet::weight(T::DbWeight::get().read * T::DbWeight::get().write)]
		pub fn start_scheduled_tasks(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(
				<automation_time::Pallet<T>>::is_shutdown(),
				Error::<T>::ScheduledTasksAlreadyRunnung
			);

			<automation_time::Shutdown<T>>::put(false);
			Self::deposit_event(Event::ScheduledTasksResumed);

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
				!ClosedPallets::<T>::contains_key(pallet_name.as_bytes())
			}
		}
	}
}
