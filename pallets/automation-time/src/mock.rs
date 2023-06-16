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
use frame_benchmarking::frame_support::assert_ok;
use frame_support::{
	construct_runtime, parameter_types, traits::Everything, weights::Weight, PalletId,
};
use frame_system::{self as system, RawOrigin};
use orml_traits::parameter_type_with_key;
use primitives::EnsureProxy;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, CheckedSub, Convert, IdentityLookup},
	AccountId32, DispatchError, Perbill,
};
use sp_std::marker::PhantomData;
use xcm::latest::prelude::*;

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

pub type Balance = u128;
pub type AccountId = AccountId32;
pub type CurrencyId = u32;

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const PARA_ID: u32 = 2000;
pub const NATIVE: CurrencyId = 0;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call},
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
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
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
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}
parameter_types! {
	pub DustAccount: AccountId = PalletId(*b"auto/dst").into_account_truncating();
}

impl orml_tokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = i64;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = ConstU32<100_000>;
	type MaxReserves = ConstU32<100_000>;
	type ReserveIdentifier = [u8; 8];
	type DustRemovalWhitelist = frame_support::traits::Nothing;
}

impl orml_currencies::Config for Test {
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}
pub type AdaptedBasicCurrency = orml_currencies::BasicCurrencyAdapter<Test, Balances, i64, u64>;

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
	) -> Result<bool, DispatchError> {
		let delegation: u128 = amount.saturated_into();
		C::reserve(delegator, delegation.saturated_into())?;
		Ok(true)
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
	pub const MaxBlockWeight: u64 = 1_000_000;
	pub const MaxWeightPercentage: Perbill = Perbill::from_percent(10);
	pub const UpdateQueueRatio: Perbill = Perbill::from_percent(50);
	pub const ExecutionWeightFee: Balance = 12;
	pub const MaxWeightPerSlot: u128 = 70_000_000;
	pub const XmpFee: u128 = 1_000_000;
	pub const GetNativeCurrencyId: CurrencyId = NATIVE;
}

pub struct MockWeight<T>(PhantomData<T>);
impl<Test: frame_system::Config> pallet_automation_time::WeightInfo for MockWeight<Test> {
	fn schedule_notify_task_empty() -> Weight {
		Weight::zero()
	}
	fn schedule_notify_task_full(_v: u32) -> Weight {
		Weight::zero()
	}
	fn schedule_native_transfer_task_empty() -> Weight {
		Weight::zero()
	}
	fn schedule_native_transfer_task_full(_v: u32) -> Weight {
		Weight::zero()
	}
	fn schedule_auto_compound_delegated_stake_task_full() -> Weight {
		Weight::zero()
	}
	fn schedule_dynamic_dispatch_task(_v: u32) -> Weight {
		Weight::zero()
	}
	fn schedule_dynamic_dispatch_task_full(_v: u32) -> Weight {
		Weight::zero()
	}
	fn schedule_xcmp_task_full(_v: u32) -> Weight {
		Weight::zero()
	}
	fn cancel_scheduled_task_full() -> Weight {
		Weight::zero()
	}
	fn force_cancel_scheduled_task() -> Weight {
		Weight::zero()
	}
	fn force_cancel_scheduled_task_full() -> Weight {
		Weight::zero()
	}
	fn run_notify_task() -> Weight {
		Weight::from_ref_time(20_000)
	}
	fn run_native_transfer_task() -> Weight {
		Weight::from_ref_time(20_000)
	}
	fn run_xcmp_task() -> Weight {
		Weight::from_ref_time(20_000)
	}
	fn run_auto_compound_delegated_stake_task() -> Weight {
		Weight::from_ref_time(20_000)
	}
	fn run_dynamic_dispatch_action() -> Weight {
		Weight::from_ref_time(20_000)
	}
	fn run_dynamic_dispatch_action_fail_decode() -> Weight {
		Weight::from_ref_time(20_000)
	}
	fn run_missed_tasks_many_found(v: u32) -> Weight {
		Weight::from_ref_time(10_000 * v as u64)
	}
	fn run_missed_tasks_many_missing(v: u32) -> Weight {
		Weight::from_ref_time(10_000 * v as u64)
	}
	fn run_tasks_many_found(v: u32) -> Weight {
		Weight::from_ref_time(50_000 * v as u64)
	}
	fn run_tasks_many_missing(v: u32) -> Weight {
		Weight::from_ref_time(10_000 * v as u64)
	}
	fn update_task_queue_overhead() -> Weight {
		Weight::from_ref_time(10_000)
	}
	fn append_to_missed_tasks(v: u32) -> Weight {
		Weight::from_ref_time(20_000 * v as u64)
	}
	fn update_scheduled_task_queue() -> Weight {
		Weight::from_ref_time(20_000)
	}
	fn shift_missed_tasks() -> Weight {
		Weight::from_ref_time(20_000)
	}
	fn migration_upgrade_weight_struct(_: u32) -> Weight {
		Weight::zero()
	}
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
		_location: xcm::latest::MultiLocation,
		_caller: T::AccountId,
		_transact_encoded_call: sp_std::vec::Vec<u8>,
		_transact_encoded_call_weight: Weight,
	) -> Result<(), sp_runtime::DispatchError> {
		Ok(().into())
	}

	fn get_xcm_fee(
		_destination: u32,
		_xcm_asset_location: xcm::latest::MultiLocation,
		_transact_encoded_call_weight: Weight,
	) -> Result<u128, sp_runtime::DispatchError> {
		Ok(XmpFee::get())
	}

	fn pay_xcm_fee(_: T::AccountId, _: u128) -> Result<(), sp_runtime::DispatchError> {
		Ok(().into())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn setup_chain_asset_data(
		_asset_location: xcm::latest::MultiLocation,
	) -> Result<(), sp_runtime::DispatchError> {
		Ok(().into())
	}
}

pub struct ScheduleAllowList;
impl Contains<RuntimeCall> for ScheduleAllowList {
	fn contains(c: &RuntimeCall) -> bool {
		match c {
			RuntimeCall::System(_) => true,
			_ => false,
		}
	}
}

pub struct MockConversionRateProvider;
impl FixedConversionRateProvider for MockConversionRateProvider {
	fn get_fee_per_second(_location: &MultiLocation) -> Option<u128> {
		Some(1)
	}
}

pub struct MockTokenIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for MockTokenIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		if id == NATIVE {
			Some(MultiLocation::new(1, Here))
		} else if id == 1 {
			Some(MultiLocation::new(1, X1(Parachain(2110))))
		} else {
			None
		}
	}
}

pub struct MockEnsureProxy;
impl EnsureProxy<AccountId> for MockEnsureProxy {
	fn ensure_ok(_delegator: AccountId, _delegatee: AccountId) -> Result<(), &'static str> {
		Ok(())
	}
}

impl pallet_automation_time::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxTasksPerSlot = MaxTasksPerSlot;
	type MaxExecutionTimes = MaxExecutionTimes;
	type MaxScheduleSeconds = MaxScheduleSeconds;
	type MaxBlockWeight = MaxBlockWeight;
	type MaxWeightPercentage = MaxWeightPercentage;
	type UpdateQueueRatio = UpdateQueueRatio;
	type WeightInfo = MockWeight<Test>;
	type ExecutionWeightFee = ExecutionWeightFee;
	type MaxWeightPerSlot = MaxWeightPerSlot;
	type Currency = Balances;
	type MultiCurrency = Currencies;
	type CurrencyId = CurrencyId;
	type FeeHandler = FeeHandler<Test, ()>;
	type DelegatorActions = MockDelegatorActions<Test, Balances>;
	type XcmpTransactor = MockXcmpTransactor<Test, Balances>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type Call = RuntimeCall;
	type ScheduleAllowList = ScheduleAllowList;
	type CurrencyIdConvert = MockTokenIdConvert;
	type FeeConversionRateProvider = MockConversionRateProvider;
	type EnsureProxy = MockEnsureProxy;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(state_block_time: u64) -> sp_io::TestExternalities {
	let t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext.execute_with(|| Timestamp::set_timestamp(state_block_time));
	ext
}

pub fn schedule_task(
	owner: [u8; 32],
	provided_id: Vec<u8>,
	scheduled_times: Vec<u64>,
	message: Vec<u8>,
) -> sp_core::H256 {
	get_funds(AccountId32::new(owner));
	let task_hash_input = TaskHashInput::new(AccountId32::new(owner), provided_id.clone());
	assert_ok!(AutomationTime::schedule_notify_task(
		RuntimeOrigin::signed(AccountId32::new(owner)),
		provided_id,
		scheduled_times,
		message,
	));
	BlakeTwo256::hash_of(&task_hash_input)
}

pub fn add_task_to_task_queue(
	owner: [u8; 32],
	provided_id: Vec<u8>,
	scheduled_times: Vec<u64>,
	action: ActionOf<Test>,
) -> sp_core::H256 {
	let schedule = Schedule::new_fixed_schedule::<Test>(scheduled_times).unwrap();
	add_to_task_queue(owner, provided_id, schedule, action)
}

pub fn add_recurring_task_to_task_queue(
	owner: [u8; 32],
	provided_id: Vec<u8>,
	scheduled_time: u64,
	frequency: u64,
	action: ActionOf<Test>,
) -> sp_core::H256 {
	let schedule = Schedule::new_recurring_schedule::<Test>(scheduled_time, frequency).unwrap();
	add_to_task_queue(owner, provided_id, schedule, action)
}

pub fn add_to_task_queue(
	owner: [u8; 32],
	provided_id: Vec<u8>,
	schedule: Schedule,
	action: ActionOf<Test>,
) -> sp_core::H256 {
	let task_id = create_task(owner, provided_id, schedule, action);
	let mut task_queue = AutomationTime::get_task_queue();
	task_queue.push((AccountId32::new(owner), task_id));
	TaskQueueV2::<Test>::put(task_queue);
	task_id
}

pub fn add_task_to_missed_queue(
	owner: [u8; 32],
	provided_id: Vec<u8>,
	scheduled_times: Vec<u64>,
	action: ActionOf<Test>,
) -> sp_core::H256 {
	let schedule = Schedule::new_fixed_schedule::<Test>(scheduled_times.clone()).unwrap();
	let task_id = create_task(owner, provided_id, schedule, action);
	let missed_task =
		MissedTaskV2Of::<Test>::new(AccountId32::new(owner), task_id, scheduled_times[0]);
	let mut missed_queue = AutomationTime::get_missed_queue();
	missed_queue.push(missed_task);
	MissedQueueV2::<Test>::put(missed_queue);
	task_id
}

pub fn create_task(
	owner: [u8; 32],
	provided_id: Vec<u8>,
	schedule: Schedule,
	action: ActionOf<Test>,
) -> sp_core::H256 {
	let task_hash_input = TaskHashInput::new(AccountId32::new(owner), provided_id.clone());
	let task_id = BlakeTwo256::hash_of(&task_hash_input);
	let task = TaskOf::<Test>::new(owner.into(), provided_id, schedule, action);
	AccountTasks::<Test>::insert(AccountId::new(owner), task_id, task);
	task_id
}

pub fn events() -> Vec<RuntimeEvent> {
	let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}

pub fn last_event() -> RuntimeEvent {
	events().pop().unwrap()
}

pub fn get_funds(account: AccountId) {
	let double_action_weight = MockWeight::<Test>::run_native_transfer_task() * 2;
	let action_fee = ExecutionWeightFee::get() * u128::from(double_action_weight.ref_time());
	let max_execution_fee = action_fee * u128::from(MaxExecutionTimes::get());
	Balances::set_balance(RawOrigin::Root.into(), account, max_execution_fee, 0).unwrap();
}

pub fn get_minimum_funds(account: AccountId, executions: u32) {
	let double_action_weight = MockWeight::<Test>::run_native_transfer_task() * 2;
	let action_fee = ExecutionWeightFee::get() * u128::from(double_action_weight.ref_time());
	let max_execution_fee = action_fee * u128::from(executions);
	Balances::set_balance(RawOrigin::Root.into(), account, max_execution_fee, 0).unwrap();
}

pub fn get_xcmp_funds(account: AccountId) {
	let double_action_weight = MockWeight::<Test>::run_xcmp_task() * 2;
	let action_fee = ExecutionWeightFee::get() * u128::from(double_action_weight.ref_time());
	let max_execution_fee = action_fee * u128::from(MaxExecutionTimes::get());
	let with_xcm_fees = max_execution_fee + XmpFee::get();
	Balances::set_balance(RawOrigin::Root.into(), account, with_xcm_fees, 0).unwrap();
}

// TODO: swap above to this pattern
pub fn fund_account_dynamic_dispatch(
	account: &AccountId,
	execution_count: usize,
	encoded_call: Vec<u8>,
) -> Result<(), DispatchError> {
	let action: ActionOf<Test> = Action::DynamicDispatch { encoded_call };
	let action_weight = action.execution_weight::<Test>()?;
	fund_account(account, action_weight, execution_count, None);
	Ok(())
}

pub fn fund_account(
	account: &AccountId,
	action_weight: u64,
	execution_count: usize,
	additional_amount: Option<u128>,
) {
	let amount: u128 =
		u128::from(action_weight) * ExecutionWeightFee::get() * execution_count as u128 +
			additional_amount.unwrap_or(0) +
			u128::from(ExistentialDeposit::get());
	_ = <Test as Config>::Currency::deposit_creating(account, amount);
}
