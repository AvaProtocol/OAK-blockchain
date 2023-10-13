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

//! # Automation price pallet
//!
//! DISCLAIMER: This pallet is still in it's early stages. At this point
//! we only support scheduling two tasks per hour, and sending an on-chain
//! with a custom message.
//!
//! This pallet allows a user to schedule tasks. Tasks can scheduled for any whole hour in the future.
//! In order to run tasks this pallet consumes up to a certain amount of weight during `on_initialize`.
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

pub mod weights;

pub mod types;
pub use types::*;

pub mod trigger;
pub use trigger::*;

mod fees;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod benchmarking;

pub use fees::*;

use codec::Decode;
use core::convert::{TryFrom, TryInto};
use cumulus_primitives_core::InteriorMultiLocation;

use cumulus_primitives_core::ParaId;
use frame_support::{pallet_prelude::*, traits::Currency, transactional};
use frame_system::pallet_prelude::*;
use orml_traits::{FixedConversionRateProvider, MultiCurrency};
use pallet_timestamp::{self as timestamp};
use scale_info::{prelude::format, TypeInfo};
use sp_runtime::{
	traits::{CheckedConversion, Convert, SaturatedConversion, Saturating},
	ArithmeticError, Perbill,
};
use sp_std::{
	boxed::Box,
	collections::btree_map::BTreeMap,
	ops::Bound::{Excluded, Included},
	vec,
	vec::Vec,
};

pub use pallet_xcmp_handler::InstructionSequence;
use primitives::EnsureProxy;
pub use weights::WeightInfo;

use pallet_xcmp_handler::XcmpTransactor;
use xcm::{latest::prelude::*, VersionedMultiLocation};

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type MultiBalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	pub type ActionOf<T> = Action<AccountOf<T>>;

	pub type MultiCurrencyId<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::CurrencyId;

	type UnixTime = u64;
	pub type TaskId = Vec<u8>;
	pub type TaskAddress<T> = (AccountOf<T>, TaskId);
	pub type TaskIdList<T> = Vec<TaskAddress<T>>;

	type ChainName = Vec<u8>;
	type Exchange = Vec<u8>;

	type AssetName = Vec<u8>;
	type AssetPair = (AssetName, AssetName);
	type AssetPrice = u128;
	type TriggerFunction = Vec<u8>;

	/// The struct that stores all information needed for a task.
	#[derive(Debug, Eq, Encode, Decode, TypeInfo, Clone)]
	#[scale_info(skip_type_params(T))]
	pub struct Task<T: Config> {
		// origin data from the account schedule the tasks
		pub owner_id: AccountOf<T>,

		// generated data
		pub task_id: TaskId,

		// user input data
		pub chain: ChainName,
		pub exchange: Exchange,
		pub asset_pair: AssetPair,
		pub expired_at: u128,

		// TODO: Maybe expose enum?
		pub trigger_function: Vec<u8>,
		pub trigger_params: Vec<u128>,
		pub action: ActionOf<T>,
	}

	/// Needed for assert_eq to compare Tasks in tests due to BoundedVec.
	impl<T: Config> PartialEq for Task<T> {
		fn eq(&self, other: &Self) -> bool {
			// TODO: correct this
			self.owner_id == other.owner_id &&
				self.task_id == other.task_id &&
				self.asset_pair == other.asset_pair &&
				self.trigger_function == other.trigger_function &&
				self.trigger_params == other.trigger_params
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The maximum number of tasks that can be scheduled for a time slot.
		#[pallet::constant]
		type MaxTasksPerSlot: Get<u32>;

		/// The maximum number of tasks that a single user can schedule
		#[pallet::constant]
		type MaxTasksPerAccount: Get<u32>;

		/// The maximum number of tasks across our entire system
		#[pallet::constant]
		type MaxTasksOverall: Get<u32>;

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

		/// Converts CurrencyId to Multiloc
		type CurrencyIdConvert: Convert<Self::CurrencyId, Option<MultiLocation>>
			+ Convert<MultiLocation, Option<Self::CurrencyId>>;

		/// Handler for fees
		type FeeHandler: HandleFees<Self>;

		//type Origin: From<<Self as SystemConfig>::RuntimeOrigin>
		//	+ Into<Result<CumulusOrigin, <Self as Config>::Origin>>;

		/// Converts between comparable currencies
		type FeeConversionRateProvider: FixedConversionRateProvider;

		/// This chain's Universal Location.
		type UniversalLocation: Get<InteriorMultiLocation>;

		//The paraId of this chain.
		type SelfParaId: Get<ParaId>;

		/// Utility for sending XCM messages
		type XcmpTransactor: XcmpTransactor<Self::AccountId, Self::CurrencyId>;

		/// Ensure proxy
		type EnsureProxy: primitives::EnsureProxy<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// TODO: Cleanup before merge
	#[derive(Debug, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct RegistryInfo<T: Config> {
		round: u128,
		decimal: u8,
		last_update: u64,
		oracle_providers: Vec<AccountOf<T>>,
	}

	// TODO: Use a ring buffer to also store last n history data effectively
	#[derive(Debug, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct PriceData {
		pub round: u128,
		pub nonce: u128,
		pub amount: u128,
	}

	// AssetRegistry holds information and metadata about the asset we support
	#[pallet::storage]
	#[pallet::getter(fn get_asset_registry_info)]
	pub type AssetRegistry<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, ChainName>,
			NMapKey<Twox64Concat, Exchange>,
			NMapKey<Twox64Concat, AssetPair>,
		),
		RegistryInfo<T>,
	>;

	// PriceRegistry holds price only information for the asset we support
	#[pallet::storage]
	#[pallet::getter(fn get_asset_price_data)]
	pub type PriceRegistry<T> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, ChainName>,
			NMapKey<Twox64Concat, Exchange>,
			NMapKey<Twox64Concat, AssetPair>,
		),
		PriceData,
	>;

	// SortedTasksIndex is our sorted by price task shard
	// Each task for a given asset is organized into a BTreeMap
	// https://doc.rust-lang.org/std/collections/struct.BTreeMap.html#method.insert
	// - key: Trigger Price
	// - value: vector of task id
	// TODO: move these to a trigger model
	// TODO: handle task expiration
	#[pallet::storage]
	#[pallet::getter(fn get_sorted_tasks_index)]
	pub type SortedTasksIndex<T> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, ChainName>,
			NMapKey<Twox64Concat, Exchange>,
			NMapKey<Twox64Concat, AssetPair>,
			NMapKey<Twox64Concat, TriggerFunction>,
		),
		BTreeMap<AssetPrice, TaskIdList<T>>,
	>;

	// SortedTasksByExpiration is our expiration sorted tasks
	#[pallet::type_value]
	pub fn DefaultSortedTasksByExpiration<T: Config>(
	) -> BTreeMap<u128, BTreeMap<TaskId, AccountOf<T>>> {
		BTreeMap::<u128, BTreeMap<TaskId, AccountOf<T>>>::new()
	}
	#[pallet::storage]
	#[pallet::getter(fn get_sorted_tasks_by_expiration)]
	pub type SortedTasksByExpiration<T> = StorageValue<
		Value = BTreeMap<u128, BTreeMap<TaskId, AccountOf<T>>>,
		QueryKind = ValueQuery,
		OnEmpty = DefaultSortedTasksByExpiration<T>,
	>;

	// All active tasks, but organized by account
	// In this storage, we only interested in returning task belong to an account, we also want to
	// have fast lookup for task inserted/remove into the storage
	//
	// We also want to remove the expired task, so by leveraging this
	#[pallet::storage]
	#[pallet::getter(fn get_task)]
	pub type Tasks<T: Config> =
		StorageDoubleMap<_, Twox64Concat, AccountOf<T>, Twox64Concat, TaskId, Task<T>>;

	// Track various metric on our chain regarding tasks such as total task
	//
	#[pallet::storage]
	#[pallet::getter(fn get_task_stat)]
	pub type TaskStats<T: Config> = StorageMap<_, Twox64Concat, StatType, u64>;

	// Track various metric per account regarding tasks
	// To count task per account, relying on Tasks storage alone mean we have to iterate overs
	// value that share the first key (owner_id) to count.
	//
	// Store the task count
	#[pallet::storage]
	#[pallet::getter(fn get_account_stat)]
	pub type AccountStats<T: Config> =
		StorageDoubleMap<_, Twox64Concat, AccountOf<T>, Twox64Concat, StatType, u64>;

	// TaskQueue stores the task to be executed. To run any tasks, they need to be move into this
	// queue, from there our task execution pick it up and run it
	//
	// When task is run, we check the price once more and if it fall out of range, we move the task
	// back to the Tasks Registry
	//
	// If the task is expired, we also won't run
	#[pallet::storage]
	#[pallet::getter(fn get_task_queue)]
	pub type TaskQueue<T: Config> = StorageValue<_, TaskIdList<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn is_shutdown)]
	pub type Shutdown<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		InvalidTaskId,
		/// Duplicate task
		DuplicateTask,

		/// Non existent asset
		AssetNotSupported,
		AssetNotInitialized,
		/// Asset already supported
		AssetAlreadySupported,
		AssetAlreadyInitialized,
		/// Asset cannot be updated by this account
		InvalidAssetSudo,
		OracleNotAuthorized,
		/// Asset must be in triggerable range.
		AssetNotInTriggerableRange,
		AssetUpdatePayloadMalform,
		/// Block Time not set
		BlockTimeNotSet,
		/// Invalid Expiration Window for new asset
		InvalidAssetExpirationWindow,
		/// Maximum tasks reached for the slot
		MaxTasksReached,
		/// Maximum tasks reached for a given account
		MaxTasksPerAccountReached,
		/// Failed to insert task
		TaskInsertionFailure,
		/// Failed to remove task
		TaskRemoveFailure,
		/// Task Not Found When canceling
		TaskNotFound,
		/// Error when setting task expired less than the current block time
		InvalidTaskExpiredAt,
		/// Error when failed to update task expiration storage
		TaskExpiredStorageFailedToUpdate,
		/// Insufficient Balance
		InsufficientBalance,
		/// Restrictions on Liquidity in Account
		LiquidityRestrictions,
		/// Too Many Assets Created
		AssetLimitReached,

		/// The version of the `VersionedMultiLocation` value used is not able
		/// to be interpreted.
		BadVersion,
	}

	/// This is a event helper struct to help us making sense of the chain state and surrounded
	/// environment state when we emit an event during task execution or task scheduling.
	///
	/// They should contains enough information for an operator to look at and reason about "why do we
	/// got here".
	/// Many fields on this struct is optinal to support multiple error condition
	#[derive(Debug, Encode, Eq, PartialEq, Decode, TypeInfo, Clone)]
	pub enum TaskCondition {
		TargetPriceMatched {
			// record the state of the asset at the time the task is triggered
			// when debugging we can use this to reason about why did the task is trigger
			chain: ChainName,
			exchange: Exchange,
			asset_pair: AssetPair,
			price: u128,
		},
		AlreadyExpired {
			// the original expired_at of this task
			expired_at: u128,
			// the block time when we emit this event. expired_at should always <= now
			now: u128,
		},

		PriceAlreadyMoved {
			chain: ChainName,
			exchange: Exchange,
			asset_pair: AssetPair,
			price: u128,

			// The target price the task set
			target_price: u128,
		},
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Schedule task success.
		TaskScheduled {
			who: AccountOf<T>,
			task_id: TaskId,
		},
		// an event when we're about to run the task
		TaskTriggered {
			who: AccountOf<T>,
			task_id: TaskId,
			condition: TaskCondition,
		},
		// An event when the task ran succesfully
		TaskExecuted {
			who: AccountOf<T>,
			task_id: TaskId,
		},
		// An event when the task is trigger, ran but result in an error
		TaskExecutionFailed {
			who: AccountOf<T>,
			task_id: TaskId,
			error: DispatchError,
		},
		// An event when the task is completed and removed from all of the queue
		TaskCompleted {
			who: AccountOf<T>,
			task_id: TaskId,
		},
		// An event when the task is cancelled, either by owner or by root
		TaskCancelled {
			who: AccountOf<T>,
			task_id: TaskId,
		},
		// An event whenever we expect a task but cannot find it
		TaskNotFound {
			who: AccountOf<T>,
			task_id: TaskId,
		},
		// An event when we are about to run task, but the task has expired right before
		// it's actually run
		TaskExpired {
			who: AccountOf<T>,
			task_id: TaskId,
			condition: TaskCondition,
		},
		// An event when we are proactively sweep expired task
		// it's actually run
		TaskSweep {
			who: AccountOf<T>,
			task_id: TaskId,
			condition: TaskCondition,
		},
		// An event happen in extreme case, where the chain is too busy, and there is pending task
		// from previous block, and their respectively price has now moved against their matching
		// target range
		PriceAlreadyMoved {
			who: AccountOf<T>,
			task_id: TaskId,
			condition: TaskCondition,
		},
		AssetCreated {
			chain: ChainName,
			exchange: Exchange,
			asset1: AssetName,
			asset2: AssetName,
			decimal: u8,
		},
		AssetUpdated {
			who: AccountOf<T>,
			chain: ChainName,
			exchange: Exchange,
			asset1: AssetName,
			asset2: AssetName,
			price: u128,
		},
		AssetDeleted {
			chain: ChainName,
			exchange: Exchange,
			asset1: AssetName,
			asset2: AssetName,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: T::BlockNumber) -> Weight {
			if Self::is_shutdown() {
				return T::DbWeight::get().reads(1u64)
			}

			let max_weight: Weight = Weight::from_ref_time(
				T::MaxWeightPercentage::get().mul_floor(T::MaxBlockWeight::get()),
			);
			Self::trigger_tasks(max_weight)
		}

		fn on_idle(_: T::BlockNumber, remaining_weight: Weight) -> Weight {
			Self::sweep_expired_task(remaining_weight)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
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
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::initialize_asset_extrinsic(asset_owners.len() as u32))]
		#[transactional]
		pub fn initialize_asset(
			origin: OriginFor<T>,
			chain: Vec<u8>,
			exchange: Vec<u8>,
			asset1: AssetName,
			asset2: AssetName,
			decimal: u8,
			asset_owners: Vec<AccountOf<T>>,
		) -> DispatchResult {
			// TODO: use sudo and remove this feature flag
			// TODO: needs fees if opened up to non-sudo
			// When enable dev-queue, we skip this check
			#[cfg(not(feature = "dev-queue"))]
			ensure_root(origin)?;

			Self::create_new_asset(chain, exchange, asset1, asset2, decimal, asset_owners)?;

			Ok(())
		}

		/// Update prices of multiple asset pairs at the same time
		///
		/// Only authorized origin can update the price. The authorized origin is set when
		/// initializing an asset.
		///
		/// An asset is identified by this tuple: (chain, exchange, (asset1, asset2)).
		///
		/// To support updating multiple pairs, each element of the tuple become a separate
		/// argument to this function, where as each of these argument is a vector.
		///
		/// Every element of each vector arguments, in the same position in the vector form the
		/// above tuple.
		///
		/// # Parameters
		/// * `chains`: a vector of chain names
		/// * `exchange`: a vector of  exchange name
		/// * `asset1`: a vector of asset1 name
		/// * `asset2`: a vector of asset2 name
		/// * `prices`: a vector of price of asset1, re-present in asset2
		/// * `submitted_at`: a vector of epoch. This epoch is the time when the price is recognized from the oracle provider
		/// * `rounds`: a number to re-present which round of the asset price we're updating.  Unused internally
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::asset_price_update_extrinsic(assets1.len() as u32))]
		#[transactional]
		pub fn update_asset_prices(
			origin: OriginFor<T>,
			chains: Vec<ChainName>,
			exchanges: Vec<Exchange>,
			assets1: Vec<AssetName>,
			assets2: Vec<AssetName>,
			prices: Vec<AssetPrice>,
			submitted_at: Vec<u128>,
			rounds: Vec<u128>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if !(chains.len() == exchanges.len() &&
				exchanges.len() == assets1.len() &&
				assets1.len() == assets2.len() &&
				assets2.len() == prices.len() &&
				prices.len() == submitted_at.len() &&
				submitted_at.len() == rounds.len())
			{
				Err(Error::<T>::AssetUpdatePayloadMalform)?
			}

			for (index, price) in prices.clone().iter().enumerate() {
				let index: usize = index.try_into().unwrap();

				let chain = chains[index].clone();
				let exchange = exchanges[index].clone();
				let asset1 = assets1[index].clone();
				let asset2 = assets2[index].clone();
				let round = rounds[index].clone();

				let key = (&chain, &exchange, (&asset1, &asset2));

				if !AssetRegistry::<T>::contains_key(&key) {
					Err(Error::<T>::AssetNotInitialized)?
				}

				if let Some(asset_registry) = Self::get_asset_registry_info(key) {
					let allow_wallets: Vec<AccountOf<T>> = asset_registry.oracle_providers;
					if !allow_wallets.contains(&who) {
						Err(Error::<T>::OracleNotAuthorized)?
					}

					// TODO: Add round and nonce check logic
					PriceRegistry::<T>::insert(&key, PriceData { round, nonce: 1, amount: *price });

					Self::deposit_event(Event::AssetUpdated {
						who: who.clone(),
						chain,
						exchange,
						asset1,
						asset2,
						price: *price,
					});
				}
			}
			Ok(().into())
		}

		/// Delete an asset. Delete may not happen immediately if there was  task scheduled for
		/// this asset. Upon
		///
		/// # Parameters
		/// * `asset`: asset type
		/// * `directions`: number of directions of data input. (up, down, ?)
		///
		/// # Errors
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::initialize_asset_extrinsic(1))]
		#[transactional]
		pub fn delete_asset(
			origin: OriginFor<T>,
			chain: ChainName,
			exchange: Exchange,
			asset1: AssetName,
			asset2: AssetName,
		) -> DispatchResult {
			// TODO: use sudo and remove this feature flag
			// When enable dev queue, we want to skip this root check so local development can
			// happen easier
			#[cfg(not(feature = "dev-queue"))]
			ensure_root(origin)?;

			let key = (&chain, &exchange, (&asset1, &asset2));
			if let Some(asset_info) = Self::get_asset_registry_info(&key) {
				AssetRegistry::<T>::remove(&key);
				PriceRegistry::<T>::remove(&key);
				Self::deposit_event(Event::AssetDeleted { chain, exchange, asset1, asset2 });
			} else {
				Err(Error::<T>::AssetNotSupported)?
			}
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_xcmp_task_extrinsic())]
		#[transactional]
		pub fn schedule_xcmp_task(
			origin: OriginFor<T>,
			chain: ChainName,
			exchange: Exchange,
			asset1: AssetName,
			asset2: AssetName,
			expired_at: u128,
			trigger_function: Vec<u8>,
			trigger_param: Vec<u128>,
			destination: Box<VersionedMultiLocation>,
			schedule_fee: Box<VersionedMultiLocation>,
			execution_fee: Box<AssetPayment>,
			encoded_call: Vec<u8>,
			encoded_call_weight: Weight,
			overall_weight: Weight,
		) -> DispatchResult {
			// Step 1:
			//   Build Task and put it into the task registry
			// Step 2:
			//   Put task id on the index
			// TODO: the value to be inserted into the BTree should come from a function that
			// extract value from param
			//
			// TODO: HANDLE FEE to see user can pay fee
			let who = ensure_signed(origin)?;
			let task_id = Self::generate_task_id();

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
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
			};

			let task: Task<T> = Task::<T> {
				owner_id: who.clone(),
				task_id: task_id.clone(),
				chain,
				exchange,
				asset_pair: (asset1, asset2),
				expired_at,
				trigger_function,
				trigger_params: trigger_param,
				action,
			};

			Self::validate_and_schedule_task(task)?;
			// TODO withdraw fee
			//T::FeeHandler::withdraw_fee(&who, fee).map_err(|_| Error::<T>::InsufficientBalance)?;
			Ok(())
		}

		/// TODO: correct weight to use schedule_xcmp_task
		/// Schedule a task through XCMP through proxy account to fire an XCMP message with a provided call.
		///
		/// Before the task can be scheduled the task must past validation checks.
		/// * The transaction is signed
		/// * The asset pair is already initialized
		///
		/// # Parameters
		/// * `chain`: The chain name where we will send the task over
		/// * `exchange`: the exchange name where we
		/// * `asset1`: The payment asset location required for scheduling automation task.
		/// * `asset2`: The fee will be paid for XCMP execution.
		/// * `expired_at`: the epoch when after that time we will remove the task if it has not been executed yet
		/// * `trigger_function`: currently only support `gt` or `lt`. Essentially mean greater than or less than.
		/// * `trigger_params`: a list of parameter to feed into `trigger_function`. with `gt` and `lt` we only need to pass the target price as a single element vector
		/// * `schedule_fee`: The payment asset location required for scheduling automation task.
		/// * `execution_fee`: The fee will be paid for XCMP execution.
		/// * `encoded_call`: Call that will be sent via XCMP to the parachain id provided.
		/// * `encoded_call_weight`: Required weight at most the provided call will take.
		/// * `overall_weight`: The overall weight in which fees will be paid for XCM instructions.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_xcmp_task_extrinsic().saturating_add(T::DbWeight::get().reads(1)))]
		#[transactional]
		pub fn schedule_xcmp_task_through_proxy(
			origin: OriginFor<T>,
			chain: ChainName,
			exchange: Exchange,
			asset1: AssetName,
			asset2: AssetName,
			expired_at: u128,
			trigger_function: Vec<u8>,
			trigger_params: Vec<u128>,

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

			let task_id = Self::generate_task_id();
			let task: Task<T> = Task::<T> {
				owner_id: who.clone(),
				task_id: task_id.clone(),
				chain,
				exchange,
				asset_pair: (asset1, asset2),
				expired_at,
				trigger_function,
				trigger_params,
				action,
			};

			Self::validate_and_schedule_task(task)?;
			Ok(())
		}

		// When cancel task we remove it from:
		//   Task Registry
		//   SortedTasksIndex
		//   AccountTasks
		//   Task Queue: if the task is already on the queue but haven't got run yet,
		//               we will attemppt to remove it
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_task_extrinsic())]
		#[transactional]
		pub fn cancel_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if let Some(task) = Self::get_task(&who, &task_id) {
				Self::remove_task(
					&task,
					Some(Event::TaskCancelled {
						who: task.owner_id.clone(),
						task_id: task.task_id.clone(),
					}),
				);
			} else {
				Err(Error::<T>::TaskNotFound)?
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn generate_task_id() -> TaskId {
			let current_block_number =
				match TryInto::<u64>::try_into(<frame_system::Pallet<T>>::block_number()).ok() {
					Some(i) => i,
					None => 0,
				};

			let tx_id = match <frame_system::Pallet<T>>::extrinsic_index() {
				Some(i) => i,
				None => 0,
			};

			let evt_index = <frame_system::Pallet<T>>::event_count();

			format!("{:}-{:}-{:}", current_block_number, tx_id, evt_index)
				.as_bytes()
				.to_vec()
		}

		// Move task from the SortedTasksIndex into TaskQueue that are ready to be process
		pub fn shift_tasks(max_weight: Weight) -> Weight {
			let weight_left: Weight = max_weight;

			// TODO: Look into asset that has price move instead
			let ref mut task_to_process: TaskIdList<T> = Vec::new();

			for key in SortedTasksIndex::<T>::iter_keys() {
				let (chain, exchange, asset_pair, trigger_func) = key.clone();

				// TODO: Swap asset to check pair
				let current_price_wrap =
					Self::get_asset_price_data((&chain, &exchange, &asset_pair));

				if current_price_wrap.is_none() {
					continue
				};
				// Example: sell orders
				//
				// In the list we had tasks such as
				//  - task1: sell when price > 10
				//  - task2: sell when price > 20
				//  - task3: sell when price > 30
				//  If price used to be 5, and now it's 15, task1 got run
				//  If price used to be 5, and now it's 25, task1 and task2 got run
				//  If price used to be 5, and now it's 35, all tasks are run
				//
				// Example: buy orders
				//
				// In the list we had tasks such as
				//  - task1: buy when price < 10
				//  - task2: buy when price < 20
				//  - task3: buy when price < 30
				//  If price used to be 500, and now it's 25, task3 got run
				//  If price used to be 500, and now it's 15, task2 and task3 got run
				//  If price used to be 500, and now it's 5,  all tasks are run
				//
				//  TODO: handle atomic and transaction
				if let Some(mut tasks) = Self::get_sorted_tasks_index(&key) {
					let current_price = current_price_wrap.unwrap();

					//Eg sell order, sell when price >
					let range;
					if trigger_func == TRIGGER_FUNC_GT.to_vec() {
						range = (Excluded(&u128::MIN), Excluded(&current_price.amount))
					} else {
						// Eg buy order, buy when price <
						range = (Included(&current_price.amount), Excluded(&u128::MAX))
					};

					for (&price, task_ids) in (tasks.clone()).range(range) {
						// Remove because we map this into task queue
						tasks.remove(&price);
						let ref mut t = &mut (task_ids.clone());
						task_to_process.append(t);
					}

					// all tasks are moved to process, delete the queue
					if tasks.is_empty() {
						SortedTasksIndex::<T>::remove(&key);
					} else {
						SortedTasksIndex::<T>::insert(&key, tasks);
					}
				}
			}

			if !task_to_process.is_empty() {
				if TaskQueue::<T>::exists() {
					let mut old_task = TaskQueue::<T>::get();
					old_task.append(task_to_process);
					TaskQueue::<T>::put(old_task);
				} else {
					TaskQueue::<T>::put(task_to_process);
				};
			}

			return weight_left
		}

		/// Trigger tasks for the block time.
		///
		/// Complete as many tasks as possible given the maximum weight.
		pub fn trigger_tasks(max_weight: Weight) -> Weight {
			let mut weight_left: Weight = max_weight;
			let check_time_and_deletion_weight = T::DbWeight::get().reads(2u64);
			if weight_left.ref_time() < check_time_and_deletion_weight.ref_time() {
				return weight_left
			}

			Self::shift_tasks(weight_left);

			// Now we can run those tasks
			// TODO: We need to calculate enough weight and balance the tasks so we won't be skew
			// by a particular kind of task asset
			//
			// Now we run as much task as possible
			// If weight is over, task will be picked up next time
			// If the price is no longer matched, they will be put back into the TaskRegistry
			let task_queue = Self::get_task_queue();

			weight_left = weight_left
				// for above read
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

		pub fn create_new_asset(
			chain: ChainName,
			exchange: Exchange,
			asset1: AssetName,
			asset2: AssetName,
			decimal: u8,
			asset_owners: Vec<AccountOf<T>>,
		) -> Result<(), DispatchError> {
			let key = (&chain, &exchange, (&asset1, &asset2));

			if AssetRegistry::<T>::contains_key(&key) {
				Err(Error::<T>::AssetAlreadyInitialized)?
			}

			let asset_info = RegistryInfo::<T> {
				decimal,
				round: 0,
				last_update: 0,
				oracle_providers: asset_owners,
			};

			AssetRegistry::<T>::insert(key, asset_info);

			Self::deposit_event(Event::AssetCreated { chain, exchange, asset1, asset2, decimal });
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

		// return epoch time of current block
		pub fn get_current_block_time() -> Result<UnixTime, DispatchError> {
			let now = <timestamp::Pallet<T>>::get()
				.checked_into::<UnixTime>()
				.ok_or(ArithmeticError::Overflow)?;

			if now == 0 {
				Err(Error::<T>::BlockTimeNotSet)?;
			}

			let now = now.checked_div(1000).ok_or(ArithmeticError::Overflow)?;
			Ok(now)
		}

		// Check whether a task can run or not based on its expiration and price.
		//
		// A task can be queued but got expired when it's about to run, in that case, we don't want
		// it to be run.
		//
		// Or the price might move by the time task is invoked, we don't want it to get run either.
		fn task_can_run(task: &Task<T>) -> (Option<TaskCondition>, Weight) {
			let mut consumed_weight: Weight = Weight::from_ref_time(0);

			// If we cannot extract time from the block, then somthing horrible wrong, let not move
			// forward
			let current_block_time = Self::get_current_block_time();
			if current_block_time.is_err() {
				return (None, consumed_weight)
			}

			let now = current_block_time.unwrap();

			if task.expired_at < now.into() {
				consumed_weight =
					consumed_weight.saturating_add(<T as Config>::WeightInfo::emit_event());

				Self::deposit_event(Event::TaskExpired {
					who: task.owner_id.clone(),
					task_id: task.task_id.clone(),
					condition: TaskCondition::AlreadyExpired {
						expired_at: task.expired_at,
						now: now.into(),
					},
				});

				return (None, consumed_weight)
			}

			// read storage once to get the price
			consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1u64));
			if let Some(this_task_asset_price) =
				Self::get_asset_price_data((&task.chain, &task.exchange, &task.asset_pair))
			{
				// trigger when target price > current price of the asset
				// Example:
				//  - current price: 100, the task is has target price: 50  -> runable
				//  - current price: 100, the task is has target price: 150 -> not runable
				//
				let price_matched_target_condtion =
					if task.trigger_function == TRIGGER_FUNC_GT.to_vec() {
						task.trigger_params[0] < this_task_asset_price.amount
					} else {
						task.trigger_params[0] > this_task_asset_price.amount
					};

				if price_matched_target_condtion {
					return (
						Some(TaskCondition::TargetPriceMatched {
							chain: task.chain.clone(),
							exchange: task.exchange.clone(),
							asset_pair: task.asset_pair.clone(),
							price: this_task_asset_price.amount,
						}),
						consumed_weight,
					)
				} else {
					Self::deposit_event(Event::PriceAlreadyMoved {
						who: task.owner_id.clone(),
						task_id: task.task_id.clone(),
						condition: TaskCondition::PriceAlreadyMoved {
							chain: task.chain.clone(),
							exchange: task.exchange.clone(),
							asset_pair: task.asset_pair.clone(),
							price: this_task_asset_price.amount,

							target_price: task.trigger_params[0],
						},
					});

					return (None, consumed_weight)
				}
			}

			// This happen because we cannot find the price, so the task cannot be run
			(None, consumed_weight)
		}

		/// Runs as many tasks as the weight allows from the provided vec of task_ids.
		///
		/// Returns a vec with the tasks that were not run and the remaining weight.
		pub fn run_tasks(
			mut task_ids: TaskIdList<T>,
			mut weight_left: Weight,
		) -> (TaskIdList<T>, Weight) {
			let mut consumed_task_index: usize = 0;

			// If we cannot extract time from the block, then somthing horrible wrong, let not move
			// forward
			let current_block_time = Self::get_current_block_time();
			if current_block_time.is_err() {
				return (task_ids, weight_left)
			}

			let now = current_block_time.unwrap();

			for (owner_id, task_id) in task_ids.iter() {
				consumed_task_index.saturating_inc();

				let action_weight = match Self::get_task(&owner_id, &task_id) {
					None => {
						Self::deposit_event(Event::TaskNotFound {
							who: owner_id.clone(),
							task_id: task_id.clone(),
						});
						<T as Config>::WeightInfo::emit_event()
					},
					Some(task) => {
						let (task_condition, test_can_run_weight) = Self::task_can_run(&task);

						if task_condition.is_none() {
							test_can_run_weight
						} else {
							Self::deposit_event(Event::TaskTriggered {
								who: task.owner_id.clone(),
								task_id: task.task_id.clone(),
								condition: task_condition.unwrap(),
							});

							let total_task =
								Self::get_task_stat(StatType::TotalTasksOverall).map_or(0, |v| v);
							let total_task_per_account = Self::get_account_stat(
								&task.owner_id,
								StatType::TotalTasksPerAccount,
							)
							.map_or(0, |v| v);

							let (task_action_weight, task_dispatch_error) =
								match task.action.clone() {
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
								};

							Self::remove_task(&task, None);

							if let Some(err) = task_dispatch_error {
								Self::deposit_event(Event::<T>::TaskExecutionFailed {
									who: task.owner_id.clone(),
									task_id: task.task_id.clone(),
									error: err,
								});
							} else {
								Self::deposit_event(Event::<T>::TaskExecuted {
									who: task.owner_id.clone(),
									task_id: task.task_id.clone(),
								});
							}

							Self::deposit_event(Event::<T>::TaskCompleted {
								who: task.owner_id.clone(),
								task_id: task.task_id.clone(),
							});

							task_action_weight
								.saturating_add(T::DbWeight::get().writes(1u64))
								.saturating_add(T::DbWeight::get().reads(1u64))
						}
					},
				};

				weight_left = weight_left.saturating_sub(action_weight);

				let run_another_task_weight = <T as Config>::WeightInfo::emit_event()
					.saturating_add(T::DbWeight::get().writes(1u64))
					.saturating_add(T::DbWeight::get().reads(1u64));
				if weight_left.ref_time() < run_another_task_weight.ref_time() {
					break
				}
			}

			if consumed_task_index == task_ids.len() {
				(vec![], weight_left)
			} else {
				(task_ids.split_off(consumed_task_index), weight_left)
			}
		}

		// Handle task removal. There are a few places task need to be remove:
		//  - Tasks storage
		//  - TaskQueue if the task is already queued
		//  - TaskStats: decrease task count
		//  - AccountStats: decrease task count
		//  - SortedTasksIndex: sorted task by price
		//  - SortedTasksByExpiration: sorted task by expired epch
		pub fn remove_task(task: &Task<T>, event: Option<Event<T>>) {
			Tasks::<T>::remove(task.owner_id.clone(), task.task_id.clone());

			// Remove it from SortedTasksIndex
			let key = (&task.chain, &task.exchange, &task.asset_pair, &task.trigger_function);
			if let Some(mut sorted_tasks_by_price) = Self::get_sorted_tasks_index(&key) {
				if let Some(tasks) = sorted_tasks_by_price.get_mut(&task.trigger_params[0]) {
					if let Some(pos) = tasks.iter().position(|x| {
						let (_, task_id) = x;
						*task_id == task.task_id
					}) {
						tasks.remove(pos);
					}

					if tasks.is_empty() {
						// if there is no more task on this slot, clear it up
						sorted_tasks_by_price.remove(&task.trigger_params[0].clone());
					}
					SortedTasksIndex::<T>::insert(&key, sorted_tasks_by_price);
				}
			}

			// Remove it from the SortedTasksByExpiration
			SortedTasksByExpiration::<T>::mutate(|sorted_tasks_by_expiration| {
				if let Some(expired_task_slot) =
					sorted_tasks_by_expiration.get_mut(&task.expired_at)
				{
					expired_task_slot.remove(&task.task_id);
					if expired_task_slot.is_empty() {
						sorted_tasks_by_expiration.remove(&task.expired_at);
					}
				}
			});

			// Update metrics
			let total_task = Self::get_task_stat(StatType::TotalTasksOverall).map_or(0, |v| v);
			let total_task_per_account =
				Self::get_account_stat(&task.owner_id, StatType::TotalTasksPerAccount)
					.map_or(0, |v| v);

			if total_task >= 1 {
				TaskStats::<T>::insert(StatType::TotalTasksOverall, total_task - 1);
			}

			if total_task_per_account >= 1 {
				AccountStats::<T>::insert(
					task.owner_id.clone(),
					StatType::TotalTasksPerAccount,
					total_task_per_account - 1,
				);
			}

			if event.is_some() {
				Self::deposit_event(event.unwrap());
			}
		}

		// Sweep as mucht ask we can and return the remaining weight
		pub fn sweep_expired_task(remaining_weight: Weight) -> Weight {
			if remaining_weight.ref_time() <= T::DbWeight::get().reads(1u64).ref_time() {
				// Weight too low, not enough to do anything useful
				return remaining_weight
			}

			let current_block_time = Self::get_current_block_time();

			if current_block_time.is_err() {
				// Cannot get time, this probably is the first block
				return remaining_weight
			}

			let now = current_block_time.unwrap() as u128;

			// At the end we will most likely need to write back the updated storage, so here we
			// account for that write
			let mut unused_weight = remaining_weight
				.saturating_sub(T::DbWeight::get().reads(1u64))
				.saturating_sub(T::DbWeight::get().writes(1u64));
			let mut tasks_by_expiration = Self::get_sorted_tasks_by_expiration();

			let mut expired_shards: Vec<u128> = vec![];
			// Use Included(now) because if this task has not run at the end of this block, then
			// that mean at next block it for sure will expired
			'outer: for (expired_time, task_ids) in
				tasks_by_expiration.range_mut((Included(&0_u128), Included(&now)))
			{
				for (task_id, owner_id) in task_ids.iter() {
					if unused_weight.ref_time() >
						T::DbWeight::get()
							.reads(1u64)
							.saturating_add(<T as Config>::WeightInfo::remove_task())
							.ref_time()
					{
						unused_weight = unused_weight
							.saturating_sub(T::DbWeight::get().reads(1u64))
							.saturating_sub(<T as Config>::WeightInfo::remove_task());

						// Now let remove the task from chain storage
						if let Some(task) = Self::get_task(owner_id, task_id) {
							Self::remove_task(
								&task,
								Some(Event::TaskSweep {
									who: task.owner_id.clone(),
									task_id: task.task_id.clone(),
									condition: TaskCondition::AlreadyExpired {
										expired_at: task.expired_at,
										now: now.into(),
									},
								}),
							);
						}
					} else {
						// If there is not enough weight left, break all the way out, we had
						// already save one weight for the write to update storage back
						break 'outer
					}
				}
				expired_shards.push(expired_time.clone());
			}

			unused_weight
		}

		// Task is write into a sorted storage, re-present by BTreeMap so we can find and expired them
		pub fn track_expired_task(task: &Task<T>) -> Result<bool, Error<T>> {
			// first we got back the reference to the underlying storage
			// perform relevant update to write task to the right shard by expired
			// time, then the value is store back to storage
			let mut tasks_by_expiration = Self::get_sorted_tasks_by_expiration();

			if let Some(task_shard) = tasks_by_expiration.get_mut(&task.expired_at) {
				task_shard.insert(task.task_id.clone(), task.owner_id.clone());
			} else {
				tasks_by_expiration.insert(
					task.expired_at.clone(),
					BTreeMap::from([(task.task_id.clone(), task.owner_id.clone())]),
				);
			}
			SortedTasksByExpiration::<T>::put(tasks_by_expiration);

			return Ok(true)
		}

		/// With transaction will protect against a partial success where N of M execution times might be full,
		/// rolling back any successful insertions into the schedule task table.
		/// Validate and schedule task.
		/// This will also charge the execution fee.
		/// TODO: double check atomic
		pub fn validate_and_schedule_task(task: Task<T>) -> Result<(), Error<T>> {
			if task.task_id.is_empty() {
				Err(Error::<T>::InvalidTaskId)?
			}

			let current_block_time = Self::get_current_block_time();
			if current_block_time.is_err() {
				// Cannot get time, this probably is the first block
				Err(Error::<T>::BlockTimeNotSet)?
			}

			let now = current_block_time.unwrap() as u128;

			if task.expired_at <= now {
				Err(Error::<T>::InvalidTaskExpiredAt)?
			}

			let total_task = Self::get_task_stat(StatType::TotalTasksOverall).map_or(0, |v| v);
			let total_task_per_account =
				Self::get_account_stat(&task.owner_id, StatType::TotalTasksPerAccount)
					.map_or(0, |v| v);

			// check task total limit per account and overall
			if total_task >= T::MaxTasksOverall::get().into() {
				Err(Error::<T>::MaxTasksReached)?
			}

			// check task total limit per account and overall
			if total_task_per_account >= T::MaxTasksPerAccount::get().into() {
				Err(Error::<T>::MaxTasksPerAccountReached)?
			}

			Tasks::<T>::insert(task.owner_id.clone(), task.task_id.clone(), &task);
			// Post task processing, increase relevant metrics data
			TaskStats::<T>::insert(StatType::TotalTasksOverall, total_task + 1);
			AccountStats::<T>::insert(
				task.owner_id.clone(),
				StatType::TotalTasksPerAccount,
				total_task_per_account + 1,
			);

			let key = (&task.chain, &task.exchange, &task.asset_pair, &task.trigger_function);

			if let Some(mut sorted_task_index) = Self::get_sorted_tasks_index(key) {
				// TODO: remove hard code and take right param
				if let Some(tasks_by_price) = sorted_task_index.get_mut(&(task.trigger_params[0])) {
					tasks_by_price.push((task.owner_id.clone(), task.task_id.clone()));
				} else {
					sorted_task_index.insert(
						task.trigger_params[0],
						vec![(task.owner_id.clone(), task.task_id.clone())],
					);
				}
				SortedTasksIndex::<T>::insert(key, sorted_task_index);
			} else {
				let mut sorted_task_index = BTreeMap::<AssetPrice, TaskIdList<T>>::new();
				sorted_task_index.insert(
					task.trigger_params[0],
					vec![(task.owner_id.clone(), task.task_id.clone())],
				);

				// TODO: sorted based on trigger_function comparison of the parameter
				// then at the time of trigger we cut off all the left part of the tree
				SortedTasksIndex::<T>::insert(key, sorted_task_index);
			}

			if let Err(_) = Self::track_expired_task(&task) {
				Err(Error::<T>::TaskExpiredStorageFailedToUpdate)?
			}

			Self::deposit_event(Event::TaskScheduled {
				who: task.owner_id.clone(),
				task_id: task.task_id,
			});

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
