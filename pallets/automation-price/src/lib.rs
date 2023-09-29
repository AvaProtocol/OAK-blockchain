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

pub mod types;
pub use types::*;

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
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement},
	transactional,
};
use frame_system::pallet_prelude::*;
use orml_traits::{FixedConversionRateProvider, MultiCurrency};
use pallet_timestamp::{self as timestamp};
use scale_info::{prelude::format, TypeInfo};
use sp_runtime::{
	traits::{Convert, SaturatedConversion, Saturating},
	Perbill,
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
	pub type TaskIdList = Vec<TaskId>;

	// TODO: Cleanup before merge
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
		BTreeMap<AssetPrice, TaskIdList>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_scheduled_asset_period_reset)]
	pub type ScheduledAssetDeletion<T: Config> =
		StorageMap<_, Twox64Concat, UnixTime, Vec<AssetName>>;

	// Tasks hold all active task, look up through (TaskId)
	#[pallet::storage]
	#[pallet::getter(fn get_task)]
	pub type Tasks<T: Config> = StorageMap<_, Twox64Concat, TaskId, Task<T>>;

	// All active tasks, but organized by account
	// In this storage, we only interested in returning task belong to an account, we also want to
	// have fast lookup for task inserted/remove into the storage
	//
	// We also want to remove the expired task, so by leveraging this
	#[pallet::storage]
	#[pallet::getter(fn get_account_task_ids)]
	pub type AccountTasks<T: Config> =
		StorageDoubleMap<_, Twox64Concat, AccountOf<T>, Twox64Concat, TaskId, u128>;

	// TaskQueue stores the task to be executed. To run any tasks, they need to be move into this
	// queue, from there our task execution pick it up and run it
	//
	// When task is run, we check the price once more and if it fall out of range, we move the task
	// back to the Tasks Registry
	//
	// If the task is expired, we also won't run
	#[pallet::storage]
	#[pallet::getter(fn get_task_queue)]
	pub type TaskQueue<T: Config> = StorageValue<_, TaskIdList, ValueQuery>;

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
		/// Failed to insert task
		TaskInsertionFailure,
		/// Failed to remove task
		TaskRemoveFailure,
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
		TaskCancelled {
			who: AccountOf<T>,
			task_id: TaskId,
		},
		Notify {
			message: Vec<u8>,
		},
		TaskNotFound {
			task_id: TaskId,
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
			asset: AssetName,
		},
		AssetPeriodReset {
			asset: AssetName,
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
			_origin: OriginFor<T>,
			chain: Vec<u8>,
			exchange: Vec<u8>,
			asset1: AssetName,
			asset2: AssetName,
			decimal: u8,
			asset_owners: Vec<AccountOf<T>>,
		) -> DispatchResult {
			// TODO: needs fees if opened up to non-sudo
			// temporary comment out for easiser development
			//ensure_root(origin)?;
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

		/// Delete an asset
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
			_origin: OriginFor<T>,
			chain: ChainName,
			exchange: Exchange,
			asset1: AssetName,
			asset2: AssetName,
		) -> DispatchResult {
			// TODO: needs fees if opened up to non-sudo
			// TODO: add a feature flag so we can toggle in dev build without sudo
			//ensure_root(origin)?;

			let key = (chain, exchange, (&asset1, &asset2));

			// TODO: handle delete
			if let Some(_asset_target_price) = Self::get_asset_registry_info(key) {
				//Self::delete_asset_tasks(asset.clone());
				Self::deposit_event(Event::AssetDeleted { asset: asset1 });
			} else {
				Err(Error::<T>::AssetNotSupported)?
			}
			Ok(())
		}

		// TODO: correct weight
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

			if let Some(task) = Self::get_task(task_id) {
				if task.owner_id != who {
					// TODO: Fine tune error
					Err(Error::<T>::TaskRemoveFailure)?
				}

				Tasks::<T>::remove(&task.task_id);
				let key = (&task.chain, &task.exchange, &task.asset_pair, &task.trigger_function);
				SortedTasksIndex::<T>::remove(&key);
				Self::remove_task_from_account(&task);
				Self::deposit_event(Event::TaskCancelled {
					who: task.owner_id.clone(),
					task_id: task.task_id.clone(),
				});
			};

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
			let ref mut task_to_process: TaskIdList = Vec::new();

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
					// TODO: move magic number into a trigger.rs module
					if trigger_func == vec![103_u8, 116_u8] {
						range = (Excluded(&u128::MIN), Included(&current_price.amount))
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

		/// Runs as many tasks as the weight allows from the provided vec of task_ids.
		///
		/// Returns a vec with the tasks that were not run and the remaining weight.
		pub fn run_tasks(
			mut task_ids: Vec<TaskId>,
			mut weight_left: Weight,
		) -> (Vec<TaskId>, Weight) {
			let mut consumed_task_index: usize = 0;
			for task_id in task_ids.iter() {
				consumed_task_index.saturating_inc();

				// TODO: re-check condition here once more time because the price might have been
				// more
				// if the task is already expired, don't run them either

				let action_weight = match Self::get_task(task_id) {
					None => {
						// TODO: add back signature when insert new task work
						//Self::deposit_event(Event::TaskNotFound { task_id: task_id.clone() });
						<T as Config>::WeightInfo::emit_event()
					},
					Some(task) => {
						Self::deposit_event(Event::TaskTriggered {
							who: task.owner_id.clone(),
							task_id: task.task_id.clone(),
						});

						let (task_action_weight, task_dispatch_error) = match task.action.clone() {
							// TODO: Run actual task later to return weight
							// not just return weight for test to pass
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

						Tasks::<T>::remove(task_id);

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

						// TODO: add this weight
						Self::remove_task_from_account(&task);

						Self::deposit_event(Event::<T>::TaskCompleted {
							who: task.owner_id.clone(),
							task_id: task.task_id.clone(),
						});

						task_action_weight
							.saturating_add(T::DbWeight::get().writes(1u64))
							.saturating_add(T::DbWeight::get().reads(1u64))
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

		fn push_task_to_account(task: &Task<T>) {
			AccountTasks::<T>::insert(task.owner_id.clone(), task.task_id.clone(), task.expired_at);
		}

		fn remove_task_from_account(task: &Task<T>) {
			AccountTasks::<T>::remove(task.owner_id.clone(), task.task_id.clone());
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

			<Tasks<T>>::insert(task.task_id.clone(), &task);
			Self::push_task_to_account(&task);

			let key = (&task.chain, &task.exchange, &task.asset_pair, &task.trigger_function);

			if let Some(mut sorted_task_index) = Self::get_sorted_tasks_index(key) {
				// TODO: remove hard code and take right param
				if let Some(tasks_by_price) = sorted_task_index.get_mut(&(task.trigger_params[0])) {
					tasks_by_price.push(task.task_id.clone());
				} else {
					sorted_task_index.insert(task.trigger_params[0], vec![task.task_id.clone()]);
				}
				SortedTasksIndex::<T>::insert(key, sorted_task_index);
			} else {
				let mut sorted_task_index = BTreeMap::<AssetPrice, TaskIdList>::new();
				sorted_task_index.insert(task.trigger_params[0], vec![task.task_id.clone()]);

				// TODO: sorted based on trigger_function comparison of the parameter
				// then at the time of trigger we cut off all the left part of the tree
				SortedTasksIndex::<T>::insert(key, sorted_task_index);
			}

			Self::deposit_event(Event::TaskScheduled {
				who: task.owner_id,
				task_id: task.task_id.clone(),
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
