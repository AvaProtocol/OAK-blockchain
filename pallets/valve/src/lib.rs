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
//! Open Valve -> Resume normal chain operations. This includes allowing all non-critical pallets to receive transactions but not opening pallet gates.
//! Open Pallet Gate -> Allow the pallet to start receiving transactions again.
//! Open Pallet Gates -> Open the pallet gates. To ensure this call is safe it will only open five gates at once and fire an event with how many gates are still closed.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod traits;
pub use traits::*;

mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		// dispatch::{CallMetadata, GetCallMetadata},
		pallet_prelude::*,
		traits::{
			CallMetadata, Contains, GenesisBuild, GetCallMetadata, PalletInfoAccess, SortedMembers,
		},
	};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The pallets that we want to close on demand.
		type ClosedCallFilter: Contains<Self::RuntimeCall>;

		/// The AutomationTime pallet.
		type AutomationTime: Shutdown;

		/// The AutomationTime pallet.
		type AutomationPrice: Shutdown;

		/// The filter for who can call this pallet's extrinsics besides sudo.
		type CallAccessFilter: SortedMembers<<Self as frame_system::Config>::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T> {
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
		/// The number of pallet gates still closed.
		PalletGatesClosed { count: u8 },
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
		/// The user is not allowed to call the extrinsic.
		NotAllowed,
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn valve_closed)]
	/// Whether the valve is closed.
	pub type ValveClosed<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The closed pallet map. Each pallet in here will not receive transcations.
	#[pallet::storage]
	#[pallet::getter(fn paused_transactions)]
	pub type ClosedPallets<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, (), OptionQuery>;

	/// The closed pallet map. Each pallet in here will not receive transcations.
	#[pallet::storage]
	#[pallet::getter(fn count_of_closed_gates)]
	pub type ClosedPalletCount<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Close the valve.
		///
		/// This will stop all the pallets defined in `ClosedCallFilter` from receiving transactions.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::close_valve())]
		pub fn close_valve(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

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
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::close_pallet_gate_new())]
		pub fn close_pallet_gate(origin: OriginFor<T>, pallet_name: Vec<u8>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

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
					ClosedPalletCount::<T>::mutate(|count| {
						*count = count.saturating_add(1);
					});
					Self::deposit_event(Event::PalletGateClosed { pallet_name_bytes: pallet_name });
				}
			});

			Ok(())
		}

		/// Return the chain to normal operating mode.
		///
		/// This will open the valve but not any closed pallet gates.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::open_valve())]
		pub fn open_valve(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

			// Ensure the valve is closed.
			// This test is not strictly necessary, but seeing the error may help a confused chain
			// operator during an emergency.
			ensure!(ValveClosed::<T>::get(), Error::<T>::ValveAlreadyOpen);

			ValveClosed::<T>::put(false);
			<Pallet<T>>::deposit_event(Event::ValveOpen);
			Ok(())
		}

		/// Open the pallet.
		///
		/// This allows the pallet to receiving transactions.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::open_pallet_gate())]
		pub fn open_pallet_gate(origin: OriginFor<T>, pallet_name: Vec<u8>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

			// If the valve is closed then you cannot open a specific pallet.
			ensure!(!ValveClosed::<T>::get(), Error::<T>::ValveAlreadyClosed);

			if ClosedPallets::<T>::take(&pallet_name).is_some() {
				ClosedPalletCount::<T>::mutate(|count| {
					*count = count.saturating_sub(1);
				});
				Self::deposit_event(Event::PalletGateOpen { pallet_name_bytes: pallet_name });
			};
			Ok(())
		}

		/// Open the pallet gates.
		///
		/// In order to ensure this call is safe it will only open five gates at once.
		/// It will send the PalletGatesClosed with a count of how many gates are still closed.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::open_pallet_gates())]
		pub fn open_pallet_gates(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

			let _ = ClosedPallets::<T>::clear(5, None);
			let closed_pallet_count = Self::count_of_closed_gates();
			let closed_pallet_count = closed_pallet_count.saturating_sub(5);
			ClosedPalletCount::<T>::put(closed_pallet_count);
			Self::deposit_event(Event::PalletGatesClosed { count: closed_pallet_count });

			Ok(())
		}

		/// Stop all scheduled tasks from running.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::stop_scheduled_tasks())]
		pub fn stop_scheduled_tasks(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

			ensure!(!T::AutomationTime::is_shutdown(), Error::<T>::ScheduledTasksAlreadyStopped);

			T::AutomationTime::shutdown();
			Self::deposit_event(Event::ScheduledTasksStopped);

			Ok(())
		}

		/// Allow scheduled tasks to run again.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::start_scheduled_tasks())]
		pub fn start_scheduled_tasks(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

			ensure!(T::AutomationTime::is_shutdown(), Error::<T>::ScheduledTasksAlreadyRunnung);

			T::AutomationTime::restart();
			Self::deposit_event(Event::ScheduledTasksResumed);

			Ok(())
		}

		/// Stop all scheduled tasks from running.
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::stop_scheduled_tasks())]
		pub fn stop_price_automation_tasks(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

			ensure!(!T::AutomationPrice::is_shutdown(), Error::<T>::ScheduledTasksAlreadyStopped);

			T::AutomationPrice::shutdown();
			Self::deposit_event(Event::ScheduledTasksStopped);

			Ok(())
		}

		/// Allow scheduled tasks to run again.
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::start_scheduled_tasks())]
		pub fn start_price_automation_tasks(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_allowed(origin)?;

			ensure!(T::AutomationPrice::is_shutdown(), Error::<T>::ScheduledTasksAlreadyRunnung);

			T::AutomationPrice::restart();
			Self::deposit_event(Event::ScheduledTasksResumed);

			Ok(())
		}
	}

	// #[derive(Default)]
	// #[pallet::genesis_config]
	// pub struct GenesisConfig {
	// 	pub start_with_valve_closed: bool,
	// 	pub closed_gates: Vec<Vec<u8>>,
	// }

	// #[pallet::genesis_build]
	// impl<T: Config> GenesisBuild<T> for GenesisConfig {
	// 	fn build(&self) {
	// 		if self.start_with_valve_closed {
	// 			ValveClosed::<T>::put(true);
	// 		}

	// 		let mut closed_pallet_count = ClosedPalletCount::<T>::get();
	// 		for gate in self.closed_gates.iter() {
	// 			ClosedPallets::<T>::insert(gate, ());
	// 			closed_pallet_count = closed_pallet_count.saturating_add(1);
	// 		}
	// 		ClosedPalletCount::<T>::put(closed_pallet_count);
	// 	}
	// }

	impl<T: Config> Pallet<T> {
		/// Sudo or a member of the CallAccessFilter can call.
		pub fn ensure_allowed(origin: OriginFor<T>) -> DispatchResult {
			if ensure_root(origin.clone()).is_err() {
				let who = ensure_signed(origin)?;
				if !T::CallAccessFilter::contains(&who) {
					Err(Error::<T>::NotAllowed)?;
				}
			}
			Ok(())
		}
	}

	impl<T: Config> Contains<T::RuntimeCall> for Pallet<T>
	where
		<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
	{
		fn contains(call: &T::RuntimeCall) -> bool {
			if ValveClosed::<T>::get() {
				T::ClosedCallFilter::contains(call)
			} else {
				let CallMetadata { function_name: _, pallet_name } = call.get_call_metadata();
				!ClosedPallets::<T>::contains_key(pallet_name.as_bytes())
			}
		}
	}
}
