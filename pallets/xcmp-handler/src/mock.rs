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

use crate::{self as pallet_xcmp_handler, XcmFlow};
use core::cell::RefCell;
use frame_support::{
	parameter_types,
	traits::{ConstU32, Everything, GenesisBuild, Nothing},
};
use frame_system as system;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, Convert, IdentityLookup},
	AccountId32,
};
use xcm::latest::{prelude::*, Weight};
use xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, EnsureXcmOrigin, FixedWeightBounds,
	ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::{
	traits::{TransactAsset, WeightTrader},
	Assets, XcmExecutor,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;
pub type Balance = u128;
pub type CurrencyId = u32;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const LOCAL_PARA_ID: u32 = 2114;
pub const NATIVE: CurrencyId = 0;
// pub const RELAY: CurrencyId = 1;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		ParachainInfo: parachain_info::{Pallet, Storage, Config},
		XcmpHandler: pallet_xcmp_handler::{Pallet, Call, Storage, Event<T>},
		XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 51;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

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

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(Junction::AccountId32 { network: None, id: account.into() }).into()
	}
}

thread_local! {
	pub static SENT_XCM: RefCell<Vec<(MultiLocation,Xcm<()>)>>  = RefCell::new(Vec::new());
	pub static TRANSACT_ASSET: RefCell<Vec<(MultiAsset,MultiLocation)>>  = RefCell::new(Vec::new());
}

pub(crate) fn sent_xcm() -> Vec<(MultiLocation, Xcm<()>)> {
	SENT_XCM.with(|q| (*q.borrow()).clone())
}

pub(crate) fn transact_asset() -> Vec<(MultiAsset, MultiLocation)> {
	TRANSACT_ASSET.with(|q| (*q.borrow()).clone())
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToCallOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	XcmPassthrough<RuntimeOrigin>,
);

/// Sender that returns error if call equals [9,9,9]
pub struct TestSendXcm;
impl SendXcm for TestSendXcm {
	type Ticket = ();

	fn validate(
		destination: &mut Option<MultiLocation>,
		message: &mut Option<opaque::Xcm>,
	) -> SendResult<Self::Ticket> {
		let err_message = Xcm(vec![Transact {
			origin_kind: OriginKind::Native,
			require_weight_at_most: Weight::from_ref_time(100_000),
			call: vec![9, 1, 1].into(),
		}]);
		if message.clone().unwrap() == err_message {
			Err(SendError::Transport("Destination location full"))
		} else {
			SENT_XCM.with(|q| {
				q.borrow_mut().push((destination.clone().unwrap(), message.clone().unwrap()))
			});
			Ok(((), MultiAssets::new()))
		}
	}

	fn deliver(_: Self::Ticket) -> Result<XcmHash, SendError> {
		Ok(XcmHash::default())
	}
}

// XCMP Mocks
parameter_types! {
	pub const UnitWeightCost: u64 = 10;
	pub const MaxInstructions: u32 = 100;
}
pub struct DummyWeightTrader;
impl WeightTrader for DummyWeightTrader {
	fn new() -> Self {
		DummyWeightTrader
	}

	fn buy_weight(&mut self, _weight: Weight, _payment: Assets) -> Result<Assets, XcmError> {
		Ok(Assets::default())
	}
}
pub struct DummyAssetTransactor;
impl TransactAsset for DummyAssetTransactor {
	fn deposit_asset(what: &MultiAsset, who: &MultiLocation, _context: &XcmContext) -> XcmResult {
		let asset = what.clone();
		let location = who.clone();
		TRANSACT_ASSET.with(|q| q.borrow_mut().push((asset, location)));
		Ok(())
	}

	fn withdraw_asset(
		what: &MultiAsset,
		who: &MultiLocation,
		_maybe_context: Option<&XcmContext>,
	) -> Result<Assets, XcmError> {
		let asset = what.clone();
		let location = who.clone();
		TRANSACT_ASSET.with(|q| q.borrow_mut().push((asset.clone(), location)));
		Ok(asset.into())
	}
}

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub UniversalLocation: InteriorMultiLocation =
		X1(Parachain(2114).into());
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
}
pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = TestSendXcm;
	type AssetTransactor = DummyAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = ();
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = DummyWeightTrader;
	type ResponseHandler = ();
	type AssetTrap = XcmPallet;
	type AssetClaims = XcmPallet;
	type SubscriptionService = XcmPallet;

	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<64>;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = ();
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
}

impl cumulus_pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub struct TokenIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for TokenIdConvert {
	fn convert(_id: CurrencyId) -> Option<MultiLocation> {
		// Mock implementation with default value
		Some(MultiLocation { parents: 1, interior: Here })
	}
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = NATIVE;
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

impl pallet_xcmp_handler::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type CurrencyId = CurrencyId;
	type Currency = Balances;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type SelfParaId = parachain_info::Pallet<Test>;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type CurrencyIdToMultiLocation = TokenIdConvert;
	type UniversalLocation = UniversalLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmSender = TestSendXcm;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(
	genesis_config: Option<Vec<(Vec<u8>, XcmFlow)>>,
) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.expect("Frame system builds valid default genesis config");

	GenesisBuild::<Test>::assimilate_storage(
		&parachain_info::GenesisConfig { parachain_id: LOCAL_PARA_ID.into() },
		&mut t,
	)
	.expect("Pallet Parachain info can be assimilated");

	if let Some(asset_data) = genesis_config {
		GenesisBuild::<Test>::assimilate_storage(
			&pallet_xcmp_handler::GenesisConfig { asset_data },
			&mut t,
		)
		.expect("Pallet Parachain info can be assimilated");
	}

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
