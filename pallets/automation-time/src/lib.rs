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
//! TODO: Finish documentation (ENG-148).
//!
//! NOTES: None of the weights are accurate yet.
//!

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{inherent::Vec, pallet_prelude::*, sp_runtime::traits::Hash, BoundedVec};
use frame_system::pallet_prelude::*;
use pallet_timestamp::{self as timestamp};
use scale_info::TypeInfo;
use sp_runtime::traits::SaturatedConversion;
use sp_std::vec;
use core::convert::TryInto;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	type AccountOf<T> = <T as frame_system::Config>::AccountId;

	#[derive(Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub enum Action {
		Notify(Vec<u8>),
	}

	#[derive(Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Task<T: Config> {
		owner_id: AccountOf<T>,
		time: u64,
		action: Action,
	}

	impl<T: Config> Task<T> {
		pub fn create_event_task(owner_id: AccountOf<T>, time: u64, message: Vec<u8>) -> Task<T> {
			let action = Action::Notify(message);
			Task::<T> { owner_id, time, action }
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The maximum number of tasks that can be scheduled for a time slot.
		#[pallet::constant]
		type MaxTasksPerSlot: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_scheduled_tasks)]
	pub type ScheduledTasks<T: Config> =
		StorageMap<_, Twox64Concat, u64, BoundedVec<T::Hash, T::MaxTasksPerSlot>>;

	#[pallet::storage]
	#[pallet::getter(fn get_task)]
	pub type Tasks<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Task<T>>;

	#[pallet::error]
	pub enum Error<T> {
		/// Time must end in a whole minute.
		InvalidTime,
		/// Time must be in the future.
		PastTime,
		/// The message cannot be empty.
		EmptyMessage,
		/// There can be no duplicate tasks.
		DuplicateTask,
		/// Time slot is full. No more tasks can be scheduled for this time.
		TimeSlotFull,
		/// You are not the owner of the task.
		NotTaskOwner,
		/// The task does not exist.
		TaskDoesNotExist,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Schedule task success. [task owner, task_id]
		TaskScheduled(T::AccountId, T::Hash),
		// Cancelled a task. [task owner, task_id]
		TaskCancelled(T::AccountId, T::Hash),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedule a task to fire an event with a custom message.
		///
		/// Before the task can be scheduled the task must past validation checks.
		/// * The transaction is signed
		/// * The time is valid
		/// * The message's length > 0
		///
		/// # Parameters
		/// * `time`: The unix standard time in seconds for when the task should run.
		/// * `message`: The message you want the event to have.
		///
		/// # Errors
		/// * `InvalidTime`: Time must end in a whole minute.
		/// * `PastTime`: Time must be in the future.
		/// * `EmptyMessage`: The message cannot be empty.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(2) + T::DbWeight::get().reads(2))]
		pub fn schedule_notify_task(
			origin: OriginFor<T>,
			time: u64,
			message: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::is_valid_time(time)?;
			if message.len() == 0 {
				Err(Error::<T>::EmptyMessage)?
			}

			let task = Task::<T>::create_event_task(who.clone(), time, message);
			let task_id = T::Hashing::hash_of(&task);

			if let Some(_) = Self::get_task(task_id) {
				Err(Error::<T>::DuplicateTask)?
			}

			match Self::get_scheduled_tasks(time) {
				None => {
					let task_ids: BoundedVec<T::Hash, T::MaxTasksPerSlot> =
						vec![task_id].try_into().unwrap();
					<ScheduledTasks<T>>::insert(time, task_ids);
				},
				Some(mut task_ids) => {
					if let Err(_) = task_ids.try_push(task_id) {
						Err(Error::<T>::TimeSlotFull)?
					}
					<ScheduledTasks<T>>::insert(time, task_ids);
				},
			}

			<Tasks<T>>::insert(task_id, task);
			Self::deposit_event(Event::TaskScheduled(who, task_id));
			Ok(().into())
		}

		/// Cancel a task.
		///
		/// Tasks can only can be cancelled by their owners.
		///
		/// TODO: Allow sudo to cancel tasks (ENG-156).
		///
		/// # Parameters
		/// * `task_id`: The id of the task.
		///
		/// # Errors
		/// * `NotTaskOwner`: You are not the owner of the task.
		/// * `TaskDoesNotExist`: The task does not exist.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(2) + T::DbWeight::get().reads(2))]
		pub fn cancel_task(origin: OriginFor<T>, task_id: T::Hash) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			match Self::get_task(task_id) {
				None => Err(Error::<T>::TaskDoesNotExist)?,
				Some(task) => {
					if who != task.owner_id {
						Err(Error::<T>::NotTaskOwner)?
					}
					match Self::get_scheduled_tasks(task.time) {
						None => {}, //TODO add some sort of error reporter here (ENG-155).
						Some(mut task_ids) =>
							for i in 0..task_ids.len() {
								if task_ids[i] == task_id {
									if task_ids.len() == 1 {
										<ScheduledTasks<T>>::remove(task.time);
									} else {
										task_ids.remove(i);
										<ScheduledTasks<T>>::insert(task.time, task_ids);
									}
									break
								}
							},
					}

					<Tasks<T>>::remove(task_id);
				},
			}

			Self::deposit_event(Event::TaskCancelled(who, task_id));
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Get the relevant time slot.
		///
		/// In order to do this we get the most recent timestamp from the chain. Then convert
		/// the ms unix timestamp to seconds. Lastly, we bring the timestamp down to the last whole minute.
		fn get_current_time_slot() -> u64 {
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

			let now = Self::get_current_time_slot();
			if scheduled_time <= now {
				Err(<Error<T>>::PastTime)?;
			}

			Ok(())
		}
	}
}
