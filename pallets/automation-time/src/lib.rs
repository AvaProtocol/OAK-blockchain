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
//! This pallet allows a user to schedule tasks. Tasks can scheduled for any whole SlotSizeSeconds in the future.
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

mod fees;
pub use fees::*;

mod autocompounding;
pub use autocompounding::*;

mod types;
pub use types::*;

use codec::Decode;
use core::convert::TryInto;
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
	sp_runtime::traits::CheckedSub,
	storage::{
		with_transaction,
		TransactionOutcome::{Commit, Rollback},
	},
	traits::{Contains, Currency, IsSubType, OriginTrait},
	weights::constants::WEIGHT_REF_TIME_PER_SECOND,
};
use frame_system::pallet_prelude::*;
use orml_traits::{location::Reserve, FixedConversionRateProvider, MultiCurrency};
use pallet_parachain_staking::DelegatorActions;
use pallet_timestamp::{self as timestamp};
pub use pallet_xcmp_handler::InstructionSequence;
use pallet_xcmp_handler::XcmpTransactor;
use primitives::EnsureProxy;
use scale_info::{prelude::format, TypeInfo};
use sp_runtime::{
	traits::{CheckedConversion, Convert, Dispatchable, SaturatedConversion, Saturating},
	ArithmeticError, DispatchError, MultiAddress, Perbill,
};
use sp_std::{boxed::Box, collections::btree_map::BTreeMap, vec, vec::Vec};
pub use weights::WeightInfo;
use xcm::{latest::prelude::*, VersionedMultiLocation};

const AUTO_COMPOUND_DELEGATION_ABORT_ERRORS: [&str; 2] = ["DelegatorDNE", "DelegationDNE"];

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type MultiBalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	pub type TaskIdV2 = Vec<u8>;

	pub type AccountTaskId<T> = (AccountOf<T>, TaskIdV2);
	pub type ActionOf<T> = Action<AccountOf<T>, BalanceOf<T>>;
	pub type TaskOf<T> = Task<AccountOf<T>, BalanceOf<T>>;
	pub type MissedTaskV2Of<T> = MissedTaskV2<AccountOf<T>, TaskIdV2>;
	pub type ScheduledTasksOf<T> = ScheduledTasks<AccountOf<T>, TaskIdV2>;
	pub type MultiCurrencyId<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::CurrencyId;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

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
		type MaxBlockWeight: Get<u64>;

		/// The maximum percentage of weight per block used for scheduled tasks.
		#[pallet::constant]
		type MaxWeightPercentage: Get<Perbill>;

		/// The maximum supported execution weight per automation slot
		#[pallet::constant]
		type MaxWeightPerSlot: Get<u128>;

		/// The minimum time interval tasks could schedule for. For example, if the value is 600, then only inputs that are multiples of 600 are allowed. In other words, tasks can only be scheduled at 0, 10, 20 ... minutes of each hour.
		#[pallet::constant]
		type SlotSizeSeconds: Get<u64>;

		/// The maximum percentage of weight per block used for scheduled tasks.
		#[pallet::constant]
		type UpdateQueueRatio: Get<Perbill>;

		#[pallet::constant]
		type ExecutionWeightFee: Get<BalanceOf<Self>>;

		/// The Currency type for interacting with balances
		type Currency: Currency<Self::AccountId>;

		/// The MultiCurrency type for interacting with balances
		type MultiCurrency: MultiCurrency<Self::AccountId>;

		/// The currencyIds that our chain supports.
		type CurrencyId: Parameter
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ Ord
			+ TypeInfo
			+ MaxEncodedLen
			+ From<MultiCurrencyId<Self>>
			+ Into<MultiCurrencyId<Self>>
			+ From<u32>;

		/// Utility for sending XCM messages
		type XcmpTransactor: XcmpTransactor<Self::AccountId, Self::CurrencyId>;

		/// Converts CurrencyId to Multiloc
		type CurrencyIdConvert: Convert<Self::CurrencyId, Option<MultiLocation>>
			+ Convert<MultiLocation, Option<Self::CurrencyId>>;

		/// Converts between comparable currencies
		type FeeConversionRateProvider: FixedConversionRateProvider;

		/// Handler for fees
		type FeeHandler: HandleFees<Self>;

		type DelegatorActions: DelegatorActions<Self::AccountId, BalanceOf<Self>>;

		/// The overarching call type.
		type Call: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;

		type ScheduleAllowList: Contains<<Self as frame_system::Config>::RuntimeCall>;

		/// Ensure proxy
		type EnsureProxy: primitives::EnsureProxy<Self::AccountId>;

		/// This chain's Universal Location.
		type UniversalLocation: Get<InteriorMultiLocation>;

		type TransferCallCreator: primitives::TransferCallCreator<
			MultiAddress<Self::AccountId, ()>,
			BalanceOf<Self>,
			<Self as frame_system::Config>::RuntimeCall,
		>;

		/// The way to retreave the reserve of a MultiAsset. This can be
		/// configured to accept absolute or relative paths for self tokens
		type ReserveProvider: Reserve;

		/// Self chain location.
		#[pallet::constant]
		type SelfLocation: Get<MultiLocation>;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_scheduled_tasks)]
	pub type ScheduledTasksV3<T: Config> =
		StorageMap<_, Twox64Concat, UnixTime, ScheduledTasksOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_account_task)]
	pub type AccountTasks<T: Config> =
		StorageDoubleMap<_, Twox64Concat, AccountOf<T>, Twox64Concat, TaskIdV2, TaskOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_queue)]
	pub type TaskQueueV2<T: Config> = StorageValue<_, Vec<AccountTaskId<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_missed_queue)]
	pub type MissedQueueV2<T: Config> = StorageValue<_, Vec<MissedTaskV2Of<T>>, ValueQuery>;

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
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// Time in seconds must be a multiple of SlotSizeSeconds
		InvalidTime,
		/// Time must be in the future.
		PastTime,
		/// Time cannot be too far in the future.
		TimeTooFarOut,
		/// There can be no duplicate tasks.
		DuplicateTask,
		/// Time slot is full. No more tasks can be scheduled for this time.
		TimeSlotFull,
		/// The task does not exist.
		TaskDoesNotExist,
		/// The task schedule_as does not match.
		TaskScheduleAsNotMatch,
		/// Block time not set.
		BlockTimeNotSet,
		/// Insufficient balance to pay execution fee.
		InsufficientBalance,
		/// Account liquidity restrictions prevent withdrawal.
		LiquidityRestrictions,
		/// Too many execution times provided.
		TooManyExecutionsTimes,
		/// The call can no longer be decoded.
		CallCannotBeDecoded,
		/// Incoverible currency ID.
		IncoveribleCurrencyId,
		/// The version of the `VersionedMultiLocation` value used is not able
		/// to be interpreted.
		BadVersion,
		// The fee payment asset location is not supported.
		UnsupportedFeePayment,
		// Mulilocation cannot be reanchored.
		CannotReanchor,
		/// Invalid asset location.
		InvalidAssetLocation,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Schedule task success.
		TaskScheduled {
			who: AccountOf<T>,
			task_id: TaskIdV2,
			schedule_as: Option<AccountOf<T>>,
		},
		/// Cancelled a task.
		TaskCancelled {
			who: AccountOf<T>,
			task_id: TaskIdV2,
		},
		/// A Task was not found.
		TaskNotFound {
			who: AccountOf<T>,
			task_id: TaskIdV2,
		},
		/// The task could not be run at the scheduled time.
		TaskMissed {
			who: AccountOf<T>,
			task_id: TaskIdV2,
			execution_time: UnixTime,
		},
		/// A recurring task was rescheduled
		TaskRescheduled {
			who: AccountOf<T>,
			task_id: TaskIdV2,
			schedule_as: Option<AccountOf<T>>,
		},
		/// A recurring task was not rescheduled
		TaskNotRescheduled {
			who: AccountOf<T>,
			task_id: TaskIdV2,
			error: DispatchError,
		},
		/// A recurring task attempted but failed to be rescheduled
		TaskRescheduleFailed {
			who: AccountOf<T>,
			task_id: TaskIdV2,
			error: DispatchError,
		},
		TaskCompleted {
			who: AccountOf<T>,
			task_id: TaskIdV2,
		},
		TaskTriggered {
			who: AccountOf<T>,
			task_id: TaskIdV2,
			condition: BTreeMap<Vec<u8>, Vec<u8>>,
			encoded_call: Option<Vec<u8>>,
		},
		TaskExecuted {
			who: AccountOf<T>,
			task_id: TaskIdV2,
		},
		TaskExecutionFailed {
			who: AccountOf<T>,
			task_id: TaskIdV2,
			error: DispatchError,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_block: T::BlockNumber) -> Weight {
			if Self::is_shutdown() {
				return T::DbWeight::get().reads(1u64)
			}

			let max_weight: Weight = Weight::from_parts(
				T::MaxWeightPercentage::get().mul_floor(T::MaxBlockWeight::get()),
				0,
			);

			Self::trigger_tasks(max_weight)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedule a task through XCMP to fire an XCMP message with a provided call.
		///
		/// Before the task can be scheduled the task must past validation checks.
		/// * The transaction is signed
		/// * The times are valid
		/// * The given asset location is supported
		///
		/// # Parameters
		/// * `schedule`: The triggering rules for recurring task or the list of unix standard times in seconds for when the task should run.
		/// * `destination`: Destination the XCMP call will be sent to.
		/// * `schedule_fee`: The payment asset location required for scheduling automation task.
		/// * `execution_fee`: The fee will be paid for XCMP execution.
		/// * `encoded_call`: Call that will be sent via XCMP to the parachain id provided.
		/// * `encoded_call_weight`: Required weight at most the provided call will take.
		/// * `overall_weight`: The overall weight in which fees will be paid for XCM instructions.
		///
		/// # Errors
		/// * `InvalidTime`: Time in seconds must be a multiple of SlotSizeSeconds.
		/// * `PastTime`: Time must be in the future.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeTooFarOut`: Execution time or frequency are past the max time horizon.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		/// * `UnsupportedFeePayment`: Unsupported fee payment.
		/// * `InvalidAssetLocation` Invalid asset location.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_xcmp_task_full(schedule.number_of_executions()))]
		pub fn schedule_xcmp_task(
			origin: OriginFor<T>,
			schedule: ScheduleParam,
			destination: Box<VersionedMultiLocation>,
			schedule_fee: Box<VersionedMultiLocation>,
			execution_fee: Box<AssetPayment>,
			encoded_call: Vec<u8>,
			encoded_call_weight: Weight,
			overall_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let destination =
				MultiLocation::try_from(*destination).map_err(|()| Error::<T>::BadVersion)?;
			let schedule_fee =
				MultiLocation::try_from(*schedule_fee).map_err(|()| Error::<T>::BadVersion)?;

			let execution_fee: AssetPayment = *execution_fee;
			let execution_fee_location =
				MultiLocation::try_from(execution_fee.clone().asset_location)
					.map_err(|()| Error::<T>::BadVersion)?;

			Self::ensure_supported_execution_fee_location(&execution_fee_location, &destination)?;

			let action = Action::XCMP {
				destination,
				schedule_fee,
				execution_fee,
				encoded_call,
				encoded_call_weight,
				overall_weight,
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
			};

			let schedule = schedule.validated_into::<T>()?;

			Self::validate_and_schedule_task(action, who, schedule, vec![])?;
			Ok(())
		}

		/// Schedule a task through XCMP through proxy account to fire an XCMP message with a provided call.
		///
		/// Before the task can be scheduled the task must past validation checks.
		/// * The transaction is signed
		/// * The times are valid
		/// * The given asset location is supported
		///
		/// # Parameters
		/// * `schedule`: The triggering rules for recurring task or the list of unix standard times in seconds for when the task should run.
		/// * `destination`: Destination the XCMP call will be sent to.
		/// * `schedule_fee`: The payment asset location required for scheduling automation task.
		/// * `execution_fee`: The fee will be paid for XCMP execution.
		/// * `encoded_call`: Call that will be sent via XCMP to the parachain id provided.
		/// * `encoded_call_weight`: Required weight at most the provided call will take.
		/// * `overall_weight`: The overall weight in which fees will be paid for XCM instructions.
		///
		/// # Errors
		/// * `InvalidTime`: Time in seconds must be a multiple of SlotSizeSeconds.
		/// * `PastTime`: Time must be in the future.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeTooFarOut`: Execution time or frequency are past the max time horizon.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		/// * `Other("proxy error: expected `ProxyType::Any`")`: schedule_as must be a proxy account of type "any" for the caller.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_xcmp_task_full(schedule.number_of_executions()).saturating_add(T::DbWeight::get().reads(1)))]
		pub fn schedule_xcmp_task_through_proxy(
			origin: OriginFor<T>,
			schedule: ScheduleParam,
			destination: Box<VersionedMultiLocation>,
			schedule_fee: Box<VersionedMultiLocation>,
			execution_fee: Box<AssetPayment>,
			encoded_call: Vec<u8>,
			encoded_call_weight: Weight,
			overall_weight: Weight,
			schedule_as: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Make sure the owner is the proxy account of the user account.
			T::EnsureProxy::ensure_ok(schedule_as.clone(), who.clone())?;

			let destination =
				MultiLocation::try_from(*destination).map_err(|()| Error::<T>::BadVersion)?;
			let schedule_fee =
				MultiLocation::try_from(*schedule_fee).map_err(|()| Error::<T>::BadVersion)?;

			let action = Action::XCMP {
				destination,
				schedule_fee,
				execution_fee: *execution_fee,
				encoded_call,
				encoded_call_weight,
				overall_weight,
				schedule_as: Some(schedule_as),
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			};
			let schedule = schedule.validated_into::<T>()?;

			Self::validate_and_schedule_task(action, who, schedule, vec![])?;
			Ok(())
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
		/// * `InvalidTime`: Execution time and frequency must be a multiple of SlotSizeSeconds.
		/// * `PastTime`: Time must be in the future.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		/// * `TimeTooFarOut`: Execution time or frequency are past the max time horizon.
		/// * `InsufficientBalance`: Not enough funds to pay execution fee.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_auto_compound_delegated_stake_task_full())]
		pub fn schedule_auto_compound_delegated_stake_task(
			origin: OriginFor<T>,
			execution_time: UnixTime,
			frequency: Seconds,
			collator_id: AccountOf<T>,
			account_minimum: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let action = Action::AutoCompoundDelegatedStake {
				delegator: who.clone(),
				collator: collator_id,
				account_minimum,
			};
			let schedule = Schedule::new_recurring_schedule::<T>(execution_time, frequency)?;

			// List of errors causing the auto compound task to be terminated.
			let errors: Vec<Vec<u8>> = AUTO_COMPOUND_DELEGATION_ABORT_ERRORS
				.iter()
				.map(|&error| error.as_bytes().to_vec())
				.collect();

			Self::validate_and_schedule_task(action, who, schedule, errors)?;
			Ok(())
		}

		/// Schedule a task that will dispatch a call.
		/// ** This is currently limited to calls from the System and Balances pallets.
		///
		/// # Parameters
		/// * `execution_times`: The list of unix standard times in seconds for when the task should run.
		/// * `call`: The call that will be dispatched.
		///
		/// # Errors
		/// * `InvalidTime`: Execution time and frequency must be a multiple of SlotSizeSeconds.
		/// * `PastTime`: Time must be in the future.
		/// * `DuplicateTask`: There can be no duplicate tasks.
		/// * `TimeSlotFull`: Time slot is full. No more tasks can be scheduled for this time.
		/// * `TimeTooFarOut`: Execution time or frequency are past the max time horizon.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_dynamic_dispatch_task_full(schedule.number_of_executions()))]
		pub fn schedule_dynamic_dispatch_task(
			origin: OriginFor<T>,
			schedule: ScheduleParam,
			call: Box<<T as Config>::Call>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let encoded_call = call.encode();
			let action = Action::DynamicDispatch { encoded_call };
			let schedule = schedule.validated_into::<T>()?;

			Self::validate_and_schedule_task(action, who, schedule, vec![])?;
			Ok(())
		}

		/// Cancel a task.
		///
		/// Tasks can only can be cancelled by their owners.
		///
		/// # Parameters
		/// * `task_id`: The id of the task.
		///
		/// # Errors
		/// * `TaskDoesNotExist`: The task does not exist.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_scheduled_task_full())]
		pub fn cancel_task(origin: OriginFor<T>, task_id: TaskIdV2) -> DispatchResult {
			let who = ensure_signed(origin)?;

			AccountTasks::<T>::get(who, task_id.clone())
				.ok_or(Error::<T>::TaskDoesNotExist)
				.map(|task| Self::remove_task(task_id.clone(), task))?;

			Ok(())
		}

		/// Sudo can force cancel a task.
		///
		/// # Parameters
		/// * `owner_id`: The owner of the task.
		/// * `task_id`: The id of the task.
		///
		/// # Errors
		/// * `TaskDoesNotExist`: The task does not exist.
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::force_cancel_scheduled_task_full())]
		pub fn force_cancel_task(
			origin: OriginFor<T>,
			owner_id: AccountOf<T>,
			task_id: TaskIdV2,
		) -> DispatchResult {
			ensure_root(origin)?;

			AccountTasks::<T>::get(owner_id, task_id.clone())
				.ok_or(Error::<T>::TaskDoesNotExist)
				.map(|task| Self::remove_task(task_id.clone(), task))?;

			Ok(())
		}

		/// Cancel task by schedule_as
		///
		/// # Parameters
		/// * `schedule_as`: The schedule_as account of the task.
		/// * `task_id`: The id of the task.
		///
		/// # Errors
		/// * `TaskDoesNotExist`: The task does not exist.
		/// * `TaskScheduleAsNotMatch`: The schedule_as account of the task does not match.
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_task_with_schedule_as_full())]
		pub fn cancel_task_with_schedule_as(
			origin: OriginFor<T>,
			owner_id: AccountOf<T>,
			task_id: TaskIdV2,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let task = AccountTasks::<T>::get(owner_id, task_id.clone())
				.ok_or(Error::<T>::TaskDoesNotExist)?;

			if !matches!(task.clone().action, Action::XCMP { schedule_as: Some(ref s), .. } if s == &who)
			{
				return Err(Error::<T>::TaskScheduleAsNotMatch.into())
			}

			Self::remove_task(task_id, task);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Based on the block time, return the time slot.
		///
		/// In order to do this we:
		/// * Get the most recent timestamp from the block.
		/// * Convert the ms unix timestamp to seconds.
		/// * Bring the timestamp down to the last whole SlotSizeSeconds.
		pub fn get_current_time_slot() -> Result<UnixTime, DispatchError> {
			let now = <timestamp::Pallet<T>>::get()
				.checked_into::<UnixTime>()
				.ok_or(ArithmeticError::Overflow)?;

			if now == 0 {
				Err(Error::<T>::BlockTimeNotSet)?
			}

			let now = now.checked_div(1000).ok_or(ArithmeticError::Overflow)?;
			let diff_to_slot =
				now.checked_rem(T::SlotSizeSeconds::get()).ok_or(ArithmeticError::Overflow)?;
			Ok(now.checked_sub(diff_to_slot).ok_or(ArithmeticError::Overflow)?)
		}

		/// Checks to see if the scheduled time is valid.
		///
		/// In order for a time to be valid it must
		/// - A multiple of SlotSizeSeconds
		/// - Be in the future
		/// - Not be more than MaxScheduleSeconds out
		pub fn is_valid_time(scheduled_time: UnixTime) -> DispatchResult {
			#[cfg(feature = "dev-queue")]
			if scheduled_time == 0 {
				return Ok(())
			}

			let remainder = scheduled_time
				.checked_rem(T::SlotSizeSeconds::get())
				.ok_or(ArithmeticError::Overflow)?;
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
		pub fn clean_execution_times_vector(execution_times: &mut Vec<UnixTime>) {
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
			let max_update_weight: Weight =
				Weight::from_parts(T::UpdateQueueRatio::get().mul_floor(weight_left.ref_time()), 0);

			let update_weight = Self::update_task_queue(max_update_weight);

			weight_left = weight_left.saturating_sub(update_weight);

			// need to calculate the weight of running just 1 task below.
			let run_task_weight = <T as Config>::WeightInfo::run_tasks_many_found(1)
				.saturating_add(T::DbWeight::get().reads(1u64))
				.saturating_add(T::DbWeight::get().writes(1u64));

			if weight_left.ref_time() < run_task_weight.ref_time() {
				return weight_left
			}

			// run as many scheduled tasks as we can
			let task_queue = Self::get_task_queue();
			weight_left = weight_left.saturating_sub(T::DbWeight::get().reads(1u64));
			if !task_queue.is_empty() {
				let (tasks_left, new_weight_left) = Self::run_tasks(task_queue, weight_left);
				TaskQueueV2::<T>::put(tasks_left);
				weight_left = new_weight_left.saturating_sub(T::DbWeight::get().writes(1u64));
			}

			// if there is weight left we need to handled the missed tasks
			let run_missed_task_weight = <T as Config>::WeightInfo::run_missed_tasks_many_found(1)
				.saturating_add(T::DbWeight::get().reads(1u64))
				.saturating_add(T::DbWeight::get().writes(1u64));
			if weight_left.ref_time() >= run_missed_task_weight.ref_time() {
				let missed_queue = Self::get_missed_queue();
				weight_left = weight_left.saturating_sub(T::DbWeight::get().reads(1u64));
				if !missed_queue.is_empty() {
					let (tasks_left, new_weight_left) =
						Self::run_missed_tasks(missed_queue, weight_left);

					MissedQueueV2::<T>::put(tasks_left);
					weight_left = new_weight_left.saturating_sub(T::DbWeight::get().writes(1u64));
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
					.saturating_sub(T::DbWeight::get().reads(1u64))
					.saturating_sub(T::DbWeight::get().writes(1u64))
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
					.saturating_add(T::DbWeight::get().reads(1u64));
			} else {
				LastTimeSlot::<T>::put((current_time_slot, current_time_slot));
				total_weight = total_weight
					.saturating_add(T::DbWeight::get().writes(1u64))
					.saturating_add(T::DbWeight::get().reads(1u64));
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
		) -> (u64, Weight) {
			if current_time_slot != last_time_slot {
				let missed_tasks = Self::get_task_queue();
				let mut missed_queue = Self::get_missed_queue();
				for (account_id, task_id) in missed_tasks {
					let new_missed_task =
						MissedTaskV2Of::<T>::new(account_id, task_id, last_time_slot);
					missed_queue.push(new_missed_task);
				}
				MissedQueueV2::<T>::put(missed_queue);
				// move current time slot to task queue or clear the task queue
				if let Some(ScheduledTasksOf::<T> { tasks: account_task_ids, .. }) =
					Self::get_scheduled_tasks(current_time_slot)
				{
					TaskQueueV2::<T>::put(account_task_ids);
					ScheduledTasksV3::<T>::remove(current_time_slot);
				} else {
					let empty_queue: Vec<AccountTaskId<T>> = vec![];
					TaskQueueV2::<T>::put(empty_queue);
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
		) -> (u64, Weight) {
			if current_time_slot != last_missed_slot {
				// will need to move missed time slots into missed queue
				let (append_weight, missed_slots_moved) = Self::append_to_missed_tasks(
					current_time_slot,
					last_missed_slot,
					allotted_weight,
				);

				let last_missed_slot_tracker = last_missed_slot
					.saturating_add(missed_slots_moved.saturating_mul(T::SlotSizeSeconds::get()));
				let used_weight = append_weight;
				(last_missed_slot_tracker, used_weight)
			} else {
				(last_missed_slot, Weight::zero())
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
			let mut diff = (current_time_slot.saturating_sub(last_missed_slot) /
				T::SlotSizeSeconds::get())
			.saturating_sub(1);
			for i in 0..diff {
				if allotted_weight.ref_time() <
					<T as Config>::WeightInfo::shift_missed_tasks().ref_time()
				{
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
			MissedQueueV2::<T>::put(missed_queue);

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
		) -> Vec<MissedTaskV2Of<T>> {
			let mut tasks = vec![];
			let seconds_in_slot = T::SlotSizeSeconds::get();
			let shift = seconds_in_slot.saturating_mul(number_of_missed_slots + 1);
			let new_time_slot = last_missed_slot.saturating_add(shift);
			if let Some(ScheduledTasksOf::<T> { tasks: account_task_ids, .. }) =
				Self::get_scheduled_tasks(new_time_slot)
			{
				ScheduledTasksV3::<T>::remove(new_time_slot);
				for (account_id, task_id) in account_task_ids {
					let new_missed_task =
						MissedTaskV2Of::<T>::new(account_id, task_id, new_time_slot);
					tasks.push(new_missed_task);
				}
			}
			tasks
		}

		/// Runs as many tasks as the weight allows from the provided vec of task_ids.
		///
		/// Returns a vec with the tasks that were not run and the remaining weight.
		pub fn run_tasks(
			mut account_task_ids: Vec<AccountTaskId<T>>,
			mut weight_left: Weight,
		) -> (Vec<AccountTaskId<T>>, Weight) {
			let mut consumed_task_index: usize = 0;

			let time_slot = Self::get_current_time_slot();
			if time_slot.is_err() {
				return (account_task_ids, weight_left)
			}
			let time_slot = time_slot.unwrap();

			for (account_id, task_id) in account_task_ids.iter() {
				consumed_task_index.saturating_inc();
				let action_weight = match AccountTasks::<T>::get(account_id.clone(), task_id) {
					None => {
						Self::deposit_event(Event::TaskNotFound {
							who: account_id.clone(),
							task_id: task_id.clone(),
						});
						<T as Config>::WeightInfo::run_tasks_many_missing(1)
					},
					Some(task) => {
						let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
						condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
						condition.insert(
							"timestamp".as_bytes().to_vec(),
							format!("{}", time_slot).into_bytes(),
						);

						let encoded_call = match task.action.clone() {
							Action::XCMP { encoded_call, .. } => Some(encoded_call),
							Action::DynamicDispatch { encoded_call } => Some(encoded_call),
							_ => None,
						};

						Self::deposit_event(Event::TaskTriggered {
							who: account_id.clone(),
							task_id: task_id.clone(),
							condition,
							encoded_call,
						});

						let (task_action_weight, dispatch_error) = match task.action.clone() {
							Action::XCMP {
								destination,
								execution_fee,
								schedule_as,
								encoded_call,
								encoded_call_weight,
								overall_weight,
								instruction_sequence,
								..
							} => Self::run_xcmp_task(
								destination,
								schedule_as.unwrap_or(task.owner_id.clone()),
								execution_fee,
								encoded_call,
								encoded_call_weight,
								overall_weight,
								instruction_sequence,
							),
							Action::AutoCompoundDelegatedStake {
								delegator,
								collator,
								account_minimum,
							} => Self::run_auto_compound_delegated_stake_task(
								delegator,
								collator,
								account_minimum,
								&task,
							),
							Action::DynamicDispatch { encoded_call } =>
								Self::run_dynamic_dispatch_action(
									task.owner_id.clone(),
									encoded_call,
								),
						};

						// If an error occurs during the task execution process, the TaskExecutionFailed event will be emitted;
						// Otherwise, the TaskExecuted event will be thrown.
						if let Some(err) = dispatch_error {
							Self::deposit_event(Event::<T>::TaskExecutionFailed {
								who: task.owner_id.clone(),
								task_id: task_id.clone(),
								error: err,
							});
						} else {
							Self::deposit_event(Event::<T>::TaskExecuted {
								who: task.owner_id.clone(),
								task_id: task_id.clone(),
							});
						}

						Self::handle_task_post_processing(task_id.clone(), task, dispatch_error);
						task_action_weight
							.saturating_add(T::DbWeight::get().writes(1u64))
							.saturating_add(T::DbWeight::get().reads(1u64))
					},
				};

				weight_left = weight_left.saturating_sub(action_weight);

				if weight_left.ref_time() <
					<T as Config>::WeightInfo::run_tasks_many_found(1).ref_time()
				{
					break
				}
			}

			if consumed_task_index == account_task_ids.len() {
				(vec![], weight_left)
			} else {
				(account_task_ids.split_off(consumed_task_index), weight_left)
			}
		}

		/// Send events for as many missed tasks as the weight allows from the provided vec of task_ids.
		///
		/// Returns a vec with the tasks that were not run and the remaining weight.
		pub fn run_missed_tasks(
			mut missed_tasks: Vec<MissedTaskV2Of<T>>,
			mut weight_left: Weight,
		) -> (Vec<MissedTaskV2Of<T>>, Weight) {
			let mut consumed_task_index: usize = 0;
			for missed_task in missed_tasks.iter() {
				consumed_task_index += 1;

				let action_weight = match AccountTasks::<T>::get(
					missed_task.owner_id.clone(),
					missed_task.task_id.clone(),
				) {
					None => {
						Self::deposit_event(Event::TaskNotFound {
							who: missed_task.owner_id.clone(),
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
						Self::handle_task_post_processing(missed_task.task_id.clone(), task, None);
						<T as Config>::WeightInfo::run_missed_tasks_many_found(1)
					},
				};

				weight_left = weight_left.saturating_sub(action_weight);

				if weight_left.ref_time() <
					<T as Config>::WeightInfo::run_missed_tasks_many_found(1).ref_time()
				{
					break
				}
			}

			if consumed_task_index == missed_tasks.len() {
				(vec![], weight_left)
			} else {
				(missed_tasks.split_off(consumed_task_index), weight_left)
			}
		}

		pub fn run_xcmp_task(
			destination: MultiLocation,
			caller: T::AccountId,
			fee: AssetPayment,
			encoded_call: Vec<u8>,
			encoded_call_weight: Weight,
			overall_weight: Weight,
			flow: InstructionSequence,
		) -> (Weight, Option<DispatchError>) {
			let fee_asset_location = MultiLocation::try_from(fee.asset_location);
			if fee_asset_location.is_err() {
				return (
					<T as Config>::WeightInfo::run_xcmp_task(),
					Some(Error::<T>::BadVersion.into()),
				)
			}
			let fee_asset_location = fee_asset_location.unwrap();

			match T::XcmpTransactor::transact_xcm(
				destination,
				fee_asset_location,
				fee.amount,
				caller,
				encoded_call,
				encoded_call_weight,
				overall_weight,
				flow,
			) {
				Ok(()) => (<T as Config>::WeightInfo::run_xcmp_task(), None),
				Err(e) => (<T as Config>::WeightInfo::run_xcmp_task(), Some(e)),
			}
		}

		/// Executes auto compounding delegation and reschedules task on success
		pub fn run_auto_compound_delegated_stake_task(
			delegator: AccountOf<T>,
			collator: AccountOf<T>,
			account_minimum: BalanceOf<T>,
			task: &TaskOf<T>,
		) -> (Weight, Option<DispatchError>) {
			let fee_amount = Self::calculate_schedule_fee_amount(&task.action, 1);
			if let Err(error) = fee_amount {
				return (
					<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
					Some(error),
				)
			}
			let fee_amount = fee_amount.unwrap();

			let reserved_funds = account_minimum.saturating_add(fee_amount);
			match T::DelegatorActions::get_delegator_stakable_free_balance(&delegator)
				.checked_sub(&reserved_funds)
			{
				Some(delegation) => {
					match T::DelegatorActions::delegator_bond_more(
						&delegator, &collator, delegation,
					) {
						Ok(_) => (
							<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
							None,
						),
						Err(e) => (
							<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
							Some(e.error),
						),
					}
				},
				None => {
					// InsufficientBalance
					(
						<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
						Some(Error::<T>::InsufficientBalance.into()),
					)
				},
			}
		}

		/// Attempt to decode and run the call.
		pub fn run_dynamic_dispatch_action(
			caller: AccountOf<T>,
			encoded_call: Vec<u8>,
		) -> (Weight, Option<DispatchError>) {
			match <T as Config>::Call::decode(&mut &*encoded_call) {
				Ok(scheduled_call) => {
					let mut dispatch_origin: T::RuntimeOrigin =
						frame_system::RawOrigin::Signed(caller).into();
					dispatch_origin.add_filter(
						|call: &<T as frame_system::Config>::RuntimeCall| {
							T::ScheduleAllowList::contains(call)
						},
					);

					let call_weight = scheduled_call.get_dispatch_info().weight;

					let (maybe_actual_call_weight, result) =
						match scheduled_call.dispatch(dispatch_origin) {
							Ok(post_info) => (post_info.actual_weight, Ok(())),
							Err(error_and_info) =>
								(error_and_info.post_info.actual_weight, Err(error_and_info.error)),
						};

					(
						maybe_actual_call_weight.unwrap_or(call_weight).saturating_add(
							<T as Config>::WeightInfo::run_dynamic_dispatch_action(),
						),
						result.err(),
					)
				},
				Err(_) => (
					<T as Config>::WeightInfo::run_dynamic_dispatch_action_fail_decode(),
					Some(Error::<T>::CallCannotBeDecoded.into()),
				),
			}
		}

		/// Decrements task executions left.
		/// If task is complete then removes task. If task not complete update task map.
		/// A task has been completed if executions left equals 0.
		fn decrement_task_and_remove_if_complete(task_id: TaskIdV2, mut task: TaskOf<T>) {
			match task.schedule {
				Schedule::Fixed { ref mut executions_left, .. } => {
					*executions_left = executions_left.saturating_sub(1);
					if *executions_left == 0 {
						AccountTasks::<T>::remove(task.owner_id.clone(), task_id.clone());
						Self::deposit_event(Event::TaskCompleted {
							who: task.owner_id.clone(),
							task_id,
						});
					} else {
						AccountTasks::<T>::insert(task.owner_id.clone(), task_id, task);
					}
				},
				Schedule::Recurring { .. } => {},
			}
		}

		/// Removes the task of the provided task_id and all scheduled tasks, including those in the task queue.
		fn remove_task(task_id: TaskIdV2, task: TaskOf<T>) {
			let mut found_task: bool = false;
			let mut execution_times = task.execution_times();
			Self::clean_execution_times_vector(&mut execution_times);
			let current_time_slot = Self::get_current_time_slot().unwrap_or(0);

			if let Some((last_time_slot, _)) = Self::get_last_slot() {
				for execution_time in execution_times.iter().rev() {
					// Execution time is less than current time slot and in the past.  No more execution times need to be removed.
					if *execution_time < current_time_slot {
						break
					}
					// Execution time is equal to last time slot and task queue should be checked for task id.
					// After checking task queue no other execution times need to be removed.
					if *execution_time == last_time_slot {
						let mut task_queue = Self::get_task_queue();
						for i in 0..task_queue.len() {
							if task_queue[i].1 == task_id {
								task_queue.remove(i);
								TaskQueueV2::<T>::put(task_queue);
								found_task = true;
								break
							}
						}
						break
					}
					// Execution time is greater than current time slot and in the future.  Remove task id from scheduled tasks.
					if let Some(ScheduledTasksOf::<T> { tasks: mut account_task_ids, weight }) =
						Self::get_scheduled_tasks(*execution_time)
					{
						for i in 0..account_task_ids.len() {
							if account_task_ids[i].1 == task_id {
								if account_task_ids.len() == 1 {
									ScheduledTasksV3::<T>::remove(*execution_time);
								} else {
									account_task_ids.remove(i);
									ScheduledTasksV3::<T>::insert(
										*execution_time,
										ScheduledTasksOf::<T> {
											tasks: account_task_ids,
											weight: weight.saturating_sub(
												task.action.execution_weight::<T>().unwrap_or(0)
													as u128,
											),
										},
									);
								}
								found_task = true;
								break
							}
						}
					}
				}
			} else {
				// If last time slot does not exist then check each time in scheduled tasks and remove if exists.
				for execution_time in execution_times.iter().rev() {
					if let Some(ScheduledTasksOf::<T> { tasks: mut account_task_ids, weight }) =
						Self::get_scheduled_tasks(*execution_time)
					{
						for i in 0..account_task_ids.len() {
							if account_task_ids[i].1 == task_id {
								if account_task_ids.len() == 1 {
									ScheduledTasksV3::<T>::remove(*execution_time);
								} else {
									account_task_ids.remove(i);
									ScheduledTasksV3::<T>::insert(
										*execution_time,
										ScheduledTasksOf::<T> {
											tasks: account_task_ids,
											weight: weight.saturating_sub(
												task.action.execution_weight::<T>().unwrap_or(0)
													as u128,
											),
										},
									);
								}
								found_task = true;
								break
							}
						}
					}
				}
			}

			if !found_task {
				Self::deposit_event(Event::TaskNotFound {
					who: task.owner_id.clone(),
					task_id: task_id.clone(),
				});
			}

			// TODO: Add refund reserved execution fees here

			AccountTasks::<T>::remove(task.owner_id.clone(), task_id.clone());

			Self::deposit_event(Event::TaskCancelled { who: task.owner_id, task_id });
		}

		/// Schedule task and return it's task_id.
		pub fn schedule_task(task: &TaskOf<T>) -> Result<TaskIdV2, Error<T>> {
			let owner_id = task.owner_id.clone();

			let execution_times = task.execution_times();

			if AccountTasks::<T>::contains_key(&owner_id, task.task_id.clone()) {
				Err(Error::<T>::DuplicateTask)?;
			}

			// If 'dev-queue' feature flag and execution_times equals [0], allows for putting a task directly on the task queue
			#[cfg(feature = "dev-queue")]
			if execution_times == vec![0] {
				let mut task_queue = Self::get_task_queue();
				task_queue.push((owner_id, task.task_id.clone()));
				TaskQueueV2::<T>::put(task_queue);

				return Ok(task.task_id.clone())
			}

			Self::insert_scheduled_tasks(task, execution_times)
		}

		/// Insert the account/task id into scheduled tasks
		/// With transaction will protect against a partial success where N of M execution times might be full,
		/// rolling back any successful insertions into the schedule task table.
		fn insert_scheduled_tasks(
			task: &TaskOf<T>,
			execution_times: Vec<UnixTime>,
		) -> Result<TaskIdV2, Error<T>> {
			let task_id = task.task_id.clone();

			with_transaction(|| -> storage::TransactionOutcome<Result<TaskIdV2, DispatchError>> {
				for time in execution_times.iter() {
					let mut scheduled_tasks = Self::get_scheduled_tasks(*time).unwrap_or_default();
					if scheduled_tasks.try_push::<T, BalanceOf<T>>(task_id.clone(), task).is_err() {
						return Rollback(Err(DispatchError::Other("time slot full")))
					}
					<ScheduledTasksV3<T>>::insert(*time, scheduled_tasks);
				}

				Commit(Ok(task_id))
			})
			.map_err(|_| Error::<T>::TimeSlotFull)
		}

		/// Validate and schedule task.
		/// This will also charge the execution fee.
		pub fn validate_and_schedule_task(
			action: ActionOf<T>,
			owner_id: AccountOf<T>,
			schedule: Schedule,
			abort_errors: Vec<Vec<u8>>,
		) -> DispatchResult {
			match action.clone() {
				Action::XCMP { execution_fee, .. } => {
					let asset_location = MultiLocation::try_from(execution_fee.asset_location)
						.map_err(|()| Error::<T>::BadVersion)?;
					let _asset_location = asset_location
						.reanchored(&T::SelfLocation::get().into(), T::UniversalLocation::get())
						.map_err(|_| Error::<T>::CannotReanchor)?;
				},
				_ => (),
			};

			let executions = schedule.known_executions_left();

			let task = TaskOf::<T>::new(
				owner_id.clone(),
				Self::generate_task_idv2(),
				schedule,
				action.clone(),
				abort_errors,
			);

			let task_id =
				T::FeeHandler::pay_checked_fees_for(&owner_id, &action, executions, || {
					let task_id = Self::schedule_task(&task)?;
					AccountTasks::<T>::insert(owner_id.clone(), task_id.clone(), task);
					Ok(task_id)
				})?;

			let schedule_as = match action {
				Action::XCMP { schedule_as, .. } => schedule_as,
				_ => None,
			};

			Self::deposit_event(Event::<T>::TaskScheduled { who: owner_id, task_id, schedule_as });

			Ok(())
		}

		fn reschedule_or_remove_task(mut task: TaskOf<T>, dispatch_error: Option<DispatchError>) {
			let task_id = task.task_id.clone();
			// When the error can be found in the abort_errors list, the next task execution will not be scheduled.
			// Otherwise, continue to schedule next execution.
			match dispatch_error {
				Some(err)
					if err == DispatchError::from(Error::<T>::CallCannotBeDecoded) ||
						task.abort_errors
							.contains(&Into::<&str>::into(err).as_bytes().to_vec()) =>
				{
					Self::deposit_event(Event::<T>::TaskNotRescheduled {
						who: task.owner_id.clone(),
						task_id: task_id.clone(),
						error: err,
					});
					AccountTasks::<T>::remove(task.owner_id.clone(), task_id);
				},
				_ => {
					let owner_id = task.owner_id.clone();
					let action = task.action.clone();
					match Self::reschedule_existing_task(&mut task) {
						Ok(_) => {
							let schedule_as = match action {
								Action::XCMP { schedule_as, .. } => schedule_as,
								_ => None,
							};
							Self::deposit_event(Event::<T>::TaskRescheduled {
								who: owner_id.clone(),
								task_id: task_id.clone(),
								schedule_as,
							});
							AccountTasks::<T>::insert(owner_id, task_id, task.clone());
						},
						Err(err) => {
							Self::deposit_event(Event::<T>::TaskRescheduleFailed {
								who: task.owner_id.clone(),
								task_id: task_id.clone(),
								error: err,
							});
							AccountTasks::<T>::remove(task.owner_id.clone(), task_id);
						},
					};
				},
			}
		}

		fn reschedule_existing_task(task: &mut TaskOf<T>) -> DispatchResult {
			let task_id = task.task_id.clone();

			match task.schedule {
				Schedule::Recurring { ref mut next_execution_time, frequency } => {
					let new_execution_time = next_execution_time
						.checked_add(frequency)
						.ok_or(Error::<T>::InvalidTime)?;
					*next_execution_time = new_execution_time;

					// TODO: should execution fee depend on whether task is recurring?
					T::FeeHandler::pay_checked_fees_for(&task.owner_id, &task.action, 1, || {
						Self::insert_scheduled_tasks(task, vec![new_execution_time])
							.map_err(|e| e.into())
					})?;

					let owner_id = task.owner_id.clone();
					AccountTasks::<T>::insert(owner_id, task_id, task.clone());
				},
				Schedule::Fixed { .. } => {},
			}
			Ok(())
		}

		fn handle_task_post_processing(
			task_id: TaskIdV2,
			task: TaskOf<T>,
			error: Option<DispatchError>,
		) {
			match task.schedule {
				Schedule::Fixed { .. } =>
					Self::decrement_task_and_remove_if_complete(task_id, task),
				Schedule::Recurring { .. } => Self::reschedule_or_remove_task(task, error),
			}
		}

		pub fn generate_task_idv2() -> TaskIdV2 {
			let current_block_number =
				TryInto::<u64>::try_into(<frame_system::Pallet<T>>::block_number())
					.ok()
					.unwrap_or(0);

			let tx_id = <frame_system::Pallet<T>>::extrinsic_index().unwrap_or(0);

			let evt_index = <frame_system::Pallet<T>>::event_count();

			format!("{:}-{:}-{:}", current_block_number, tx_id, evt_index)
				.as_bytes()
				.to_vec()
		}

		// Find auto compount tasks of a paritcular account by iterate over AccountTasks
		// StorageDoubleMap using the first key prefix which is account_id.
		pub fn get_auto_compound_delegated_stake_task_ids(
			account_id: AccountOf<T>,
		) -> Vec<TaskIdV2> {
			AccountTasks::<T>::iter_prefix_values(account_id)
				.filter(|task| {
					match task.action {
						// We don't care about the inner content, we just want to pick out the
						// AutoCompoundDelegatedStake
						Action::AutoCompoundDelegatedStake { .. } => true,
						_ => false,
					}
				})
				.map(|task| task.task_id)
				.collect()
		}

		/// Calculates the execution fee for a given action based on weight and num of executions
		///
		/// Fee saturates at Weight/BalanceOf when there are an unreasonable num of executions
		/// In practice, executions is bounded by T::MaxExecutionTimes and unlikely to saturate
		pub fn calculate_schedule_fee_amount(
			action: &ActionOf<T>,
			executions: u32,
		) -> Result<BalanceOf<T>, DispatchError> {
			let total_weight = action.execution_weight::<T>()?.saturating_mul(executions.into());

			let schedule_fee_location = action.schedule_fee_location::<T>();
			let schedule_fee_location = schedule_fee_location
				.reanchored(&T::SelfLocation::get().into(), T::UniversalLocation::get())
				.map_err(|_| Error::<T>::CannotReanchor)?;

			let fee = if schedule_fee_location == MultiLocation::default() {
				T::ExecutionWeightFee::get()
					.saturating_mul(<BalanceOf<T>>::saturated_from(total_weight))
			} else {
				let raw_fee =
					T::FeeConversionRateProvider::get_fee_per_second(&schedule_fee_location)
						.ok_or("CouldNotDetermineFeePerSecond")?
						.checked_mul(total_weight as u128)
						.ok_or("FeeOverflow")
						.map(|raw_fee| raw_fee / (WEIGHT_REF_TIME_PER_SECOND as u128))?;
				<BalanceOf<T>>::saturated_from(raw_fee)
			};

			Ok(fee)
		}

		/// Checks if the execution fee location is supported for scheduling a task
		///
		/// if the locations can not be verified, an error such as InvalidAssetLocation or UnsupportedFeePayment will be thrown
		pub fn ensure_supported_execution_fee_location(
			exeuction_fee_location: &MultiLocation,
			destination: &MultiLocation,
		) -> Result<(), DispatchError> {
			let exeuction_fee =
				MultiAsset { id: Concrete(*exeuction_fee_location), fun: Fungibility::Fungible(0) };
			let reserve = T::ReserveProvider::reserve(&exeuction_fee)
				.ok_or(Error::<T>::InvalidAssetLocation)?;
			if reserve != MultiLocation::here() && &reserve != destination {
				return Err(Error::<T>::UnsupportedFeePayment.into())
			}

			Ok(())
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
