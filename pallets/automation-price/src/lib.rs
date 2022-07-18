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

//! # Automation time pallet
//!
//! DISCLAIMER: This pallet is still in it's early stages. At this point
//! we only support scheduling two tasks per hour, and sending an on-chain
//! with a custom message.
//!
//! This pallet allows a user to schedule tasks. Tasks can scheduled for any whole hour in the future.
//! In order to run tasks this pallet consumes up to a certain amount of weight during `on_initialize`.
//!
//! The pallet supports the following tasks:
//! * On-chain events with custom text
//!

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod benchmarking;
pub mod weights;

mod exchange;
pub use exchange::*;

use core::convert::TryInto;
use cumulus_pallet_xcm::{Origin as CumulusOrigin};
use frame_support::{
	pallet_prelude::*, sp_runtime::traits::Hash, traits::StorageVersion, transactional, BoundedVec,
};
use frame_system::{pallet_prelude::*, Config as SystemConfig};
use log::info;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{Saturating},
	Perbill,
};
use sp_std::{vec, vec::Vec};
use xcm::latest::prelude::*;

pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
	pub type BalanceOf<T> = <<T as Config>::NativeTokenExchange as NativeTokenExchange<T>>::Balance;

	/// The struct that stores all information needed for a task.
	#[derive(Debug, Eq, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Task<T: Config> {
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
		asset: Vec<u8>,
		direction: u8,
		trigger_percentage: u128,
	}

	/// Needed for assert_eq to compare Tasks in tests due to BoundedVec.
	impl<T: Config> PartialEq for Task<T> {
		fn eq(&self, other: &Self) -> bool {
			self.owner_id == other.owner_id &&
			self.provided_id == other.provided_id &&
			self.asset == other.asset &&
			self.direction == other.direction &&
			self.trigger_percentage == other.trigger_percentage
		}
	}

	impl<T: Config> Task<T> {
		pub fn create_event_task(
			owner_id: AccountOf<T>,
			provided_id: Vec<u8>,
			asset: Vec<u8>,
			direction: u8,
			trigger_percentage: u128,
		) -> Task<T> {
			Task::<T> { owner_id, provided_id, asset, direction, trigger_percentage }
		}
	}

	#[derive(Debug, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct TaskHashInput<T: Config> {
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
	}

	impl<T: Config> TaskHashInput<T> {
		pub fn create_hash_input(owner_id: AccountOf<T>, provided_id: Vec<u8>) -> TaskHashInput<T> {
			TaskHashInput::<T> { owner_id, provided_id }
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The maximum number of tasks that can be scheduled for a time slot.
		#[pallet::constant]
		type MaxTasksPerSlot: Get<u32>;

		/// The maximum number of times that a task can be scheduled for.
		#[pallet::constant]
		type MaxExecutionTimes: Get<u32>;

		/// The farthest out a task can be scheduled.
		#[pallet::constant]
		type MaxScheduleSeconds: Get<u64>;

		/// The maximum weight per block.
		#[pallet::constant]
		type MaxBlockWeight: Get<Weight>;

		/// The maximum percentage of weight per block used for scheduled tasks.
		#[pallet::constant]
		type MaxWeightPercentage: Get<Perbill>;

		/// The maximum percentage of weight per block used for scheduled tasks.
		#[pallet::constant]
		type UpdateQueueRatio: Get<Perbill>;

		/// The time each block takes.
		#[pallet::constant]
		type SecondsPerBlock: Get<u64>;

		#[pallet::constant]
		type ExecutionWeightFee: Get<BalanceOf<Self>>;

		/// Handler for fees and native token transfers.
		type NativeTokenExchange: NativeTokenExchange<Self>;

		/// Utility for sending XCM messages
		type XcmSender: SendXcm;

		type Origin: From<<Self as SystemConfig>::Origin>
			+ Into<Result<CumulusOrigin, <Self as Config>::Origin>>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_scheduled_tasks)]
	pub type ScheduledTasks<T: Config> =
		StorageNMap<_, (
			NMapKey<Twox64Concat, Vec<u8>>, // asset name
			NMapKey<Twox64Concat, u8>, // direction
			NMapKey<Twox64Concat, u128>, // price
		), Vec<T::Hash>>;

	#[pallet::storage]
	#[pallet::getter(fn get_asset_target_price)]
	pub type AssetTargetPrices<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, u128>;

	#[pallet::storage]
	#[pallet::getter(fn get_asset_price)]
	pub type AssetPrices<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, u128>;

	#[pallet::storage]
	#[pallet::getter(fn get_task)]
	pub type Tasks<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Task<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_queue)]
	pub type TaskQueue<T: Config> = StorageValue<_, Vec<T::Hash>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn is_shutdown)]
	pub type Shutdown<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Time must end in a whole hour.
		InvalidTime,
		/// Duplicate task
		DuplicateTask,
		/// Non existent asset
		AssetNotSupported,
		/// Asset already supported
		AssetAlreadySupported
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Schedule task success.
		TaskScheduled {
			who: AccountOf<T>,
			task_id: T::Hash,
		},
		Notify {
			message: Vec<u8>,
		},
		TaskNotFound {
			task_id: T::Hash,
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: T::BlockNumber) -> Weight {
			if Self::is_shutdown() == true {
				return T::DbWeight::get().reads(1 as Weight)
			}

			let max_weight: Weight =
				T::MaxWeightPercentage::get().mul_floor(T::MaxBlockWeight::get());
			Self::trigger_tasks(max_weight)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedule a task to fire an event with a custom message.
		///
		/// Before the task can be scheduled the task must past validation checks.
		///
		/// # Parameters
		/// * `provided_id`: An id provided by the user. This id must be unique for the user.
		/// * `asset`: asset type
		/// * `direction`: direction of trigger movement
		/// * `trigger_percentage`: what percentage task should be triggered at 
		///
		/// # Errors
		#[pallet::weight(<T as Config>::WeightInfo::schedule_notify_task_full(1))]
		#[transactional]
		pub fn schedule_notify_task(
			origin: OriginFor<T>,
			provided_id: Vec<u8>,
			asset: Vec<u8>,
			direction: u8,
			trigger_percentage: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::validate_and_schedule_task(who, provided_id, asset, direction, trigger_percentage)?;
			Ok(().into())
		}

		/// Initialize an asset
		///
		/// Before the task can be scheduled the task must past validation checks.
		///
		/// # Parameters
		/// * `asset`: asset type
		/// * `directions`: number of directions of data input. (up, down, ?)
		///
		/// # Errors
		#[pallet::weight(<T as Config>::WeightInfo::schedule_notify_task_full(1))]
		#[transactional]
		pub fn add_asset(
			origin: OriginFor<T>,
			asset: Vec<u8>,
			target_price: u128,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			if let Some(_asset_target_price) = Self::get_asset_target_price(asset.clone()) {
				Err(Error::<T>::AssetAlreadySupported)?
			} else {
				AssetTargetPrices::<T>::insert(asset, target_price);
			}
			Ok(().into())
		}

		/// Post asset update
		///
		/// Before the task can be scheduled the task must past validation checks.
		///
		/// # Parameters
		/// * `asset`: asset type
		/// * `directions`: number of directions of data input. (up, down, ?)
		///
		/// # Errors
		#[pallet::weight(<T as Config>::WeightInfo::schedule_notify_task_full(1))]
		#[transactional]
		pub fn asset_update(
			origin: OriginFor<T>,
			asset: Vec<u8>,
			value: u128,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			if let Some(asset_target_price) = Self::get_asset_target_price(asset.clone()) {
				let direction: u8 = if asset_target_price > value { 0 } else { 1 };
				let asset_move_percentage = if direction == 0 {
					((asset_target_price - value) * 100) / asset_target_price
				} else {
					((value - asset_target_price) * 100) / asset_target_price
				};
				info!("direction: {}", direction);
				info!("asset_move_percentage: {}", asset_move_percentage);
				if let Some(asset_tasks) = Self::get_scheduled_tasks((asset.clone(), direction.clone(), asset_move_percentage.clone())) {
					// let asset_clone: Vec<Vec<Vec<T::Hash>>> = asset_tasks.clone();
				// if let Some(asset_target_price) = Self::get_asset_target_price(asset.clone()) {
					// let direction: u8 = if asset_target_price > value { 0 } else { 1 };
					// let asset_move_percentage = if direction == 0 {
					// 	((asset_target_price - value) * 100) / asset_target_price
					// } else {
					// 	((value - asset_target_price) * 100) / asset_target_price
					// };
					// // let mut taskList = asset_clone[direction as usize][asset_move_percentage as usize];
					// info!("direction: {}", direction);
					// info!("asset_move_percentage: {}", asset_move_percentage);

					let mut existing_task_queue: Vec<T::Hash> = Self::get_task_queue();
					for task in asset_tasks {
						existing_task_queue.push(task);
					}
					// let newTaskQueue: Vec<T::Hash> = existing_task_queue.append(&mut asset_tasks);
					// let newTaskQueue: Vec<T::Hash> = existing_task_queue;
					TaskQueue::<T>::put(existing_task_queue);

					<ScheduledTasks<T>>::remove((asset, direction, asset_move_percentage));
				} else {
					info!("hiiiiiii");
				};
			} else {
				Err(Error::<T>::AssetNotSupported)?
			}
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Trigger tasks for the block time.
		///
		/// Complete as many tasks as possible given the maximum weight.
		pub fn trigger_tasks(max_weight: Weight) -> Weight {
			let mut weight_left: Weight = max_weight;
			let run_task_weight = <T as Config>::WeightInfo::run_tasks_many_found(1)
				.saturating_add(T::DbWeight::get().reads(1 as Weight))
				.saturating_add(T::DbWeight::get().writes(1 as Weight));
			if weight_left < run_task_weight {
				return weight_left
			}

			// run as many scheduled tasks as we can
			let task_queue = Self::get_task_queue();
			weight_left = weight_left.saturating_sub(T::DbWeight::get().reads(1 as Weight));
			if task_queue.len() > 0 {
				let (tasks_left, new_weight_left) = Self::run_tasks(task_queue, weight_left);
				TaskQueue::<T>::put(tasks_left);
				weight_left =
					new_weight_left.saturating_sub(T::DbWeight::get().writes(1 as Weight));
			}
			weight_left
		}

		/// Update the task queue.
		///
		///
		pub fn update_task_queue(allotted_weight: Weight) -> Weight {
			allotted_weight
		}

		/// Update the task queue with scheduled tasks for the current slot
		///
		///
		pub fn update_scheduled_task_queue() -> Weight {
			100
		}

		pub fn run_notify_task(message: Vec<u8>) -> Weight {
			Self::deposit_event(Event::Notify { message });
			<T as Config>::WeightInfo::run_notify_task()
		}

		/// Runs as many tasks as the weight allows from the provided vec of task_ids.
		///
		/// Returns a vec with the tasks that were not run and the remaining weight.
		pub fn run_tasks(
			mut task_ids: Vec<T::Hash>,
			mut weight_left: Weight,
		) -> (Vec<T::Hash>, Weight) {
			let mut consumed_task_index: usize = 0;
			for task_id in task_ids.iter() {
				consumed_task_index.saturating_inc();
				let action_weight = match Self::get_task(task_id) {
					None => {
						Self::deposit_event(Event::TaskNotFound { task_id: task_id.clone() });
						<T as Config>::WeightInfo::run_tasks_many_missing(1)
					},
					Some(task) => {
						let task_action_weight = Self::run_notify_task(task.asset);
						Tasks::<T>::remove(task_id);
						task_action_weight
							.saturating_add(T::DbWeight::get().writes(1 as Weight))
							.saturating_add(T::DbWeight::get().reads(1 as Weight))
					},
				};

				weight_left = weight_left.saturating_sub(action_weight);

				if weight_left < <T as Config>::WeightInfo::run_tasks_many_found(1) {
					break
				}
			}

			if consumed_task_index == task_ids.len() {
				return (vec![], weight_left)
			} else {
				return (task_ids.split_off(consumed_task_index), weight_left)
			}
		}

		pub fn generate_task_id(owner_id: AccountOf<T>, provided_id: Vec<u8>) -> T::Hash {
			let task_hash_input =
				TaskHashInput::<T> { owner_id: owner_id.clone(), provided_id: provided_id.clone() };
			T::Hashing::hash_of(&task_hash_input)
		}

		/// Schedule task and return it's task_id.
		/// With transaction will protect against a partial success where N of M execution times might be full,
		/// rolling back any successful insertions into the schedule task table.
		pub fn schedule_task(
			owner_id: AccountOf<T>,
			provided_id: Vec<u8>,
			asset: Vec<u8>,
			direction: u8,
			trigger_percentage: u128,
		) -> Result<T::Hash, Error<T>> {
			let task_id = Self::generate_task_id(owner_id.clone(), provided_id.clone());
			if let Some(_) = Self::get_task(task_id.clone()) {
				Err(Error::<T>::DuplicateTask)?
			}
			if let Some(asset_target_price) = Self::get_asset_target_price(asset.clone()) {
				if let Some(mut asset_tasks) = Self::get_scheduled_tasks((asset.clone(), direction.clone(), trigger_percentage.clone())) {
					asset_tasks.push(task_id.clone());
					// let mut task_list = asset_clone[0][0][0];
					
					// let mut inner_task_list = task_list[0];
					// let mut task_list2 = inner_task_list.push(task_id.clone());
					// let taskList = asset_tasks[direction as usize][(trigger_percentage - 1) as usize];
					// taskList.push(task_id.clone());
	
					// std::mem::replace(&mut asset_tasks[direction as usize][(trigger_percentage - 1) as usize], taskList);
					// TODO: temp, please remove! 
					// TaskQueue::<T>::put(vec![task_id]);
	
					<ScheduledTasks<T>>::insert((asset, direction, trigger_percentage), asset_tasks);
				} else {
					<ScheduledTasks<T>>::insert((asset, direction, trigger_percentage), vec![task_id.clone()]);
				}	
			} else {
				Err(Error::<T>::AssetNotSupported)?
			}
			Ok(task_id)
		}

		/// Validate and schedule task.
		/// This will also charge the execution fee.
		pub fn validate_and_schedule_task(
			who: T::AccountId,
			provided_id: Vec<u8>,
			asset: Vec<u8>,
			direction: u8,
			trigger_percentage: u128,
		) -> Result<(), Error<T>> {
			let task_id =
				Self::schedule_task(who.clone(), provided_id.clone(), asset.clone(), direction, trigger_percentage)?;
			let task: Task<T> = Task::<T> {
				owner_id: who.clone(),
				provided_id,
				asset,
				direction,
				trigger_percentage,
			};
			<Tasks<T>>::insert(task_id, task);

			// // This should never error if can_pay_fee passed.
			// T::NativeTokenExchange::withdraw_fee(&who, fee.clone())
			// 	.map_err(|_| Error::LiquidityRestrictions)?;

			Self::deposit_event(Event::TaskScheduled { who, task_id });
			Ok(())
		}
	}
}
