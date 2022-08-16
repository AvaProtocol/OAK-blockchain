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
pub mod migrations;
pub mod weights;

mod fees;
pub use fees::*;

mod autocompounding;
pub use autocompounding::*;

use core::convert::TryInto;
use cumulus_pallet_xcm::Origin as CumulusOrigin;
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::DispatchErrorWithPostInfo,
	pallet_prelude::*,
	sp_runtime::traits::Hash,
	storage::{
		with_transaction,
		TransactionOutcome::{Commit, Rollback},
	},
	traits::{Currency, ExistenceRequirement, StorageVersion},
	BoundedVec,
};
use frame_system::{pallet_prelude::*, Config as SystemConfig};
use log::info;
use pallet_automation_time_rpc_runtime_api::AutomationAction;
use pallet_parachain_staking::DelegatorActions;
use pallet_timestamp::{self as timestamp};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{CheckedConversion, SaturatedConversion, Saturating},
	ArithmeticError, DispatchError, Perbill,
};
use sp_std::{vec, vec::Vec};
pub use weights::WeightInfo;
use xcm::latest::prelude::*;

// NOTE: this is the current storage version for the code.
// On migration, you will need to increment this.
const CURRENT_CODE_STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	type UnixTime = u64;
	type Seconds = u64;

	/// The enum that stores all action specific data.
	#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub enum Action<T: Config> {
		Notify {
			message: Vec<u8>,
		},
		NativeTransfer {
			sender: AccountOf<T>,
			recipient: AccountOf<T>,
			amount: BalanceOf<T>,
		},
		XCMP {
			para_id: ParaId,
			call: Vec<u8>,
			weight_at_most: Weight,
		},
		AutoCompoundDelegatedStake {
			delegator: AccountOf<T>,
			collator: AccountOf<T>,
			account_minimum: BalanceOf<T>,
			frequency: Seconds,
		},
	}

	impl<T: Config> From<AutomationAction> for Action<T> {
		fn from(a: AutomationAction) -> Self {
			let default_account =
				T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
					.expect("always valid");
			match a {
				AutomationAction::Notify => Action::Notify { message: "default".into() },
				AutomationAction::NativeTransfer => Action::NativeTransfer {
					sender: default_account.clone(),
					recipient: default_account,
					amount: 0u32.into(),
				},
				AutomationAction::XCMP => Action::XCMP {
					para_id: ParaId::from(2114 as u32),
					call: vec![0],
					weight_at_most: 0,
				},
				AutomationAction::AutoCompoundDelegatedStake =>
					Action::AutoCompoundDelegatedStake {
						delegator: default_account.clone(),
						collator: default_account,
						account_minimum: 0u32.into(),
						frequency: 0,
					},
			}
		}
	}

	/// The struct that stores data for a missed task.
	#[derive(Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct MissedTask<T: Config> {
		task_id: T::Hash,
		execution_time: UnixTime,
	}

	impl<T: Config> MissedTask<T> {
		pub fn create_missed_task(task_id: T::Hash, execution_time: UnixTime) -> MissedTask<T> {
			MissedTask::<T> { task_id, execution_time }
		}
	}

	/// The struct that stores all information needed for a task.
	#[derive(Debug, Eq, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Task<T: Config> {
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
		pub execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
		executions_left: u32,
		action: Action<T>,
	}

	/// Needed for assert_eq to compare Tasks in tests due to BoundedVec.
	impl<T: Config> PartialEq for Task<T> {
		fn eq(&self, other: &Self) -> bool {
			self.owner_id == other.owner_id &&
				self.provided_id == other.provided_id &&
				self.action == other.action &&
				self.executions_left == other.executions_left &&
				self.execution_times.len() == other.execution_times.len() &&
				self.execution_times.capacity() == other.execution_times.capacity() &&
				self.execution_times.to_vec() == other.execution_times.to_vec()
		}
	}

	impl<T: Config> Task<T> {
		pub fn create_task(
			owner_id: AccountOf<T>,
			provided_id: Vec<u8>,
			execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
			action: Action<T>,
		) -> Task<T> {
			let executions_left: u32 = execution_times.len().try_into().unwrap();
			Task::<T> { owner_id, provided_id, execution_times, executions_left, action }
		}

		pub fn create_event_task(
			owner_id: AccountOf<T>,
			provided_id: Vec<u8>,
			execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
			message: Vec<u8>,
		) -> Task<T> {
			let action = Action::Notify { message };
			Self::create_task(owner_id, provided_id, execution_times, action)
		}

		pub fn create_native_transfer_task(
			owner_id: AccountOf<T>,
			provided_id: Vec<u8>,
			execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
			recipient_id: AccountOf<T>,
			amount: BalanceOf<T>,
		) -> Task<T> {
			let action = Action::NativeTransfer {
				sender: owner_id.clone(),
				recipient: recipient_id,
				amount,
			};
			Self::create_task(owner_id, provided_id, execution_times, action)
		}

		pub fn create_xcmp_task(
			owner_id: AccountOf<T>,
			provided_id: Vec<u8>,
			execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
			para_id: ParaId,
			call: Vec<u8>,
			weight_at_most: Weight,
		) -> Task<T> {
			let action = Action::XCMP { para_id, call, weight_at_most };
			Self::create_task(owner_id, provided_id, execution_times, action)
		}

		pub fn create_auto_compound_delegated_stake_task(
			owner_id: AccountOf<T>,
			provided_id: Vec<u8>,
			execution_time: UnixTime,
			frequency: Seconds,
			collator_id: AccountOf<T>,
			account_minimum: BalanceOf<T>,
		) -> Task<T> {
			let action = Action::AutoCompoundDelegatedStake {
				delegator: owner_id.clone(),
				collator: collator_id,
				account_minimum,
				frequency,
			};
			Self::create_task(
				owner_id,
				provided_id,
				vec![execution_time].try_into().unwrap(),
				action,
			)
		}

		pub fn get_executions_left(&self) -> u32 {
			self.executions_left
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

		/// The Currency type for interacting with balances
		type Currency: Currency<Self::AccountId>;

		/// The currencyIds that our chain supports.
		type CurrencyId: Parameter
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ Ord
			+ TypeInfo
			+ MaxEncodedLen;

		/// Handler for fees
		type FeeHandler: HandleFees<Self>;

		/// Utility for sending XCM messages
		type XcmSender: SendXcm;

		type Origin: From<<Self as SystemConfig>::Origin>
			+ Into<Result<CumulusOrigin, <Self as Config>::Origin>>;

		type DelegatorActions: DelegatorActions<Self::AccountId, BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_scheduled_tasks)]
	pub type ScheduledTasks<T: Config> =
		StorageMap<_, Twox64Concat, u64, BoundedVec<T::Hash, T::MaxTasksPerSlot>>;

	#[pallet::storage]
	#[pallet::getter(fn get_task)]
	pub type Tasks<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Task<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_queue)]
	pub type TaskQueue<T: Config> = StorageValue<_, Vec<T::Hash>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_missed_queue)]
	pub type MissedQueue<T: Config> = StorageValue<_, Vec<MissedTask<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_last_slot)]
	// NOTE: The 2 UnixTime stamps represent (last_time_slot, last_missed_slot).
	// `last_time_slot` represents the last time slot that the task queue was updated.
	// `last_missed_slot` represents the last scheduled slot where the missed queue has checked for missed tasks.
	pub type LastTimeSlot<T: Config> = StorageValue<_, (UnixTime, UnixTime)>;

	#[pallet::storage]
	#[pallet::getter(fn is_shutdown)]
	pub type Shutdown<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Time must end in a whole hour.
		InvalidTime,
		/// Time must be in the future.
		PastTime,
		/// Time cannot be too far in the future.
		TimeTooFarOut,
		/// The message cannot be empty.
		EmptyMessage,
		/// The provided_id cannot be empty
		EmptyProvidedId,
		/// There can be no duplicate tasks.
		DuplicateTask,
		/// Time slot is full. No more tasks can be scheduled for this time.
		TimeSlotFull,
		/// You are not the owner of the task.
		NotTaskOwner,
		/// The task does not exist.
		TaskDoesNotExist,
		/// Block time not set.
		BlockTimeNotSet,
		/// Amount has to be larger than 0.1 OAK.
		InvalidAmount,
		/// Sender cannot transfer money to self.
		TransferToSelf,
		/// Insufficient balance to pay execution fee.
		InsufficientBalance,
		/// Account liquidity restrictions prevent withdrawal.
		LiquidityRestrictions,
		/// Too many execution times provided.
		TooManyExecutionsTimes,
		/// ParaId provided does not match origin paraId.
		ParaIdMismatch,
		/// Task is currently not supported.
		TaskNotSupported,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Schedule task success.
		TaskScheduled {
			who: AccountOf<T>,
			task_id: T::Hash,
		},
		// Cancelled a task.
		TaskCancelled {
			who: AccountOf<T>,
			task_id: T::Hash,
		},
		/// Notify event for the task.
		Notify {
			message: Vec<u8>,
		},
		/// A Task was not found.
		TaskNotFound {
			task_id: T::Hash,
		},
		/// Successfully transferred funds
		SuccessfullyTransferredFunds {
			task_id: T::Hash,
		},
		/// Successfully sent XCMP
		SuccessfullySentXCMP {
			task_id: T::Hash,
			para_id: ParaId,
		},
		/// Failed to send XCMP
		FailedToSendXCMP {
			task_id: T::Hash,
			para_id: ParaId,
			error: SendError,
		},
		/// Transfer Failed
		TransferFailed {
			task_id: T::Hash,
			error: DispatchError,
		},
		SuccesfullyAutoCompoundedDelegatorStake {
			task_id: T::Hash,
			amount: BalanceOf<T>,
		},
		AutoCompoundDelegatorStakeFailed {
			task_id: T::Hash,
			error_message: Vec<u8>,
			error: DispatchErrorWithPostInfo,
		},
		/// The task could not be run at the scheduled time.
		TaskMissed {
			who: T::AccountId,
			task_id: T::Hash,
			execution_time: UnixTime,
		},
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

		fn on_runtime_upgrade() -> Weight {
			let on_chain_storage_version = StorageVersion::get::<Pallet<T>>();
			info!("on chain storage version, {:?}", on_chain_storage_version);
			if on_chain_storage_version < CURRENT_CODE_STORAGE_VERSION {
				migrations::v2::migrate::<T>()
			} else {
				info!("migration already run before");
				0
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedule a task to fire an event with a custom message.
		///
		/// Before the task can be scheduled the task must past validation checks.
		/// * The transaction is signed
		/// * The provided_id's length > 0
		/// * The message's length > 0
		/// * The times are valid
		///
		/// # Parameters
		/// * `provided_id`: An id provided by the user. This id must be unique for the user.
		/// * `execution_times`: The list of unix standard times in seconds for when the task should run.
		/// * `message`: The message you want the event to have.
		///
		/// # Errors
		/// * `InvalidTime`: Time must end in a whole hour.
		/// * `PastTime`: Time must be in the future.
		/// * `EmptyMessage`: The message cannot be empty.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		#[pallet::weight(<T as Config>::WeightInfo::schedule_notify_task_full(execution_times.len().try_into().unwrap()))]
		pub fn schedule_notify_task(
			origin: OriginFor<T>,
			provided_id: Vec<u8>,
			execution_times: Vec<UnixTime>,
			message: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			if message.len() == 0 {
				Err(Error::<T>::EmptyMessage)?
			}

			Self::validate_and_schedule_task(
				Action::Notify { message },
				who,
				provided_id,
				execution_times,
			)?;
			Ok(().into())
		}

		/// Schedule a task to transfer native token balance from sender to recipient.
		///
		/// Before the task can be scheduled the task must past validation checks.
		/// * The transaction is signed
		/// * The provided_id's length > 0
		/// * The times are valid
		/// * Larger transfer amount than the acceptable minimum
		/// * Transfer to account other than to self
		///
		/// # Parameters
		/// * `provided_id`: An id provided by the user. This id must be unique for the user.
		/// * `execution_times`: The list of unix standard times in seconds for when the task should run.
		/// * `recipient_id`: Account ID of the recipient.
		/// * `amount`: Amount of balance to transfer.
		///
		/// # Errors
		/// * `InvalidTime`: Time must end in a whole hour.
		/// * `PastTime`: Time must be in the future.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		/// * `InvalidAmount`: Amount has to be larger than 0.1 OAK.
		/// * `TransferToSelf`: Sender cannot transfer money to self.
		/// * `TransferFailed`: Transfer failed for unknown reason.
		#[pallet::weight(<T as Config>::WeightInfo::schedule_native_transfer_task_full(execution_times.len().try_into().unwrap()))]
		pub fn schedule_native_transfer_task(
			origin: OriginFor<T>,
			provided_id: Vec<u8>,
			execution_times: Vec<UnixTime>,
			recipient_id: T::AccountId,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// check for greater than existential deposit
			if amount < T::Currency::minimum_balance() {
				Err(<Error<T>>::InvalidAmount)?
			}
			// check not sent to self
			if who == recipient_id {
				Err(<Error<T>>::TransferToSelf)?
			}
			let action =
				Action::NativeTransfer { sender: who.clone(), recipient: recipient_id, amount };
			Self::validate_and_schedule_task(action, who, provided_id, execution_times)?;
			Ok(().into())
		}

		/// Schedule a task through XCMP to fire an XCMP message with a provided call.
		///
		/// Before the task can be scheduled the task must past validation checks.
		/// * The transaction is signed
		/// * The provided_id's length > 0
		/// * The para_id is that of the sender
		/// * The times are valid
		///
		/// # Parameters
		/// * `provided_id`: An id provided by the user. This id must be unique for the user.
		/// * `execution_times`: The list of unix standard times in seconds for when the task should run.
		/// * `para_id`: Parachain id the XCMP call will be sent to.
		/// * `call`: Call that will be sent via XCMP to the parachain id provided.
		/// * `weight_at_most`: Required weight at most the provided call will take.
		///
		/// # Errors
		/// * `InvalidTime`: Time must end in a whole hour.
		/// * `PastTime`: Time must be in the future.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		/// * `ParaIdMismatch`: ParaId provided does not match origin paraId.
		///
		/// TODO: Create benchmark for schedule_xcmp_task
		#[pallet::weight(<T as Config>::WeightInfo::schedule_notify_task_full(execution_times.len().try_into().unwrap()))]
		pub fn schedule_xcmp_task(
			origin: OriginFor<T>,
			provided_id: Vec<u8>,
			execution_times: Vec<UnixTime>,
			para_id: ParaId,
			currency_id: T::CurrencyId,
			encoded_call: Vec<u8>,
			encoded_call_weight: Weight,
		) -> DispatchResult {
			// Remove below directive when implemented
			#![allow(unused_variables)]
			let _who = ensure_signed(origin)?;

			Err(Error::<T>::TaskNotSupported)?;

			Ok(().into())
		}

		/// Schedule a task to increase delegation to a specified up to a minimum balance
		/// Task will reschedule itself to run on a given frequency until a failure occurs
		///
		/// # Parameters
		/// * `execution_time`: The unix timestamp when the task should run for the first time
		/// * `frequency`: Number of seconds to wait inbetween task executions
		/// * `collator_id`: Account ID of the target collator
		/// * `account_minimum`: The minimum amount of funds that should be left in the wallet
		///
		/// # Errors
		/// * `InvalidTime`: Execution time and frequency must end in a whole hour.
		/// * `PastTime`: Time must be in the future.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		/// * `TimeTooFarOut`: Execution time or frequency are past the max time horizon.
		/// * `InsufficientBalance`: Not enough funds to pay execution fee.
		#[pallet::weight(<T as Config>::WeightInfo::schedule_auto_compound_delegated_stake_task_full())]
		pub fn schedule_auto_compound_delegated_stake_task(
			origin: OriginFor<T>,
			execution_time: UnixTime,
			frequency: Seconds,
			collator_id: T::AccountId,
			account_minimum: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let provided_id: Vec<u8> =
				Self::generate_auto_compound_delegated_stake_provided_id(&who, &collator_id);

			// Validate frequency by ensuring that the next proposed execution is at a valid time
			let next_execution =
				execution_time.checked_add(frequency).ok_or(Error::<T>::TimeTooFarOut)?;
			Self::is_valid_time(next_execution)?;
			if next_execution == execution_time {
				Err(Error::<T>::InvalidTime)?;
			}

			let action = Action::AutoCompoundDelegatedStake {
				delegator: who.clone(),
				collator: collator_id,
				account_minimum,
				frequency,
			};
			Self::validate_and_schedule_task(action, who, provided_id, vec![execution_time; 1])?;
			Ok(().into())
		}

		/// Cancel a task.
		///
		/// Tasks can only can be cancelled by their owners.
		///
		/// # Parameters
		/// * `task_id`: The id of the task.
		///
		/// # Errors
		/// * `NotTaskOwner`: You are not the owner of the task.
		/// * `TaskDoesNotExist`: The task does not exist.
		#[pallet::weight(<T as Config>::WeightInfo::cancel_scheduled_task_full())]
		pub fn cancel_task(origin: OriginFor<T>, task_id: T::Hash) -> DispatchResult {
			let who = ensure_signed(origin)?;

			match Self::get_task(task_id) {
				None => Err(Error::<T>::TaskDoesNotExist)?,
				Some(task) => {
					if who != task.owner_id {
						Err(Error::<T>::NotTaskOwner)?
					}
					Self::remove_task(task_id, task);
				},
			}
			Ok(().into())
		}

		/// Sudo can force cancel a task.
		///
		/// # Parameters
		/// * `task_id`: The id of the task.
		///
		/// # Errors
		/// * `TaskDoesNotExist`: The task does not exist.
		#[pallet::weight(<T as Config>::WeightInfo::force_cancel_scheduled_task_full())]
		pub fn force_cancel_task(origin: OriginFor<T>, task_id: T::Hash) -> DispatchResult {
			ensure_root(origin)?;

			match Self::get_task(task_id) {
				None => Err(Error::<T>::TaskDoesNotExist)?,
				Some(task) => Self::remove_task(task_id, task),
			}

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Based on the block time, return the time slot.
		///
		/// In order to do this we:
		/// * Get the most recent timestamp from the block.
		/// * Convert the ms unix timestamp to seconds.
		/// * Bring the timestamp down to the last whole hour.
		pub fn get_current_time_slot() -> Result<UnixTime, DispatchError> {
			let now = <timestamp::Pallet<T>>::get()
				.checked_into::<UnixTime>()
				.ok_or(ArithmeticError::Overflow)?;

			if now == 0 {
				Err(Error::<T>::BlockTimeNotSet)?
			}

			let now = now.checked_div(1000).ok_or(ArithmeticError::Overflow)?;
			let diff_to_hour = now.checked_rem(3600).ok_or(ArithmeticError::Overflow)?;
			Ok(now.checked_sub(diff_to_hour).ok_or(ArithmeticError::Overflow)?)
		}

		/// Checks to see if the scheduled time is valid.
		///
		/// In order for a time to be valid it must
		/// - End in a whole hour
		/// - Be in the future
		/// - Not be more than MaxScheduleSeconds out
		fn is_valid_time(scheduled_time: UnixTime) -> Result<(), DispatchError> {
			#[cfg(feature = "dev-queue")]
			if scheduled_time == 0 {
				return Ok(())
			}

			let remainder = scheduled_time.checked_rem(3600).ok_or(ArithmeticError::Overflow)?;
			if remainder != 0 {
				Err(<Error<T>>::InvalidTime)?;
			}

			let current_time_slot = Self::get_current_time_slot()?;
			if scheduled_time <= current_time_slot {
				Err(<Error<T>>::PastTime)?;
			}

			let max_schedule_time = current_time_slot
				.checked_add(T::MaxScheduleSeconds::get())
				.ok_or(ArithmeticError::Overflow)?;

			if scheduled_time > max_schedule_time {
				Err(Error::<T>::TimeTooFarOut)?;
			}

			Ok(())
		}

		/// Cleans the executions times by removing duplicates and putting in ascending order.
		fn clean_execution_times_vector(execution_times: &mut Vec<UnixTime>) {
			execution_times.sort_unstable();
			execution_times.dedup();
		}

		/// Trigger tasks for the block time.
		///
		/// Complete as many tasks as possible given the maximum weight.
		pub fn trigger_tasks(max_weight: Weight) -> Weight {
			let mut weight_left: Weight = max_weight;

			// The last_missed_slot might not be caught up within just 1 block.
			// It might take multiple blocks to fully catch up, so we limit update to a max weight.
			let max_update_weight: Weight = T::UpdateQueueRatio::get().mul_floor(weight_left);
			let update_weight = Self::update_task_queue(max_update_weight);

			weight_left = weight_left.saturating_sub(update_weight);

			// need to calculate the weight of running just 1 task below.
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

			// if there is weight left we need to handled the missed tasks
			let run_missed_task_weight = <T as Config>::WeightInfo::run_missed_tasks_many_found(1)
				.saturating_add(T::DbWeight::get().reads(1 as Weight))
				.saturating_add(T::DbWeight::get().writes(1 as Weight));
			if weight_left >= run_missed_task_weight {
				let missed_queue = Self::get_missed_queue();
				weight_left = weight_left.saturating_sub(T::DbWeight::get().reads(1 as Weight));
				if missed_queue.len() > 0 {
					let (tasks_left, new_weight_left) =
						Self::run_missed_tasks(missed_queue, weight_left);

					MissedQueue::<T>::put(tasks_left);
					weight_left =
						new_weight_left.saturating_sub(T::DbWeight::get().writes(1 as Weight));
				}
			}

			max_weight.saturating_sub(weight_left)
		}

		/// Update the task queue.
		///
		/// This function checks to see if we are in a new time slot, and if so it updates the task queue and missing queue by doing the following.
		/// 1. (update_scheduled_task_queue) If new slot, append the current task queue to the missed queue and remove tasks from task queue.
		/// 2. (update_scheduled_task_queue) Move all tasks from the new slot into the task queue and remove the slot from Scheduled tasks map.
		/// 3. (update_missed_queue) If we skipped any time slots (due to an outage) move those tasks to the missed queue.
		/// 4. (update_missed_queue) Remove all missed time slots that were moved to missed queue from the Scheduled tasks map.
		///
		pub fn update_task_queue(allotted_weight: Weight) -> Weight {
			let mut total_weight = <T as Config>::WeightInfo::update_task_queue_overhead();

			let current_time_slot = match Self::get_current_time_slot() {
				Ok(time_slot) => time_slot,
				Err(_) => return total_weight,
			};

			if let Some((last_time_slot, last_missed_slot)) = Self::get_last_slot() {
				let missed_queue_allotted_weight = allotted_weight
					.saturating_sub(T::DbWeight::get().reads(1 as Weight))
					.saturating_sub(T::DbWeight::get().writes(1 as Weight))
					.saturating_sub(<T as Config>::WeightInfo::update_scheduled_task_queue());
				let (updated_last_time_slot, scheduled_queue_update_weight) =
					Self::update_scheduled_task_queue(current_time_slot, last_time_slot);
				let (updated_last_missed_slot, missed_queue_update_weight) =
					Self::update_missed_queue(
						current_time_slot,
						last_missed_slot,
						missed_queue_allotted_weight,
					);
				LastTimeSlot::<T>::put((updated_last_time_slot, updated_last_missed_slot));
				total_weight = total_weight
					.saturating_add(missed_queue_update_weight)
					.saturating_add(scheduled_queue_update_weight)
					.saturating_add(T::DbWeight::get().reads(1 as Weight));
			} else {
				LastTimeSlot::<T>::put((current_time_slot, current_time_slot));
				total_weight = total_weight
					.saturating_add(T::DbWeight::get().writes(1 as Weight))
					.saturating_add(T::DbWeight::get().reads(1 as Weight));
			}

			total_weight
		}

		/// Update the task queue with scheduled tasks for the current slot
		///
		/// 1. If new slot, append the current task queue to the missed queue and remove tasks from task queue.
		/// 2. Move all tasks from the new slot into the task queue and remove the slot from Scheduled tasks map.
		pub fn update_scheduled_task_queue(
			current_time_slot: u64,
			last_time_slot: u64,
		) -> (Weight, u64) {
			if current_time_slot != last_time_slot {
				let missed_tasks = Self::get_task_queue();
				let mut missed_queue = Self::get_missed_queue();
				for missed_task in missed_tasks {
					let new_missed_task: MissedTask<T> =
						MissedTask::<T> { task_id: missed_task, execution_time: last_time_slot };
					missed_queue.push(new_missed_task);
				}
				MissedQueue::<T>::put(missed_queue);
				// move current time slot to task queue or clear the task queue
				if let Some(task_ids) = Self::get_scheduled_tasks(current_time_slot) {
					TaskQueue::<T>::put(task_ids);
					ScheduledTasks::<T>::remove(current_time_slot);
				} else {
					let empty_queue: Vec<T::Hash> = vec![];
					TaskQueue::<T>::put(empty_queue);
				}
			}
			let weight_used = <T as Config>::WeightInfo::update_scheduled_task_queue();
			(current_time_slot, weight_used)
		}

		/// Checks if append_to_missed_tasks needs to run and then runs and measures weight as needed
		pub fn update_missed_queue(
			current_time_slot: u64,
			last_missed_slot: u64,
			allotted_weight: Weight,
		) -> (Weight, u64) {
			if current_time_slot != last_missed_slot {
				// will need to move missed time slots into missed queue
				let (append_weight, missed_slots_moved) = Self::append_to_missed_tasks(
					current_time_slot,
					last_missed_slot,
					allotted_weight,
				);

				let last_missed_slot_tracker =
					last_missed_slot.saturating_add(missed_slots_moved.saturating_mul(3600));
				let used_weight = append_weight;
				(last_missed_slot_tracker, used_weight)
			} else {
				(last_missed_slot, 0)
			}
		}

		/// Checks each previous time slots to move any missed tasks into the missed_queue
		///
		/// 1. If we skipped any time slots (due to an outage) move those tasks to the missed queue.
		/// 2. Remove all missed time slots that were moved to missed queue from the Scheduled tasks map.
		pub fn append_to_missed_tasks(
			current_time_slot: UnixTime,
			last_missed_slot: UnixTime,
			mut allotted_weight: Weight,
		) -> (Weight, u64) {
			// will need to move task queue into missed queue
			let mut missed_tasks = vec![];
			let mut diff =
				(current_time_slot.saturating_sub(last_missed_slot) / 3600).saturating_sub(1);
			for i in 0..diff {
				if allotted_weight < <T as Config>::WeightInfo::shift_missed_tasks() {
					diff = i;
					break
				}
				let mut slot_missed_tasks = Self::shift_missed_tasks(last_missed_slot, i);
				missed_tasks.append(&mut slot_missed_tasks);
				allotted_weight =
					allotted_weight.saturating_sub(<T as Config>::WeightInfo::shift_missed_tasks());
			}
			// Update the missed queue
			let mut missed_queue = Self::get_missed_queue();
			missed_queue.append(&mut missed_tasks);
			MissedQueue::<T>::put(missed_queue);

			let weight = <T as Config>::WeightInfo::append_to_missed_tasks(diff.saturated_into());
			(weight, diff)
		}

		/// Grabs all of the missed tasks from a time slot.
		/// The time slot to grab missed tasks is calculated given:
		/// 1. last missed slot that was stored
		/// 2. the number of slots that it should skip after that
		pub fn shift_missed_tasks(
			last_missed_slot: UnixTime,
			number_of_missed_slots: u64,
		) -> Vec<MissedTask<T>> {
			let mut tasks = vec![];
			let seconds_in_slot = 3600;
			let shift = seconds_in_slot.saturating_mul(number_of_missed_slots + 1);
			let new_time_slot = last_missed_slot.saturating_add(shift);
			if let Some(task_ids) = Self::get_scheduled_tasks(new_time_slot) {
				ScheduledTasks::<T>::remove(new_time_slot);
				for task_id in task_ids {
					let new_missed_task: MissedTask<T> =
						MissedTask::<T> { task_id, execution_time: new_time_slot };
					tasks.push(new_missed_task);
				}
			}
			return tasks
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
					Some(mut task) => {
						let task_action_weight = match task.action.clone() {
							Action::Notify { message } => Self::run_notify_task(message),
							Action::NativeTransfer { sender, recipient, amount } =>
								Self::run_native_transfer_task(
									sender,
									recipient,
									amount,
									task_id.clone(),
								),
							Action::XCMP { para_id, call, weight_at_most } =>
								Self::run_xcmp_task(para_id, call, weight_at_most, task_id.clone()),
							Action::AutoCompoundDelegatedStake {
								delegator,
								collator,
								account_minimum,
								frequency,
							} => {
								let (mut_task, weight) =
									Self::run_auto_compound_delegated_stake_task(
										delegator,
										collator,
										account_minimum,
										frequency,
										task_id.clone(),
										task,
									);
								task = mut_task;
								weight
							},
						};
						Self::decrement_task_and_remove_if_complete(*task_id, task);
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

		/// Send events for as many missed tasks as the weight allows from the provided vec of task_ids.
		///
		/// Returns a vec with the tasks that were not run and the remaining weight.
		pub fn run_missed_tasks(
			mut missed_tasks: Vec<MissedTask<T>>,
			mut weight_left: Weight,
		) -> (Vec<MissedTask<T>>, Weight) {
			let mut consumed_task_index: usize = 0;
			for missed_task in missed_tasks.iter() {
				consumed_task_index += 1;

				let action_weight = match Self::get_task(missed_task.task_id) {
					None => {
						Self::deposit_event(Event::TaskNotFound {
							task_id: missed_task.task_id.clone(),
						});
						<T as Config>::WeightInfo::run_missed_tasks_many_missing(1)
					},
					Some(task) => {
						Self::deposit_event(Event::TaskMissed {
							who: task.owner_id.clone(),
							task_id: missed_task.task_id.clone(),
							execution_time: missed_task.execution_time,
						});
						Self::decrement_task_and_remove_if_complete(missed_task.task_id, task);
						<T as Config>::WeightInfo::run_missed_tasks_many_found(1)
					},
				};

				weight_left = weight_left.saturating_sub(action_weight);

				if weight_left < <T as Config>::WeightInfo::run_missed_tasks_many_found(1) {
					break
				}
			}

			if consumed_task_index == missed_tasks.len() {
				return (vec![], weight_left)
			} else {
				return (missed_tasks.split_off(consumed_task_index), weight_left)
			}
		}

		/// Fire the notify event with the custom message.
		pub fn run_notify_task(message: Vec<u8>) -> Weight {
			Self::deposit_event(Event::Notify { message });
			<T as Config>::WeightInfo::run_notify_task()
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

		pub fn run_xcmp_task(
			para_id: ParaId,
			call: Vec<u8>,
			weight_at_most: Weight,
			task_id: T::Hash,
		) -> Weight {
			let destination = (1, Junction::Parachain(para_id.into()));
			let message = Xcm(vec![Transact {
				origin_type: OriginKind::Native,
				require_weight_at_most: weight_at_most,
				call: call.into(),
			}]);
			match T::XcmSender::send_xcm(destination, message) {
				Ok(()) => {
					Self::deposit_event(Event::SuccessfullySentXCMP { task_id, para_id });
				},
				Err(e) => {
					Self::deposit_event(Event::FailedToSendXCMP { task_id, para_id, error: e });
				},
			}
			// Adding 1 DB write that doesn't get accounted for in the benchmarks to run an xcmp task
			T::DbWeight::get()
				.writes(1)
				.saturating_add(<T as Config>::WeightInfo::run_xcmp_task())
		}

		/// Executes auto compounding delegation and reschedules task on success
		pub fn run_auto_compound_delegated_stake_task(
			delegator: T::AccountId,
			collator: T::AccountId,
			account_minimum: BalanceOf<T>,
			frequency: Seconds,
			task_id: T::Hash,
			mut task: Task<T>,
		) -> (Task<T>, Weight) {
			// TODO: Handle edge case where user has enough funds to run task but not reschedule
			let reserved_funds =
				account_minimum.saturating_add(Self::calculate_execution_fee(&task.action, 1));
			match T::DelegatorActions::delegator_bond_till_minimum(
				&delegator,
				&collator,
				reserved_funds,
			) {
				Ok(delegation) =>
					Self::deposit_event(Event::SuccesfullyAutoCompoundedDelegatorStake {
						task_id,
						amount: delegation,
					}),
				Err(e) => {
					Self::deposit_event(Event::AutoCompoundDelegatorStakeFailed {
						task_id,
						error_message: Into::<&str>::into(e).as_bytes().to_vec(),
						error: e,
					});
					return (
						task,
						// TODO: benchmark and return a smaller weight here to account for the early exit
						<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
					)
				},
			}

			let new_execution_times: Vec<UnixTime> =
				task.execution_times.iter().map(|when| when.saturating_add(frequency)).collect();
			let _ = Self::reschedule_existing_task(
				task_id,
				task.owner_id.clone(),
				&task.action,
				new_execution_times.clone(),
			)
			.map(|_| {
				let new_executions_left: u32 = new_execution_times.len().try_into().unwrap();
				task.executions_left += new_executions_left;
				new_execution_times.iter().try_for_each(|t| {
					task.execution_times.try_push(*t).and_then(|_| {
						task.execution_times.remove(0);
						Ok(())
					})
				})
			})
			.map_err(|e| {
				let err: DispatchErrorWithPostInfo = e.into();
				Self::deposit_event(Event::AutoCompoundDelegatorStakeFailed {
					task_id,
					error_message: Into::<&str>::into(err).as_bytes().to_vec(),
					error: err,
				});
			});

			(task, <T as Config>::WeightInfo::run_auto_compound_delegated_stake_task())
		}

		/// Decrements task executions left.
		/// If task is complete then removes task. If task not complete update task map.
		/// A task has been completed if executions left equals 0.
		fn decrement_task_and_remove_if_complete(task_id: T::Hash, mut task: Task<T>) {
			task.executions_left = task.executions_left.saturating_sub(1);
			if task.executions_left <= 0 {
				Tasks::<T>::remove(task_id);
			} else {
				Tasks::<T>::insert(task_id, task);
			}
		}

		/// Removes the task of the provided task_id and all scheduled tasks, including those in the task queue.
		fn remove_task(task_id: T::Hash, task: Task<T>) {
			let mut found_task: bool = false;
			Self::clean_execution_times_vector(&mut task.execution_times.to_vec());
			let current_time_slot = match Self::get_current_time_slot() {
				Ok(time_slot) => time_slot,
				// This will only occur for the first block in the chain.
				Err(_) => 0,
			};

			if let Some((last_time_slot, _)) = Self::get_last_slot() {
				for execution_time in task.execution_times.iter().rev() {
					// Execution time is less than current time slot and in the past.  No more execution times need to be removed.
					if *execution_time < current_time_slot {
						break
					}
					// Execution time is equal to last time slot and task queue should be checked for task id.
					// After checking task queue no other execution times need to be removed.
					if *execution_time == last_time_slot {
						let mut task_queue = Self::get_task_queue();
						for i in 0..task_queue.len() {
							if task_queue[i] == task_id {
								task_queue.remove(i);
								TaskQueue::<T>::put(task_queue);
								found_task = true;
								break
							}
						}
						break
					}
					// Execution time is greater than current time slot and in the future.  Remove task id from scheduled tasks.
					if let Some(mut task_ids) = Self::get_scheduled_tasks(*execution_time) {
						for i in 0..task_ids.len() {
							if task_ids[i] == task_id {
								if task_ids.len() == 1 {
									<ScheduledTasks<T>>::remove(*execution_time);
								} else {
									task_ids.remove(i);
									<ScheduledTasks<T>>::insert(*execution_time, task_ids);
								}
								found_task = true;
								break
							}
						}
					}
				}
			} else {
				// If last time slot does not exist then check each time in scheduled tasks and remove if exists.
				for execution_time in task.execution_times.iter().rev() {
					if let Some(mut task_ids) = Self::get_scheduled_tasks(*execution_time) {
						for i in 0..task_ids.len() {
							if task_ids[i] == task_id {
								if task_ids.len() == 1 {
									<ScheduledTasks<T>>::remove(*execution_time);
								} else {
									task_ids.remove(i);
									<ScheduledTasks<T>>::insert(*execution_time, task_ids);
								}
								found_task = true;
								break
							}
						}
					}
				}
			}

			if !found_task {
				Self::deposit_event(Event::TaskNotFound { task_id });
			}

			<Tasks<T>>::remove(task_id);
			Self::deposit_event(Event::TaskCancelled { who: task.owner_id, task_id });
		}

		/// Schedule task and return it's task_id.
		pub fn schedule_task(
			owner_id: AccountOf<T>,
			provided_id: Vec<u8>,
			execution_times: Vec<UnixTime>,
		) -> Result<T::Hash, Error<T>> {
			let task_id = Self::generate_task_id(owner_id.clone(), provided_id.clone());

			if let Some(_) = Self::get_task(task_id) {
				Err(Error::<T>::DuplicateTask)?
			}

			// If 'dev-queue' feature flag and execution_times equals [0], allows for putting a task directly on the task queue
			#[cfg(feature = "dev-queue")]
			if execution_times == vec![0] {
				let mut task_queue = Self::get_task_queue();
				task_queue.push(task_id);
				TaskQueue::<T>::put(task_queue);

				return Ok(task_id)
			}

			Self::insert_scheduled_tasks(task_id, execution_times)
		}

		/// Insert task id into scheduled tasks
		/// With transaction will protect against a partial success where N of M execution times might be full,
		/// rolling back any successful insertions into the schedule task table.
		fn insert_scheduled_tasks(
			task_id: T::Hash,
			execution_times: Vec<UnixTime>,
		) -> Result<T::Hash, Error<T>> {
			with_transaction(|| -> storage::TransactionOutcome<Result<T::Hash, DispatchError>> {
				for time in execution_times.iter() {
					match Self::get_scheduled_tasks(*time) {
						None => {
							let task_ids: BoundedVec<T::Hash, T::MaxTasksPerSlot> =
								vec![task_id].try_into().unwrap();
							<ScheduledTasks<T>>::insert(*time, task_ids);
						},
						Some(mut task_ids) => {
							if let Err(_) = task_ids.try_push(task_id) {
								return Rollback(Err(DispatchError::Other("time slot full")))
							}
							<ScheduledTasks<T>>::insert(*time, task_ids);
						},
					}
				}

				Commit(Ok(task_id))
			})
			.map_err(|_| Error::<T>::TimeSlotFull)
		}

		/// Validate and schedule task.
		/// This will also charge the execution fee.
		pub fn validate_and_schedule_task(
			action: Action<T>,
			who: T::AccountId,
			provided_id: Vec<u8>,
			mut execution_times: Vec<UnixTime>,
		) -> Result<(), DispatchError> {
			if provided_id.len() == 0 {
				Err(Error::<T>::EmptyProvidedId)?
			}

			Self::clean_execution_times_vector(&mut execution_times);
			let max_allowed_executions: usize = T::MaxExecutionTimes::get().try_into().unwrap();
			if execution_times.len() > max_allowed_executions {
				Err(Error::<T>::TooManyExecutionsTimes)?;
			}
			for time in execution_times.iter() {
				Self::is_valid_time(*time)?;
			}

			let fee =
				Self::calculate_execution_fee(&action, execution_times.len().try_into().unwrap());
			T::FeeHandler::can_pay_fee(&who, fee.clone())
				.map_err(|_| Error::<T>::InsufficientBalance)?;

			let task_id =
				Self::schedule_task(who.clone(), provided_id.clone(), execution_times.clone())?;
			let executions_left: u32 = execution_times.len().try_into().unwrap();
			let task: Task<T> = Task::<T> {
				owner_id: who.clone(),
				provided_id,
				execution_times: execution_times.try_into().unwrap(),
				executions_left,
				action,
			};
			<Tasks<T>>::insert(task_id, task);

			// This should never error if can_pay_fee passed.
			T::FeeHandler::withdraw_fee(&who, fee.clone())
				.map_err(|_| Error::<T>::LiquidityRestrictions)?;

			Self::deposit_event(Event::<T>::TaskScheduled { who, task_id });
			Ok(())
		}

		/// Reschedules an existing task for a given number of execution times
		fn reschedule_existing_task(
			task_id: T::Hash,
			who: T::AccountId,
			action: &Action<T>,
			execution_times: Vec<UnixTime>,
		) -> Result<(), DispatchError> {
			let new_executions = execution_times.len().try_into().unwrap();
			let fee = Self::calculate_execution_fee(action, new_executions);
			T::FeeHandler::can_pay_fee(&who, fee.clone())
				.map_err(|_| Error::<T>::InsufficientBalance)?;

			Self::insert_scheduled_tasks(task_id, execution_times.clone())?;

			T::FeeHandler::withdraw_fee(&who, fee.clone())
				.map_err(|_| Error::<T>::LiquidityRestrictions)?;

			Self::deposit_event(Event::<T>::TaskScheduled { who, task_id });
			Ok(())
		}

		pub fn generate_task_id(owner_id: AccountOf<T>, provided_id: Vec<u8>) -> T::Hash {
			let task_hash_input =
				TaskHashInput::<T> { owner_id: owner_id.clone(), provided_id: provided_id.clone() };
			T::Hashing::hash_of(&task_hash_input)
		}

		pub fn generate_auto_compound_delegated_stake_provided_id(
			delegator: &AccountOf<T>,
			collator: &AccountOf<T>,
		) -> Vec<u8> {
			let mut provided_id = "AutoCompoundDelegatedStake".as_bytes().to_vec();
			provided_id.extend(delegator.encode());
			provided_id.extend(collator.encode());
			provided_id
		}

		/// Calculates the execution fee for a given action based on weight and num of executions
		///
		/// Fee saturates at Weight/BalanceOf when there are an unreasonable num of executions
		/// In practice, executions is bounded by T::MaxExecutionTimes and unlikely to saturate
		pub fn calculate_execution_fee(action: &Action<T>, executions: u32) -> BalanceOf<T> {
			let action_weight = match action {
				Action::Notify { .. } => <T as Config>::WeightInfo::run_notify_task(),
				Action::NativeTransfer { .. } =>
					<T as Config>::WeightInfo::run_native_transfer_task(),
				// Adding 1 DB write that doesn't get accounted for in the benchmarks to run an xcmp task
				Action::XCMP { .. } => T::DbWeight::get()
					.writes(1)
					.saturating_add(<T as Config>::WeightInfo::run_xcmp_task()),
				Action::AutoCompoundDelegatedStake { .. } =>
					<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
			};

			let total_weight = action_weight.saturating_mul(executions.into());
			let weight_as_balance = <BalanceOf<T>>::saturated_from(total_weight);

			T::ExecutionWeightFee::get().saturating_mul(weight_as_balance)
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
