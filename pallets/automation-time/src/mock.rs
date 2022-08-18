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

use super::*;
use crate as pallet_automation_time;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{Everything, OnUnbalanced},
	weights::Weight,
};
use frame_system as system;
use pallet_balances::NegativeImbalance;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, CheckedSub, IdentityLookup},
	AccountId32, Perbill,
};
use sp_std::marker::PhantomData;

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

pub type Balance = u128;
pub type AccountId = AccountId32;

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const PARA_ID: u32 = 2000;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		AutomationTime: pallet_automation_time::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 51;
}

impl system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1000;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

pub struct MockDelegatorActions<T, C>(PhantomData<(T, C)>);
impl<
		T: Config + pallet::Config<Currency = C>,
		C: frame_support::traits::ReservableCurrency<T::AccountId>,
	> pallet_parachain_staking::DelegatorActions<T::AccountId, BalanceOf<T>>
	for MockDelegatorActions<T, C>
{
	fn delegator_bond_more(
		delegator: &T::AccountId,
		_: &T::AccountId,
		amount: BalanceOf<T>,
	) -> DispatchResultWithPostInfo {
		let delegation: u128 = amount.saturated_into();
		C::reserve(delegator, delegation.saturated_into())?;
		Ok(().into())
	}
	fn delegator_bond_till_minimum(
		delegator: &T::AccountId,
		_: &T::AccountId,
		account_minimum: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchErrorWithPostInfo> {
		let delegation = C::free_balance(&delegator)
			.checked_sub(&account_minimum)
			.ok_or(Error::<T>::InsufficientBalance)?;
		C::reserve(delegator, delegation)?;
		Ok(delegation)
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn setup_delegator(_: &T::AccountId, _: &T::AccountId) -> DispatchResultWithPostInfo {
		Ok(().into())
	}
}

parameter_types! {
	pub const MaxTasksPerSlot: u32 = 2;
	#[derive(Debug)]
	pub const MaxExecutionTimes: u32 = 3;
	pub const MaxScheduleSeconds: u64 = 1 * 24 * 60 * 60;
	pub const MaxBlockWeight: Weight = 1_000_000;
	pub const MaxWeightPercentage: Perbill = Perbill::from_percent(10);
	pub const UpdateQueueRatio: Perbill = Perbill::from_percent(50);
	pub const SecondsPerBlock: u64 = 12;
	pub const ExecutionWeightFee: Balance = 12;
	pub const XmpFee: u128 = 1_000_000;
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Native;
}

pub struct MockWeight<T>(PhantomData<T>);
impl<Test: frame_system::Config> pallet_automation_time::WeightInfo for MockWeight<Test> {
	fn schedule_notify_task_empty() -> Weight {
		0
	}
	fn schedule_notify_task_full(_v: u32) -> Weight {
		0
	}
	fn schedule_native_transfer_task_empty() -> Weight {
		0
	}
	fn schedule_native_transfer_task_full(_v: u32) -> Weight {
		0
	}
	fn schedule_auto_compound_delegated_stake_task_full() -> Weight {
		0
	}
	fn schedule_xcmp_task_full(_v: u32) -> Weight {
		0
	}
	fn cancel_scheduled_task_full() -> Weight {
		0
	}
	fn force_cancel_scheduled_task() -> Weight {
		0
	}
	fn force_cancel_scheduled_task_full() -> Weight {
		0
	}
	fn run_notify_task() -> Weight {
		20_000
	}
	fn run_native_transfer_task() -> Weight {
		20_000
	}
	fn run_xcmp_task() -> Weight {
		20_000
	}
	fn run_auto_compound_delegated_stake_task() -> Weight {
		20_000
	}
	fn run_missed_tasks_many_found(v: u32) -> Weight {
		(10_000 * v).into()
	}
	fn run_missed_tasks_many_missing(v: u32) -> Weight {
		(10_000 * v).into()
	}
	fn run_tasks_many_found(v: u32) -> Weight {
		(50_000 * v).into()
	}
	fn run_tasks_many_missing(v: u32) -> Weight {
		(10_000 * v).into()
	}
	fn update_task_queue_overhead() -> Weight {
		10_000
	}
	fn append_to_missed_tasks(v: u32) -> Weight {
		(20_000 * v).into()
	}
	fn update_scheduled_task_queue() -> Weight {
		20_000
	}
	fn shift_missed_tasks() -> Weight {
		20_000
	}
}

#[derive(
	Encode,
	Decode,
	Deserialize,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Serialize,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum CurrencyId {
	Native,
	ROC,
	UNIT,
}

pub struct DealWithExecutionFees<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for DealWithExecutionFees<R>
where
	R: pallet_balances::Config,
{
	fn on_unbalanceds<B>(_fees: impl Iterator<Item = NegativeImbalance<R>>) {}
}

pub struct MockXcmpTransactor<T, C>(PhantomData<(T, C)>);
impl<T, C> pallet_xcmp_handler::XcmpTransactor<T::AccountId, CurrencyId>
	for MockXcmpTransactor<T, C>
where
	T: Config + pallet::Config<Currency = C>,
	C: frame_support::traits::ReservableCurrency<T::AccountId>,
{
	fn transact_xcm(
		_para_id: u32,
		_currency_id: CurrencyId,
		_caller: T::AccountId,
		_transact_encoded_call: sp_std::vec::Vec<u8>,
		_transact_encoded_call_weight: u64,
	) -> Result<(), sp_runtime::DispatchError> {
		Ok(().into())
	}

	fn get_xcm_fee(
		_para_id: u32,
		_currency_id: CurrencyId,
		_transact_encoded_call_weight: u64,
	) -> Result<u128, sp_runtime::DispatchError> {
		Ok(XmpFee::get())
	}

	fn pay_xcm_fee(_: T::AccountId, _: u128) -> Result<(), sp_runtime::DispatchError> {
		Ok(().into())
	}
}

impl pallet_automation_time::Config for Test {
	type Event = Event;
	type MaxTasksPerSlot = MaxTasksPerSlot;
	type MaxExecutionTimes = MaxExecutionTimes;
	type MaxScheduleSeconds = MaxScheduleSeconds;
	type MaxBlockWeight = MaxBlockWeight;
	type MaxWeightPercentage = MaxWeightPercentage;
	type UpdateQueueRatio = UpdateQueueRatio;
	type SecondsPerBlock = SecondsPerBlock;
	type WeightInfo = MockWeight<Test>;
	type ExecutionWeightFee = ExecutionWeightFee;
	type Currency = Balances;
	type CurrencyId = CurrencyId;
	type FeeHandler = FeeHandler<DealWithExecutionFees<Test>>;
	type DelegatorActions = MockDelegatorActions<Test, Balances>;
	type XcmpTransactor = MockXcmpTransactor<Test, Balances>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(state_block_time: u64) -> sp_io::TestExternalities {
	let t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext.execute_with(|| Timestamp::set_timestamp(state_block_time));
	ext
}
