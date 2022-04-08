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
use crate as pallet_valve;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{Contains, GenesisBuild, OnUnbalanced},
	weights::Weight,
};
use pallet_balances::NegativeImbalance;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};
use sp_std::marker::PhantomData;

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		AutomationTime: pallet_automation_time::{Pallet, Call, Storage, Event<T>},
		Valve: pallet_valve::{Pallet, Call, Storage, Event<T>, Config},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const SS58Prefix: u8 = 51;
}
impl frame_system::Config for Test {
	type BaseCallFilter = Valve;
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
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
	type BlockWeights = ();
	type BlockLength = ();
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

parameter_types! {
	pub const MaxTasksPerSlot: u32 = 2;
	pub const MaxScheduleSeconds: u64 = 1 * 24 * 60 * 60;
	pub const MaxBlockWeight: Weight = 1200_000;
	pub const MaxWeightPercentage: Perbill = Perbill::from_percent(10);
	pub const UpdateQueueRatio: Perbill = Perbill::from_percent(50);
	pub const SecondsPerBlock: u64 = 12;
	pub const ExecutionWeightFee: Balance = 12;
}

pub struct MockAutomationTimeWeight<T>(PhantomData<T>);
impl<Test: frame_system::Config> pallet_automation_time::WeightInfo
	for MockAutomationTimeWeight<Test>
{
	fn schedule_notify_task_empty() -> Weight {
		0
	}
	fn schedule_notify_task_full() -> Weight {
		0
	}
	fn schedule_native_transfer_task_empty() -> Weight {
		0
	}
	fn schedule_native_transfer_task_full() -> Weight {
		0
	}
	fn cancel_scheduled_task() -> Weight {
		0
	}
	fn cancel_scheduled_task_full() -> Weight {
		0
	}
	fn cancel_overflow_task() -> Weight {
		0
	}
	fn force_cancel_scheduled_task() -> Weight {
		0
	}
	fn force_cancel_scheduled_task_full() -> Weight {
		0
	}
	fn force_cancel_overflow_task() -> Weight {
		0
	}
	fn run_notify_task() -> Weight {
		0
	}
	fn run_native_transfer_task() -> Weight {
		0
	}
	fn run_missed_tasks_many_found(_v: u32) -> Weight {
		0
	}
	fn run_missed_tasks_many_missing(_v: u32) -> Weight {
		0
	}
	fn run_tasks_many_found(_v: u32) -> Weight {
		0
	}
	fn run_tasks_many_missing(_v: u32) -> Weight {
		0
	}
	fn update_task_queue_overhead() -> Weight {
		0
	}
	fn append_to_missed_tasks(_v: u32) -> Weight {
		0
	}
	fn update_scheduled_task_queue() -> Weight {
		0
	}
	fn shift_missed_tasks() -> Weight {
		0
	}
}

pub struct DealWithExecutionFees<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for DealWithExecutionFees<R>
where
	R: pallet_balances::Config,
{
	fn on_unbalanceds<B>(_fees: impl Iterator<Item = NegativeImbalance<R>>) {}
}

impl pallet_automation_time::Config for Test {
	type Event = Event;
	type MaxTasksPerSlot = MaxTasksPerSlot;
	type MaxScheduleSeconds = MaxScheduleSeconds;
	type MaxBlockWeight = MaxBlockWeight;
	type MaxWeightPercentage = MaxWeightPercentage;
	type UpdateQueueRatio = UpdateQueueRatio;
	type SecondsPerBlock = SecondsPerBlock;
	type WeightInfo = MockAutomationTimeWeight<Test>;
	type ExecutionWeightFee = ExecutionWeightFee;
	type NativeTokenExchange =
		pallet_automation_time::CurrencyAdapter<Balances, DealWithExecutionFees<Test>>;
}

/// During maintenance mode we will not allow any calls.
pub struct ClosedCallFilter;
impl Contains<Call> for ClosedCallFilter {
	fn contains(_: &Call) -> bool {
		false
	}
}

pub struct MockWeight<T>(PhantomData<T>);
impl<Test: frame_system::Config> pallet_valve::WeightInfo for MockWeight<Test> {
	fn close_valve() -> Weight {
		0
	}
	fn open_valve() -> Weight {
		0
	}
	fn close_pallet_gate_new() -> Weight {
		0
	}
	fn close_pallet_gate_existing() -> Weight {
		0
	}
	fn open_pallet_gate() -> Weight {
		0
	}
	fn open_pallet_gates() -> Weight {
		0
	}
	fn stop_scheduled_tasks() -> Weight {
		0
	}
	fn start_scheduled_tasks() -> Weight {
		0
	}
}

impl Config for Test {
	type Event = Event;
	type WeightInfo = MockWeight<Test>;
	type ClosedCallFilter = ClosedCallFilter;
}

/// Externality builder for pallet maintenance mode's mock runtime
pub(crate) struct ExtBuilder {
	valve_closed: bool,
	closed_gates: Vec<Vec<u8>>,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder { valve_closed: false, closed_gates: vec![] }
	}
}

impl ExtBuilder {
	pub(crate) fn with_valve_closed(mut self, c: bool) -> Self {
		self.valve_closed = c;
		self
	}

	pub(crate) fn with_gate_closed(mut self, g: Vec<u8>) -> Self {
		self.closed_gates = vec![g];
		self
	}

	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.expect("Frame system builds valid default genesis config");

		GenesisBuild::<Test>::assimilate_storage(
			&pallet_valve::GenesisConfig {
				start_with_valve_closed: self.valve_closed,
				closed_gates: self.closed_gates,
			},
			&mut t,
		)
		.expect("Pallet valve storage can be assimilated");

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn events() -> Vec<pallet_valve::Event<Test>> {
	let evt = System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| if let Event::Valve(inner) = e { Some(inner) } else { None })
		.collect::<Vec<_>>();

	System::reset_events();
	evt
}
