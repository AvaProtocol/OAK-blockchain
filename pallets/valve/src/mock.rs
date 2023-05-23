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
	traits::{Contains, GenesisBuild, SortedMembers},
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32,
};
use sp_std::cell::RefCell;

pub type AccountId = AccountId32;
pub type BlockNumber = u64;
pub type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const COLLECTIVE_MEMBER: [u8; 32] = [1u8; 32];
pub const NON_COLLECTIVE_MEMBER: [u8; 32] = [2u8; 32];

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Valve: pallet_valve::{Pallet, Call, Storage, Event<T>, Config},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 51;
}
impl frame_system::Config for Test {
	type BaseCallFilter = Valve;
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
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
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

/// During maintenance mode we will not allow any calls.
pub struct ClosedCallFilter;
impl Contains<RuntimeCall> for ClosedCallFilter {
	fn contains(_: &RuntimeCall) -> bool {
		false
	}
}

pub struct TechCollective;
impl SortedMembers<AccountId> for TechCollective {
	fn contains(account_id: &AccountId) -> bool {
		account_id == &AccountId32::new(COLLECTIVE_MEMBER)
	}

	fn sorted_members() -> Vec<AccountId> {
		vec![]
	}
}

thread_local! {
	pub static MOCK_AUTOMATION_TIME_STORE: RefCell<bool> = RefCell::new(false);
}
pub struct MockAutomationTime;
impl Shutdown for MockAutomationTime {
	fn is_shutdown() -> bool {
		MOCK_AUTOMATION_TIME_STORE.with(|s| *s.borrow())
	}
	fn shutdown() {
		MOCK_AUTOMATION_TIME_STORE.with(|l| {
			let r = *l.borrow();
			*l.borrow_mut() = true;
			r
		});
	}
	fn restart() {
		MOCK_AUTOMATION_TIME_STORE.with(|l| {
			let r = *l.borrow();
			*l.borrow_mut() = false;
			r
		});
	}
}

pub struct MockAutomationPrice;
impl Shutdown for MockAutomationPrice {
	fn is_shutdown() -> bool {
		MOCK_AUTOMATION_TIME_STORE.with(|s| *s.borrow())
	}
	fn shutdown() {
		MOCK_AUTOMATION_TIME_STORE.with(|l| {
			let r = *l.borrow();
			*l.borrow_mut() = true;
			r
		});
	}
	fn restart() {
		MOCK_AUTOMATION_TIME_STORE.with(|l| {
			let r = *l.borrow();
			*l.borrow_mut() = false;
			r
		});
	}
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ClosedCallFilter = ClosedCallFilter;
	type AutomationTime = MockAutomationTime;
	type AutomationPrice = MockAutomationPrice;
	type CallAccessFilter = TechCollective;
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
		.filter_map(|e| if let RuntimeEvent::Valve(inner) = e { Some(inner) } else { None })
		.collect::<Vec<_>>();

	System::reset_events();
	evt
}
