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

pub mod weights;

mod fees;
pub use fees::*;

use core::convert::TryInto;
use cumulus_pallet_xcm::Origin as CumulusOrigin;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::Hash,
	traits::{Currency, ExistenceRequirement},
	transactional, BoundedVec,
};
use frame_system::{pallet_prelude::*, Config as SystemConfig};
use pallet_timestamp::{self as timestamp};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{SaturatedConversion, Saturating},
	Perbill,
};
use sp_std::{vec, vec::Vec};

pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	type UnixTime = u64;
	type AssetName = Vec<u8>;
	type AssetDirection = Direction;
	type AssetPrice = u128;
	type AssetPercentage = u128;

	#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
	pub enum Direction {
		Up,
		Down,
	}

	/// The enum that stores all action specific data.
	#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub enum Action<T: Config> {
		NativeTransfer { sender: AccountOf<T>, recipient: AccountOf<T>, amount: BalanceOf<T> },
	}

	/// The struct that stores all information needed for a task.
	#[derive(Debug, Eq, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Task<T: Config> {
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
		asset: AssetName,
		direction: AssetDirection,
		trigger_percentage: AssetPercentage,
		action: Action<T>,
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
			asset: AssetName,
			direction: AssetDirection,
			trigger_percentage: AssetPercentage,
			recipient: AccountOf<T>,
			amount: BalanceOf<T>,
		) -> Task<T> {
			let action = Action::NativeTransfer { sender: owner_id.clone(), recipient, amount };
			Task::<T> { owner_id, provided_id, asset, direction, trigger_percentage, action }
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

	#[derive(Debug, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct AssetMetadatum<T: Config> {
		upper_bound: u16,
		lower_bound: u8,
		expiration_period: UnixTime,
		asset_sudo: AccountOf<T>,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The maximum number of tasks that can be scheduled for a time slot.
		#[pallet::constant]
		type MaxTasksPerSlot: Get<u32>;

		/// The maximum weight per block.
		#[pallet::constant]
		type MaxBlockWeight: Get<u64>;

		/// The maximum percentage of weight per block used for scheduled tasks.
		#[pallet::constant]
		type MaxWeightPercentage: Get<Perbill>;

		#[pallet::constant]
		type ExecutionWeightFee: Get<BalanceOf<Self>>;

		/// The Currency type for interacting with balances
		type Currency: Currency<Self::AccountId>;

		/// Handler for fees
		type FeeHandler: HandleFees<Self>;

		type Origin: From<<Self as SystemConfig>::Origin>
			+ Into<Result<CumulusOrigin, <Self as Config>::Origin>>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_scheduled_tasks)]
	pub type ScheduledTasks<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, AssetName>,
			NMapKey<Twox64Concat, AssetDirection>,
			NMapKey<Twox64Concat, AssetPercentage>,
		),
		BoundedVec<T::Hash, T::MaxTasksPerSlot>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_scheduled_asset_period_reset)]
	pub type ScheduledAssetDeletion<T: Config> =
		StorageMap<_, Twox64Concat, UnixTime, Vec<AssetName>>;

	#[pallet::storage]
	#[pallet::getter(fn get_asset_baseline_price)]
	pub type AssetBaselinePrices<T: Config> = StorageMap<_, Twox64Concat, AssetName, AssetPrice>;

	#[pallet::storage]
	#[pallet::getter(fn get_asset_price)]
	pub type AssetPrices<T: Config> = StorageMap<_, Twox64Concat, AssetName, AssetPrice>;

	#[pallet::storage]
	#[pallet::getter(fn get_task)]
	pub type Tasks<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, AssetName>, // asset name
			NMapKey<Twox64Concat, T::Hash>,   // task ID
		),
		Task<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_queue)]
	pub type TaskQueue<T: Config> = StorageValue<_, Vec<(AssetName, T::Hash)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn is_shutdown)]
	pub type Shutdown<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_asset_metadata)]
	pub type AssetMetadata<T: Config> = StorageMap<_, Twox64Concat, AssetName, AssetMetadatum<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_number_of_assets)]
	pub type NumberOfAssets<T: Config> = StorageValue<_, u8>;

	#[pallet::error]
	pub enum Error<T> {
		/// The provided_id cannot be empty
		EmptyProvidedId,
		/// Time must end in a whole hour.
		InvalidTime,
		/// Duplicate task
		DuplicateTask,
		/// Non existent asset
		AssetNotSupported,
		/// Asset already supported
		AssetAlreadySupported,
		/// Asset cannot be updated by this account
		InvalidAssetSudo,
		/// Asset must be in triggerable range.
		AssetNotInTriggerableRange,
		/// Block Time not set
		BlockTimeNotSet,
		/// Invalid Expiration Window for new asset
		InvalidAssetExpirationWindow,
		/// Maximum tasks reached for the slot
		MaxTasksReached,
		/// Failed to insert task
		TaskInsertionFailure,
		/// Insufficient Balance
		InsufficientBalance,
		/// Restrictions on Liquidity in Account
		LiquidityRestrictions,
		/// Too Many Assets Created
		AssetLimitReached,
		/// Direction Not Supported
		DirectionNotSupported,
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
		},
		AssetCreated {
			asset: AssetName,
		},
		AssetUpdated {
			asset: AssetName,
		},
		AssetDeleted {
			asset: AssetName,
		},
		AssetPeriodReset {
			asset: AssetName,
		},
		/// Successfully transferred funds
		SuccessfullyTransferredFunds {
			task_id: T::Hash,
		},
		/// Transfer Failed
		TransferFailed {
			task_id: T::Hash,
			error: DispatchError,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: T::BlockNumber) -> Weight {
			if Self::is_shutdown() == true {
				return T::DbWeight::get().reads(1u64)
			}

			let max_weight: Weight = Weight::from_ref_time(
				T::MaxWeightPercentage::get().mul_floor(T::MaxBlockWeight::get()),
			);
			Self::trigger_tasks(max_weight)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedule a task to fire an event with a custom message.
		///
		/// Schedule a transfer task for price triggers
		///
		/// # Parameters
		/// * `provided_id`: An id provided by the user. This id must be unique for the user.
		/// * `asset`: asset type
		/// * `direction`: direction of trigger movement
		/// * `trigger_percentage`: what percentage task should be triggered at
		/// * `recipient`: person to transfer money to
		/// * `amount`: amount to transfer
		///
		/// # Errors
		#[pallet::weight(<T as Config>::WeightInfo::schedule_transfer_task_extrinsic())]
		#[transactional]
		pub fn schedule_transfer_task(
			origin: OriginFor<T>,
			provided_id: Vec<u8>,
			asset: AssetName,
			direction: AssetDirection,
			trigger_percentage: AssetPercentage,
			recipient: T::AccountId,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let fee = <BalanceOf<T>>::saturated_from(1_000_000_000_000u64);
			T::FeeHandler::can_pay_fee(&who, fee.clone())
				.map_err(|_| Error::<T>::InsufficientBalance)?;
			Self::validate_and_schedule_task(
				who.clone(),
				provided_id,
				asset,
				direction,
				trigger_percentage,
				recipient,
				amount,
			)?;
			T::FeeHandler::withdraw_fee(&who, fee.clone())
				.map_err(|_| Error::<T>::LiquidityRestrictions)?;
			Ok(().into())
		}

		/// Initialize an asset
		///
		/// Add a new asset
		///
		/// # Parameters
		/// * `asset`: asset type
		/// * `target_price`: baseline price of the asset
		/// * `upper_bound`: TBD - highest executable percentage increase for asset
		/// * `lower_bound`: TBD - highest executable percentage decrease for asset
		/// * `asset_owner`: owner of the asset
		/// * `expiration_period`: how frequently the tasks for an asset should expire
		///
		/// # Errors
		#[pallet::weight(<T as Config>::WeightInfo::add_asset_extrinsic())]
		#[transactional]
		pub fn add_asset(
			origin: OriginFor<T>,
			asset: AssetName,
			target_price: AssetPrice,
			upper_bound: u16,
			lower_bound: u8,
			asset_owner: AccountOf<T>,
			expiration_period: UnixTime,
		) -> DispatchResult {
			// TODO: needs fees if opened up to non-sudo
			ensure_root(origin)?;
			if expiration_period % 86400 != 0 {
				Err(Error::<T>::InvalidAssetExpirationWindow)?
			}
			if let Some(_asset_target_price) = Self::get_asset_baseline_price(asset.clone()) {
				Err(Error::<T>::AssetAlreadySupported)?
			}
			if let Some(number_of_assets) = Self::get_number_of_assets() {
				// TODO: remove hardcoded 2 asset limit
				if number_of_assets >= 2 {
					Err(Error::<T>::AssetLimitReached)?
				} else {
					Self::create_new_asset(
						asset,
						target_price,
						upper_bound,
						lower_bound,
						asset_owner,
						expiration_period,
						number_of_assets,
					)?;
				}
			} else {
				Self::create_new_asset(
					asset,
					target_price,
					upper_bound,
					lower_bound,
					asset_owner,
					expiration_period,
					0,
				)?;
			}
			Ok(().into())
		}

		/// Post asset update
		///
		/// Update the asset price
		///
		/// # Parameters
		/// * `asset`: asset type
		/// * `value`: value of asset
		///
		/// # Errors
		#[pallet::weight(<T as Config>::WeightInfo::asset_price_update_extrinsic())]
		#[transactional]
		pub fn asset_price_update(
			origin: OriginFor<T>,
			asset: AssetName,
			value: AssetPrice,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			if let Some(asset_metadatum) = Self::get_asset_metadata(asset.clone()) {
				let asset_sudo: AccountOf<T> = asset_metadatum.asset_sudo;
				if asset_sudo != who {
					Err(Error::<T>::InvalidAssetSudo)?
				}
			}
			let fee = <BalanceOf<T>>::saturated_from(1_000_000_000_000u64);
			T::FeeHandler::can_pay_fee(&who.clone(), fee.clone())
				.map_err(|_| Error::<T>::InsufficientBalance)?;
			if let Some(asset_target_price) = Self::get_asset_baseline_price(asset.clone()) {
				let last_asset_price: AssetPrice = match Self::get_asset_price(asset.clone()) {
					None => Err(Error::<T>::AssetNotSupported)?,
					Some(asset_price) => asset_price,
				};
				let asset_update_percentage =
					Self::calculate_asset_percentage(value, asset_target_price).saturating_add(1);
				// NOTE: this is temporarily set to 0 for ease of calculation. Ideally, we can perform
				// 			Self::calculate_asset_percentage(value, last_asset_price) and be able to compare the
				// 			last percentage to the current one. However, calculate_asset_percentage does not return
				// 			a direction. Therefore, let's say base price is 100. Last price is 95, current is 105.
				// 			Since calculate_asset_percentage returns a u128, calculate_asset_percentage will return 5%
				// 			for both, since there's no concept of positive/negative/direction (generic doesn't do just +/-).
				// 			Therefore, this function will think 5% -> 5% means no change occurred, but instead we want to
				// 			check 0% -> 5%. Therefore, we always check all the slots from 0% to x% where x is the updated
				// 			percentage. This is less efficient, but more guaranteed. In the future, we will have to return
				// 			direction for calculate_asset_percentage in the future.
				let asset_last_percentage = 0;
				if value > last_asset_price {
					Self::move_scheduled_tasks(
						asset.clone(),
						asset_last_percentage,
						asset_update_percentage,
						Direction::Up,
					)?;
				} else {
					Self::move_scheduled_tasks(
						asset.clone(),
						asset_last_percentage,
						asset_update_percentage,
						Direction::Down,
					)?;
				}
				AssetPrices::<T>::insert(asset.clone(), value);
				T::FeeHandler::withdraw_fee(&who, fee.clone())
					.map_err(|_| Error::<T>::LiquidityRestrictions)?;
				Self::deposit_event(Event::AssetUpdated { asset });
			} else {
				Err(Error::<T>::AssetNotSupported)?
			}
			Ok(().into())
		}

		/// Delete an asset
		///
		/// # Parameters
		/// * `asset`: asset type
		/// * `directions`: number of directions of data input. (up, down, ?)
		///
		/// # Errors
		#[pallet::weight(<T as Config>::WeightInfo::delete_asset_extrinsic())]
		#[transactional]
		pub fn delete_asset(origin: OriginFor<T>, asset: AssetName) -> DispatchResult {
			// TODO: needs fees if opened up to non-sudo
			ensure_root(origin)?;
			if let Some(_asset_target_price) = Self::get_asset_baseline_price(asset.clone()) {
				AssetBaselinePrices::<T>::remove(asset.clone());
				AssetPrices::<T>::remove(asset.clone());
				AssetMetadata::<T>::remove(asset.clone());
				Self::delete_asset_tasks(asset.clone());
				if let Some(number_of_assets) = Self::get_number_of_assets() {
					let new_number_of_assets = number_of_assets - 1;
					NumberOfAssets::<T>::put(new_number_of_assets);
				}
				Self::deposit_event(Event::AssetDeleted { asset });
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
			let check_time_and_deletion_weight = T::DbWeight::get().reads(2u64);
			if weight_left < check_time_and_deletion_weight {
				return weight_left
			}

			// remove assets as necessary
			let current_time_slot = match Self::get_current_time_slot() {
				Ok(time_slot) => time_slot,
				Err(_) => return weight_left,
			};
			if let Some(scheduled_deletion_assets) =
				Self::get_scheduled_asset_period_reset(current_time_slot)
			{
				// delete assets' tasks
				let asset_reset_weight = <T as Config>::WeightInfo::reset_asset(
					scheduled_deletion_assets.len().saturated_into(),
				);
				if weight_left < asset_reset_weight {
					return weight_left
				}
				// TODO: this assumes that all assets that need to be reset in a period can all be done successfully in a block.
				// 			 in the future, we need to make sure to be able to break out of for loop if out of weight and continue
				//       in the next block. Right now, we will not run out of weight - we will simply not execute anything if
				//       not all of the asset resets can be run at once. this may cause the asset reset triggers to not go off,
				//       but at least it should not brick the chain.
				for asset in scheduled_deletion_assets {
					if let Some(last_asset_price) = Self::get_asset_price(asset.clone()) {
						AssetBaselinePrices::<T>::insert(asset.clone(), last_asset_price);
						Self::delete_asset_tasks(asset.clone());
						Self::update_asset_reset(asset.clone(), current_time_slot);
						Self::deposit_event(Event::AssetPeriodReset { asset });
					};
				}
				ScheduledAssetDeletion::<T>::remove(current_time_slot);
				weight_left = weight_left - asset_reset_weight;
			}

			// run as many scheduled tasks as we can
			let task_queue = Self::get_task_queue();
			weight_left = weight_left
				.saturating_sub(T::DbWeight::get().reads(1u64))
				// For measuring the TaskQueue::<T>::put(tasks_left);
				.saturating_sub(T::DbWeight::get().writes(1u64));
			if task_queue.len() > 0 {
				let (tasks_left, new_weight_left) = Self::run_tasks(task_queue, weight_left);
				weight_left = new_weight_left;
				TaskQueue::<T>::put(tasks_left);
			}
			weight_left
		}

		pub fn update_asset_reset(asset: AssetName, current_time_slot: u64) {
			if let Some(metadata) = Self::get_asset_metadata(asset.clone()) {
				let expiration_period: u64 = metadata.expiration_period;
				// start new duration
				// 1. schedule new deletion time
				let new_time_slot = current_time_slot.saturating_add(expiration_period);
				if let Some(mut future_scheduled_deletion_assets) =
					Self::get_scheduled_asset_period_reset(new_time_slot)
				{
					future_scheduled_deletion_assets.push(asset.clone());
					<ScheduledAssetDeletion<T>>::insert(
						new_time_slot,
						future_scheduled_deletion_assets,
					);
				} else {
					let new_asset_list = vec![asset.clone()];
					<ScheduledAssetDeletion<T>>::insert(new_time_slot, new_asset_list);
				}
			};
		}

		pub fn create_new_asset(
			asset: AssetName,
			target_price: AssetPrice,
			upper_bound: u16,
			lower_bound: u8,
			asset_owner: AccountOf<T>,
			expiration_period: UnixTime,
			number_of_assets: u8,
		) -> Result<(), DispatchError> {
			AssetBaselinePrices::<T>::insert(asset.clone(), target_price);
			let asset_metadatum = AssetMetadatum::<T> {
				upper_bound,
				lower_bound,
				expiration_period,
				asset_sudo: asset_owner.clone(),
			};
			AssetMetadata::<T>::insert(asset.clone(), asset_metadatum);
			let new_time_slot = Self::get_current_time_slot()?.saturating_add(expiration_period);
			if let Some(mut future_scheduled_deletion_assets) =
				Self::get_scheduled_asset_period_reset(new_time_slot)
			{
				future_scheduled_deletion_assets.push(asset.clone());
				<ScheduledAssetDeletion<T>>::insert(
					new_time_slot,
					future_scheduled_deletion_assets,
				);
			} else {
				let new_asset_list = vec![asset.clone()];
				<ScheduledAssetDeletion<T>>::insert(new_time_slot, new_asset_list);
			}
			AssetPrices::<T>::insert(asset.clone(), target_price);
			let new_number_of_assets = number_of_assets + 1;
			NumberOfAssets::<T>::put(new_number_of_assets);
			Self::deposit_event(Event::AssetCreated { asset });
			Ok(())
		}

		pub fn get_current_time_slot() -> Result<UnixTime, Error<T>> {
			let now = <timestamp::Pallet<T>>::get().saturated_into::<UnixTime>();
			if now == 0 {
				Err(Error::<T>::BlockTimeNotSet)?
			}
			let now = now.saturating_div(1000);
			let diff_to_min = now % 60;
			Ok(now.saturating_sub(diff_to_min))
		}

		pub fn delete_asset_tasks(asset: AssetName) {
			// delete scheduled tasks
			let _ = ScheduledTasks::<T>::clear_prefix((asset.clone(),), u32::MAX, None);
			// delete tasks from tasks table
			let _ = Tasks::<T>::clear_prefix((asset.clone(),), u32::MAX, None);
			// delete tasks from task queue
			let existing_task_queue: Vec<(AssetName, T::Hash)> = Self::get_task_queue();
			let mut updated_task_queue: Vec<(AssetName, T::Hash)> = vec![];
			for task in existing_task_queue {
				if task.0 != asset {
					updated_task_queue.push(task);
				}
			}
			TaskQueue::<T>::put(updated_task_queue);
		}

		pub fn run_native_transfer_task(
			sender: T::AccountId,
			recipient: T::AccountId,
			amount: BalanceOf<T>,
			task_id: T::Hash,
		) -> Weight {
			match T::Currency::transfer(
				&sender,
				&recipient,
				amount,
				ExistenceRequirement::KeepAlive,
			) {
				Ok(_number) => Self::deposit_event(Event::SuccessfullyTransferredFunds { task_id }),
				Err(e) => Self::deposit_event(Event::TransferFailed { task_id, error: e }),
			};

			<T as Config>::WeightInfo::run_native_transfer_task()
		}

		/// Runs as many tasks as the weight allows from the provided vec of task_ids.
		///
		/// Returns a vec with the tasks that were not run and the remaining weight.
		pub fn run_tasks(
			mut task_ids: Vec<(AssetName, T::Hash)>,
			mut weight_left: Weight,
		) -> (Vec<(AssetName, T::Hash)>, Weight) {
			let mut consumed_task_index: usize = 0;
			for task_id in task_ids.iter() {
				consumed_task_index.saturating_inc();
				let action_weight = match Self::get_task(task_id) {
					None => {
						Self::deposit_event(Event::TaskNotFound { task_id: task_id.1.clone() });
						<T as Config>::WeightInfo::emit_event()
					},
					Some(task) => {
						let task_action_weight = match task.action.clone() {
							Action::NativeTransfer { sender, recipient, amount } =>
								Self::run_native_transfer_task(
									sender,
									recipient,
									amount,
									task_id.clone().1,
								),
						};
						Tasks::<T>::remove(task_id);
						task_action_weight
							.saturating_add(T::DbWeight::get().writes(1u64))
							.saturating_add(T::DbWeight::get().reads(1u64))
					},
				};

				weight_left = weight_left.saturating_sub(action_weight);

				let run_another_task_weight = <T as Config>::WeightInfo::emit_event()
					.saturating_add(T::DbWeight::get().writes(1u64))
					.saturating_add(T::DbWeight::get().reads(1u64));
				if weight_left < run_another_task_weight {
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
			asset: AssetName,
			direction: AssetDirection,
			trigger_percentage: AssetPercentage,
		) -> Result<T::Hash, Error<T>> {
			let task_id = Self::generate_task_id(owner_id.clone(), provided_id.clone());
			if let Some(_) = Self::get_task((asset.clone(), task_id.clone())) {
				Err(Error::<T>::DuplicateTask)?
			}
			if let Some(mut asset_tasks) = Self::get_scheduled_tasks((
				asset.clone(),
				direction.clone(),
				trigger_percentage.clone(),
			)) {
				if let Err(_) = asset_tasks.try_push(task_id.clone()) {
					Err(Error::<T>::MaxTasksReached)?
				}
				<ScheduledTasks<T>>::insert((asset, direction, trigger_percentage), asset_tasks);
			} else {
				let scheduled_tasks: BoundedVec<T::Hash, T::MaxTasksPerSlot> =
					vec![task_id.clone()].try_into().unwrap();
				<ScheduledTasks<T>>::insert(
					(asset, direction, trigger_percentage),
					scheduled_tasks,
				);
			}
			Ok(task_id)
		}

		/// Validate and schedule task.
		/// This will also charge the execution fee.
		pub fn validate_and_schedule_task(
			who: T::AccountId,
			provided_id: Vec<u8>,
			asset: AssetName,
			direction: AssetDirection,
			trigger_percentage: AssetPercentage,
			recipient: T::AccountId,
			amount: BalanceOf<T>,
		) -> Result<(), Error<T>> {
			if provided_id.len() == 0 {
				Err(Error::<T>::EmptyProvidedId)?
			}
			let asset_target_price: AssetPrice = match Self::get_asset_baseline_price(asset.clone())
			{
				None => Err(Error::<T>::AssetNotSupported)?,
				Some(asset_price) => asset_price,
			};
			let last_asset_price: AssetPrice = match Self::get_asset_price(asset.clone()) {
				None => Err(Error::<T>::AssetNotSupported)?,
				Some(asset_price) => asset_price,
			};
			match direction.clone() {
				Direction::Down =>
					if last_asset_price < asset_target_price {
						let last_asset_percentage =
							Self::calculate_asset_percentage(last_asset_price, asset_target_price);
						if trigger_percentage < last_asset_percentage {
							Err(Error::<T>::AssetNotInTriggerableRange)?
						}
					},
				Direction::Up =>
					if last_asset_price > asset_target_price {
						let last_asset_percentage =
							Self::calculate_asset_percentage(last_asset_price, asset_target_price);
						if trigger_percentage < last_asset_percentage {
							Err(Error::<T>::AssetNotInTriggerableRange)?
						}
					},
			}
			let task_id = Self::schedule_task(
				who.clone(),
				provided_id.clone(),
				asset.clone(),
				direction.clone(),
				trigger_percentage,
			)?;
			let action = Action::NativeTransfer { sender: who.clone(), recipient, amount };
			let task: Task<T> = Task::<T> {
				owner_id: who.clone(),
				provided_id,
				asset: asset.clone(),
				direction,
				trigger_percentage,
				action,
			};
			<Tasks<T>>::insert((asset, task_id), task);

			Self::deposit_event(Event::TaskScheduled { who, task_id });
			Ok(())
		}

		pub fn move_scheduled_tasks(
			asset: AssetName,
			lower: AssetPercentage,
			higher: AssetPercentage,
			direction: AssetDirection,
		) -> DispatchResult {
			let mut existing_task_queue: Vec<(AssetName, T::Hash)> = Self::get_task_queue();
			// TODO: fix adjusted_higher to not peg to 20. Should move with the removal of 100 % increase.
			let adjusted_higher = match higher > 20 {
				true => 20,
				false => higher,
			};
			for percentage in lower..adjusted_higher {
				// TODO: pull all and cycle through in memory
				if let Some(asset_tasks) = Self::get_scheduled_tasks((
					asset.clone(),
					direction.clone(),
					percentage.clone(),
				)) {
					for task in asset_tasks {
						existing_task_queue.push((asset.clone(), task));
					}
					<ScheduledTasks<T>>::remove((asset.clone(), direction.clone(), percentage));
				}
			}
			TaskQueue::<T>::put(existing_task_queue);
			Ok(())
		}

		pub fn calculate_asset_percentage(
			asset_update_value: AssetPrice,
			asset_target_price: AssetPrice,
		) -> AssetPercentage {
			// TODO: fix 100 hardcode
			if asset_target_price > asset_update_value {
				asset_target_price
					.saturating_sub(asset_update_value)
					.saturating_mul(100)
					.saturating_div(asset_target_price)
			} else {
				asset_update_value
					.saturating_sub(asset_target_price)
					.saturating_mul(100)
					.saturating_div(asset_target_price)
			}
		}
	}

	impl<T: Config> pallet_valve::Shutdown for Pallet<T> {
		fn is_shutdown() -> bool {
			Self::is_shutdown()
		}
		fn shutdown() {
			Shutdown::<T>::put(true);
		}
		fn restart() {
			Shutdown::<T>::put(false);
		}
	}
}
