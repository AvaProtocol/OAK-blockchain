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
use crate as pallet_automation_price;
use crate::TaskId;

use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, Contains, Everything},
	weights::Weight,
	PalletId,
};
use frame_system::{self as system, RawOrigin};
use orml_traits::parameter_type_with_key;
use primitives::{EnsureProxy, TransferCallCreator};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, Convert, IdentityLookup},
	AccountId32, MultiAddress, Perbill,
};
use sp_std::{marker::PhantomData, vec::Vec};
use xcm::latest::prelude::*;

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

use crate::weights::WeightInfo;

pub type Balance = u128;
pub type AccountId = AccountId32;
pub type CurrencyId = u32;

pub const DEFAULT_SCHEDULE_FEE_LOCATION: MultiLocation = MOONBASE_ASSET_LOCATION;

pub const ALICE: [u8; 32] = [1u8; 32];
pub const DELEGATOR_ACCOUNT: [u8; 32] = [3u8; 32];
pub const PROXY_ACCOUNT: [u8; 32] = [4u8; 32];

pub const PARA_ID: u32 = 2000;
pub const NATIVE: CurrencyId = 0;
pub const NATIVE_LOCATION: MultiLocation = MultiLocation { parents: 0, interior: Here };
pub const NATIVE_EXECUTION_WEIGHT_FEE: u128 = 12;
pub const FOREIGN_CURRENCY_ID: CurrencyId = 1;

const DOLLAR: u128 = 10_000_000_000;
const DECIMAL: u8 = 10;

pub const MOONBASE_ASSET_LOCATION: MultiLocation =
	MultiLocation { parents: 1, interior: X2(Parachain(1000), PalletInstance(3)) };

pub const exchange1: &[u8] = "exchange1".as_bytes();
pub const exchange2: &[u8] = "exchange2".as_bytes();

pub const chain1: &[u8] = "KUSAMA".as_bytes();
pub const chain2: &[u8] = "DOT".as_bytes();

pub const asset1: &[u8] = "TUR".as_bytes();
pub const asset2: &[u8] = "USDC".as_bytes();
pub const asset3: &[u8] = "KSM".as_bytes();

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
		AutomationPrice: pallet_automation_price::{Pallet, Call, Storage, Event<T>},
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

impl pallet_automation_price::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxTasksPerSlot = MaxTasksPerSlot;
	type MaxBlockWeight = MaxBlockWeight;
	type MaxWeightPercentage = MaxWeightPercentage;
	type WeightInfo = MockWeight<Test>;
	type ExecutionWeightFee = ExecutionWeightFee;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Currencies;
	type Currency = Balances;
	type CurrencyIdConvert = MockTokenIdConvert;
	type FeeHandler = FeeHandler<Test, ()>;
	type FeeConversionRateProvider = MockConversionRateProvider;
	type UniversalLocation = UniversalLocation;
	type SelfParaId = parachain_info::Pallet<Test>;
	type XcmpTransactor = MockXcmpTransactor<Test, Balances>;

	type EnsureProxy = MockEnsureProxy;
}

parameter_types! {
	pub const MaxTasksPerSlot: u32 = 2;
	#[derive(Debug)]
	pub const MaxScheduleSeconds: u64 = 24 * 60 * 60;
	pub const MaxBlockWeight: u64 = 20_000_000;
	pub const MaxWeightPercentage: Perbill = Perbill::from_percent(40);
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
impl<Test: frame_system::Config> pallet_automation_price::WeightInfo for MockWeight<Test> {
	fn emit_event() -> Weight {
		Weight::from_ref_time(20_000_000_u64)
	}

	fn asset_price_update_extrinsic(v: u32) -> Weight {
		Weight::from_ref_time(220_000_000_u64 * v as u64)
	}

	fn initialize_asset_extrinsic(v: u32) -> Weight {
		Weight::from_ref_time(220_000_000_u64)
	}

	fn schedule_xcmp_task_extrinsic() -> Weight {
		Weight::from_ref_time(200_000_000_u64)
	}

	fn cancel_task_extrinsic() -> Weight {
		Weight::from_ref_time(20_000_000_u64)
	}

	fn run_xcmp_task() -> Weight {
		Weight::from_ref_time(200_000_000_u64)
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
		Ok(())
	}

	fn pay_xcm_fee(_: T::AccountId, _: u128) -> Result<(), sp_runtime::DispatchError> {
		Ok(())
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

// TODO: We should extract this and share code with automation-time
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

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(state_block_time: u64) -> sp_io::TestExternalities {
	let genesis_storage = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut ext = sp_io::TestExternalities::new(genesis_storage);
	ext.execute_with(|| System::set_block_number(1));
	ext.execute_with(|| Timestamp::set_timestamp(state_block_time));
	ext
}

pub fn events() -> Vec<RuntimeEvent> {
	let events = System::events();
	let evt = events.into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}

// A utility test function to pluck out the task id from events, useful when dealing with multiple
// task scheduling
pub fn get_task_ids_from_events() -> Vec<TaskId> {
	System::events()
		.into_iter()
		.filter_map(|e| match e.event {
			RuntimeEvent::AutomationPrice(crate::Event::TaskScheduled { task_id, .. }) =>
				Some(task_id),
			_ => None,
		})
		.collect::<Vec<_>>()
}

pub fn get_funds(account: AccountId) {
	let double_action_weight = Weight::from_ref_time(20_000_u64) * 2;

	let action_fee = ExecutionWeightFee::get() * u128::from(double_action_weight.ref_time());
	let max_execution_fee = action_fee;
	Balances::set_balance(RawOrigin::Root.into(), account, max_execution_fee, 0).unwrap();
}

pub fn get_minimum_funds(account: AccountId, executions: u32) {
	let double_action_weight = Weight::from_ref_time(20_000_u64) * 2;
	let action_fee = ExecutionWeightFee::get() * u128::from(double_action_weight.ref_time());
	let max_execution_fee = action_fee * u128::from(executions);
	Balances::set_balance(RawOrigin::Root.into(), account, max_execution_fee, 0).unwrap();
}

pub fn get_xcmp_funds(account: AccountId) {
	let double_action_weight = MockWeight::<Test>::run_xcmp_task() * 2;
	let action_fee = ExecutionWeightFee::get() * u128::from(double_action_weight.ref_time());
	let max_execution_fee = action_fee * u128::from(1u32);
	let with_xcm_fees = max_execution_fee + XmpFee::get();
	Balances::set_balance(RawOrigin::Root.into(), account, with_xcm_fees, 0).unwrap();
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

pub fn get_fee_per_second(location: &MultiLocation) -> Option<u128> {
	let location = location
		.reanchored(
			&MultiLocation::new(1, X1(Parachain(<Test as Config>::SelfParaId::get().into()))),
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

// setup a sample default asset to support test
pub fn setup_asset(sender: &AccountId32, chain: Vec<u8>) {
	AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		chain,
		exchange1.to_vec(),
		asset1.to_vec(),
		asset2.to_vec(),
		10,
		vec![sender.clone()],
	);
}

// setup a few sample assets, initialize it with sane default vale and set a price to support test cases
pub fn setup_prices(sender: &AccountId32) {
	AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		chain1.to_vec(),
		exchange1.to_vec(),
		asset1.to_vec(),
		asset2.to_vec(),
		10,
		vec![sender.clone()],
	);

	AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		chain2.to_vec(),
		exchange1.to_vec(),
		asset2.to_vec(),
		asset3.to_vec(),
		10,
		vec![sender.clone()],
	);

	AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		chain2.to_vec(),
		exchange1.to_vec(),
		asset1.to_vec(),
		asset3.to_vec(),
		10,
		vec![sender.clone()],
	);
}
