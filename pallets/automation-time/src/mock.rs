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
use crate::TaskIdV2;

use frame_benchmarking::frame_support::assert_ok;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU128, ConstU32, Everything},
	weights::Weight,
	PalletId,
};
use frame_system::{self as system, EnsureRoot, RawOrigin};
use orml_traits::parameter_type_with_key;
use primitives::{EnsureProxy, TransferCallCreator};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, Convert, IdentityLookup},
	AccountId32, DispatchError, MultiAddress, Perbill,
};
use sp_std::{marker::PhantomData, vec::Vec};
use xcm::latest::prelude::*;

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

pub type Balance = u128;
pub type AccountId = AccountId32;
pub type CurrencyId = u32;

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const DELEGATOR_ACCOUNT: [u8; 32] = [3u8; 32];
pub const PROXY_ACCOUNT: [u8; 32] = [4u8; 32];
pub const COLLATOR_ACCOUNT: [u8; 32] = [5u8; 32];

pub const PARA_ID: u32 = 2000;
pub const NATIVE: CurrencyId = 0;
pub const NATIVE_LOCATION: MultiLocation = MultiLocation { parents: 0, interior: Here };
pub const NATIVE_EXECUTION_WEIGHT_FEE: u128 = 12;
pub const FOREIGN_CURRENCY_ID: CurrencyId = 1;

const DOLLAR: u128 = 10_000_000_000;

pub const MOONBASE_ASSET_LOCATION: MultiLocation =
	MultiLocation { parents: 1, interior: X2(Parachain(1000), PalletInstance(3)) };
pub const UNKNOWN_SCHEDULE_FEE: MultiLocation =
	MultiLocation { parents: 1, interior: X1(Parachain(4000)) };

pub struct MockAssetFeePerSecond {
	pub asset_location: MultiLocation,
	pub fee_per_second: u128,
}

pub const ASSET_FEE_PER_SECOND: [MockAssetFeePerSecond; 3] = [
	MockAssetFeePerSecond {
		asset_location: MultiLocation { parents: 1, interior: X1(Parachain(2000)) },
		fee_per_second: 416_000_000_000,
	},
	MockAssetFeePerSecond {
		asset_location: MultiLocation {
			parents: 1,
			interior: X2(Parachain(2110), GeneralKey { length: 4, data: [0; 32] }),
		},
		fee_per_second: 416_000_000_000,
	},
	MockAssetFeePerSecond {
		asset_location: MOONBASE_ASSET_LOCATION,
		fee_per_second: 10_000_000_000_000_000_000,
	},
];

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		ParachainInfo: parachain_info::{Pallet, Storage, Config},
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call},
		AutomationTime: pallet_automation_time::{Pallet, Call, Storage, Event<T>},
		ParachainStaking: pallet_parachain_staking::{Pallet, Call, Storage, Event<T>, Config<T>},
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
	//type RuntimeEvent = From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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

impl parachain_info::Config for Test {}

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
	/// Minimum stake required to become a collator
	pub const MinCollatorStk: u128 = 400_000 * DOLLAR;
	pub const MinimumPeriod: u64 = 1000;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

impl pallet_parachain_staking::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MonetaryGovernanceOrigin = EnsureRoot<AccountId>;
	/// Minimum round length is 2 minutes (10 * 12 second block times)
	type MinBlocksPerRound = ConstU32<10>;
	/// Rounds before the collator leaving the candidates request can be executed
	type LeaveCandidatesDelay = ConstU32<2>;
	/// Rounds before the candidate bond increase/decrease can be executed
	type CandidateBondLessDelay = ConstU32<2>;
	/// Rounds before the delegator exit can be executed
	type LeaveDelegatorsDelay = ConstU32<2>;
	/// Rounds before the delegator revocation can be executed
	type RevokeDelegationDelay = ConstU32<2>;
	/// Rounds before the delegator bond increase/decrease can be executed
	type DelegationBondLessDelay = ConstU32<2>;
	/// Rounds before the reward is paid
	type RewardPaymentDelay = ConstU32<2>;
	/// Minimum collators selected per round, default at genesis and minimum forever after
	type MinSelectedCandidates = ConstU32<5>;
	/// Maximum top delegations per candidate
	type MaxTopDelegationsPerCandidate = ConstU32<10>;
	/// Maximum bottom delegations per candidate
	type MaxBottomDelegationsPerCandidate = ConstU32<50>;
	/// Maximum delegations per delegator
	type MaxDelegationsPerDelegator = ConstU32<10>;
	type MinCollatorStk = MinCollatorStk;
	/// Minimum stake required to be reserved to be a candidate
	type MinCandidateStk = ConstU128<{ 500 * DOLLAR }>;
	/// Minimum delegation amount after initial
	type MinDelegation = ConstU128<{ 50 * DOLLAR }>;
	/// Minimum initial stake required to be reserved to be a delegator
	type MinDelegatorStk = ConstU128<{ 50 * DOLLAR }>;
	/// Handler to notify the runtime when a collator is paid
	type OnCollatorPayout = ();
	type PayoutCollatorReward = ();
	/// Handler to notify the runtime when a new round begins
	type OnNewRound = ();
	/// Any additional issuance that should be used for inflation calcs
	type AdditionalIssuance = ();
	type WeightInfo = pallet_parachain_staking::weights::SubstrateWeight<Test>;
}

pub struct MockDelegatorActions<T, C>(PhantomData<(T, C)>);
impl<
		T: Config + pallet::Config<Currency = C> + pallet_parachain_staking::Config,
		C: frame_support::traits::ReservableCurrency<T::AccountId>,
	> pallet_parachain_staking::DelegatorActions<T::AccountId, BalanceOf<T>>
	for MockDelegatorActions<T, C>
{
	fn delegator_bond_more(
		delegator: &T::AccountId,
		candidate: &T::AccountId,
		amount: BalanceOf<T>,
	) -> Result<bool, DispatchError> {
		if *delegator != T::AccountId::decode(&mut DELEGATOR_ACCOUNT.as_ref()).unwrap() {
			return Err(<pallet_parachain_staking::Error<T>>::DelegatorDNE.into())
		}

		if *candidate != T::AccountId::decode(&mut COLLATOR_ACCOUNT.as_ref()).unwrap() {
			return Err(<pallet_parachain_staking::Error<T>>::DelegationDNE.into())
		}

		let delegation: u128 = amount.saturated_into();
		C::reserve(delegator, delegation.saturated_into())?;
		System::deposit_event(pallet_parachain_staking::Event::DelegationIncreased {
			delegator: AccountId::from(DELEGATOR_ACCOUNT),
			candidate: AccountId::from(COLLATOR_ACCOUNT),
			amount: delegation,
			in_top: true,
		});
		Ok(true)
	}

	fn get_delegator_stakable_free_balance(delegator: &T::AccountId) -> BalanceOf<T> {
		C::free_balance(delegator)
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
	pub const MaxBlockWeight: u64 = 20_000_000;
	pub const MaxWeightPercentage: Perbill = Perbill::from_percent(40);
	pub const UpdateQueueRatio: Perbill = Perbill::from_percent(50);
	pub const ExecutionWeightFee: Balance = NATIVE_EXECUTION_WEIGHT_FEE;

	// When unit testing dynamic dispatch, we use the real weight value of the extrinsics call
	// This is an external lib that we don't own so we try to not mock, follow the rule don't mock
	// what you don't own
	// One of test we do is Balances::transfer call, which has its weight define here:
	// https://github.com/paritytech/substrate/blob/polkadot-v0.9.38/frame/balances/src/weights.rs#L61-L73
	// When logging the final calculated amount, its value is 73_314_000.
	//
	// in our unit test, we test a few transfers with dynamic dispatch. On top
	// of that, there is also weight of our call such as fetching the tasks,
	// move from schedule slot to tasks queue,.. so the weight of a schedule
	// transfer with dynamic dispatch is even higher.
	//
	// and because we test run a few of them so I set it to ~10x value of 73_314_000
	pub const MaxWeightPerSlot: u128 = 700_000_000;
	pub const XmpFee: u128 = 1_000_000;
	pub const GetNativeCurrencyId: CurrencyId = NATIVE;
}

pub struct MockPalletBalanceWeight<T>(PhantomData<T>);
impl<Test: frame_system::Config> pallet_balances::WeightInfo for MockPalletBalanceWeight<Test> {
	fn transfer() -> Weight {
		Weight::from_ref_time(100_000)
	}

	fn transfer_keep_alive() -> Weight {
		Weight::zero()
	}
	fn set_balance_creating() -> Weight {
		Weight::zero()
	}
	fn set_balance_killing() -> Weight {
		Weight::zero()
	}
	fn force_transfer() -> Weight {
		Weight::zero()
	}
	fn transfer_all() -> Weight {
		Weight::zero()
	}
	fn force_unreserve() -> Weight {
		Weight::zero()
	}
}

pub struct MockWeight<T>(PhantomData<T>);
impl<Test: frame_system::Config> pallet_automation_time::WeightInfo for MockWeight<Test> {
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
		Weight::from_ref_time(900_000)
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
		_destination: MultiLocation,
		_location: xcm::latest::MultiLocation,
		_fee: u128,
		_caller: T::AccountId,
		_transact_encoded_call: sp_std::vec::Vec<u8>,
		_transact_encoded_call_weight: Weight,
		_overall_weight: Weight,
		_flow: InstructionSequence,
	) -> Result<(), sp_runtime::DispatchError> {
		Ok(().into())
	}

	fn pay_xcm_fee(_: T::AccountId, _: u128) -> Result<(), sp_runtime::DispatchError> {
		Ok(().into())
	}
}

pub struct ScheduleAllowList;
impl Contains<RuntimeCall> for ScheduleAllowList {
	fn contains(c: &RuntimeCall) -> bool {
		match c {
			RuntimeCall::System(_) => true,
			RuntimeCall::Balances(_) => true,
			_ => false,
		}
	}
}

pub struct MockConversionRateProvider;
impl FixedConversionRateProvider for MockConversionRateProvider {
	fn get_fee_per_second(location: &MultiLocation) -> Option<u128> {
		get_fee_per_second(location)
	}
}

pub struct MockTokenIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for MockTokenIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		if id == NATIVE {
			Some(MultiLocation::new(0, Here))
		} else if id == FOREIGN_CURRENCY_ID {
			Some(MultiLocation::new(1, X1(Parachain(PARA_ID))))
		} else {
			None
		}
	}
}

impl Convert<MultiLocation, Option<CurrencyId>> for MockTokenIdConvert {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		if location == MultiLocation::new(0, Here) {
			Some(NATIVE)
		} else if location == MultiLocation::new(1, X1(Parachain(PARA_ID))) {
			Some(FOREIGN_CURRENCY_ID)
		} else {
			None
		}
	}
}

pub struct MockEnsureProxy;
impl EnsureProxy<AccountId> for MockEnsureProxy {
	fn ensure_ok(_delegator: AccountId, _delegatee: AccountId) -> Result<(), &'static str> {
		if _delegator == DELEGATOR_ACCOUNT.into() && _delegatee == PROXY_ACCOUNT.into() {
			Ok(())
		} else {
			Err("proxy error: expected `ProxyType::Any`")
		}
	}
}

pub struct MockTransferCallCreator;
impl TransferCallCreator<MultiAddress<AccountId, ()>, Balance, RuntimeCall>
	for MockTransferCallCreator
{
	fn create_transfer_call(dest: MultiAddress<AccountId, ()>, value: Balance) -> RuntimeCall {
		let account_id = match dest {
			MultiAddress::Id(i) => Some(i),
			_ => None,
		};

		let call: RuntimeCall =
			pallet_balances::Call::transfer { dest: account_id.unwrap(), value }.into();
		call
	}
}

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	// The universal location within the global consensus system
	pub UniversalLocation: InteriorMultiLocation =
		X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
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
	type Call = RuntimeCall;
	type ScheduleAllowList = ScheduleAllowList;
	type CurrencyIdConvert = MockTokenIdConvert;
	type FeeConversionRateProvider = MockConversionRateProvider;
	type EnsureProxy = MockEnsureProxy;
	type UniversalLocation = UniversalLocation;
	type SelfParaId = parachain_info::Pallet<Test>;
	type TransferCallCreator = MockTransferCallCreator;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(state_block_time: u64) -> sp_io::TestExternalities {
	let genesis_storage = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut ext = sp_io::TestExternalities::new(genesis_storage);
	ext.execute_with(|| System::set_block_number(1));
	ext.execute_with(|| Timestamp::set_timestamp(state_block_time));
	ext
}

// A function to support test scheduleing a Fixed schedule
// We don't focus on making sure the execution run properly. We just focus on
// making sure a task is scheduled into the queue
pub fn schedule_task(owner: [u8; 32], scheduled_times: Vec<u64>, message: Vec<u8>) -> TaskIdV2 {
	let call: RuntimeCall = frame_system::Call::remark_with_event { remark: message }.into();
	let task_id = schedule_dynamic_dispatch_task(owner, scheduled_times, call);
	task_id
}

pub fn schedule_dynamic_dispatch_task(
	owner: [u8; 32],
	scheduled_times: Vec<u64>,
	call: RuntimeCall,
) -> TaskIdV2 {
	let account_id = AccountId32::new(owner);

	assert_ok!(fund_account_dynamic_dispatch(
		&account_id,
		scheduled_times.len(),
		call.clone().encode()
	));

	assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
		RuntimeOrigin::signed(account_id.clone()),
		ScheduleParam::Fixed { execution_times: scheduled_times },
		Box::new(call.clone()),
	));
	last_task_id()
}

// A function to support test scheduling a Recurring schedule
// We don't focus on making sure the execution run properly. We just focus on
// making sure a task is scheduled into the queue
pub fn schedule_recurring_task(
	owner: [u8; 32],
	next_execution_time: UnixTime,
	frequency: Seconds,
	message: Vec<u8>,
) -> TaskIdV2 {
	let account_id = AccountId32::new(owner);
	let call: RuntimeCall = frame_system::Call::remark_with_event { remark: message }.into();

	assert_ok!(fund_account_dynamic_dispatch(&account_id, 1, call.encode()));

	assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
		RuntimeOrigin::signed(account_id.clone()),
		ScheduleParam::Recurring { next_execution_time, frequency },
		Box::new(call),
	));
	last_task_id()
}

pub fn add_task_to_task_queue(
	owner: [u8; 32],
	task_id: TaskIdV2,
	scheduled_times: Vec<u64>,
	action: ActionOf<Test>,
	abort_errors: Vec<Vec<u8>>,
) -> TaskIdV2 {
	let schedule = Schedule::new_fixed_schedule::<Test>(scheduled_times).unwrap();
	add_to_task_queue(owner, task_id, schedule, action, abort_errors)
}

pub fn add_recurring_task_to_task_queue(
	owner: [u8; 32],
	task_id: TaskIdV2,
	scheduled_time: u64,
	frequency: u64,
	action: ActionOf<Test>,
	abort_errors: Vec<Vec<u8>>,
) -> TaskIdV2 {
	let schedule = Schedule::new_recurring_schedule::<Test>(scheduled_time, frequency).unwrap();
	add_to_task_queue(owner, task_id, schedule, action, abort_errors)
}

pub fn add_to_task_queue(
	owner: [u8; 32],
	task_id: TaskIdV2,
	schedule: Schedule,
	action: ActionOf<Test>,
	abort_errors: Vec<Vec<u8>>,
) -> TaskIdV2 {
	let task_id = create_task(owner, task_id, schedule, action, abort_errors);
	let mut task_queue = AutomationTime::get_task_queue();
	task_queue.push((AccountId32::new(owner), task_id.clone()));
	TaskQueueV2::<Test>::put(task_queue);
	task_id
}

pub fn add_task_to_missed_queue(
	owner: [u8; 32],
	task_id: TaskIdV2,
	scheduled_times: Vec<u64>,
	action: ActionOf<Test>,
	abort_errors: Vec<Vec<u8>>,
) -> TaskIdV2 {
	let schedule = Schedule::new_fixed_schedule::<Test>(scheduled_times.clone()).unwrap();
	let task_id = create_task(owner, task_id.clone(), schedule, action, abort_errors);
	let missed_task =
		MissedTaskV2Of::<Test>::new(AccountId32::new(owner), task_id.clone(), scheduled_times[0]);
	let mut missed_queue = AutomationTime::get_missed_queue();
	missed_queue.push(missed_task);
	MissedQueueV2::<Test>::put(missed_queue);
	task_id
}

pub fn create_task(
	owner: [u8; 32],
	task_id: TaskIdV2,
	schedule: Schedule,
	action: ActionOf<Test>,
	abort_errors: Vec<Vec<u8>>,
) -> TaskIdV2 {
	let task = TaskOf::<Test>::new(owner.into(), task_id.clone(), schedule, action, abort_errors);
	AccountTasks::<Test>::insert(AccountId::new(owner), task_id.clone(), task);
	task_id
}

pub fn events() -> Vec<RuntimeEvent> {
	let events = System::events();
	let evt = events.into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}

pub fn last_event() -> RuntimeEvent {
	events().pop().unwrap()
}

// A utility test function to simplify the process of getting a task id that we just scheduled in the
// test by looking at the last id and pluck it
pub fn last_task_id() -> TaskIdV2 {
	get_task_ids_from_events()
		.last()
		.expect("Unable to find a task_id from the existing TaskScheduled events")
		.clone()
}

// A utility test function to pluck out the task id from events, useful when dealing with multiple
// task scheduling
pub fn get_task_ids_from_events() -> Vec<TaskIdV2> {
	System::events()
		.into_iter()
		.filter_map(|e| match e.event {
			RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
				who,
				schedule_as,
				task_id,
			}) => Some(task_id),
			_ => None,
		})
		.collect::<Vec<_>>()
}

pub fn get_funds(account: AccountId) {
	let double_action_weight = Weight::from_ref_time(20_000 as u64) * 2;

	let action_fee = ExecutionWeightFee::get() * u128::from(double_action_weight.ref_time());
	let max_execution_fee = action_fee * u128::from(MaxExecutionTimes::get());
	Balances::set_balance(RawOrigin::Root.into(), account, max_execution_fee, 0).unwrap();
}

pub fn get_minimum_funds(account: AccountId, executions: u32) {
	let double_action_weight = Weight::from_ref_time(20_000 as u64) * 2;
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

pub fn get_fee_per_second(location: &MultiLocation) -> Option<u128> {
	let location = location
		.reanchored(
			&MultiLocation::new(1, X1(Parachain(<Test as Config>::SelfParaId::get().into())))
				.into(),
			<Test as Config>::UniversalLocation::get(),
		)
		.expect("Reanchor location failed");

	let found_asset = ASSET_FEE_PER_SECOND.into_iter().find(|item| match item {
		MockAssetFeePerSecond { asset_location, .. } => *asset_location == location,
	});

	if found_asset.is_some() {
		Some(found_asset.unwrap().fee_per_second)
	} else {
		None
	}
}
