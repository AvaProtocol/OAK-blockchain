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
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Decode, Encode, MaxEncodedLen};
use hex_literal::hex;
use pallet_automation_time_rpc_runtime_api::{
	AutomationAction, AutostakingResult, FeeDetails as AutomationFeeDetails,
};
use primitives::{assets::CustomMetadata, TokenId};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto, Zero,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	AccountId32, ApplyExtrinsicResult, Percent, RuntimeDebug,
};
use xcm::latest::{prelude::*, MultiLocation};
use xcm_builder::Account32Hash;
use xcm_executor::traits::Convert;

use sp_std::{cmp::Ordering, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	ensure, parameter_types,
	traits::{
		ConstU128, ConstU16, ConstU32, ConstU8, Contains, EitherOfDiverse, EnsureOrigin,
		EnsureOriginWithArg, InstanceFilter, PrivilegeCmp,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
		ConstantMultiplier, Weight,
	},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
pub use sp_runtime::{Perbill, Permill, Perquintill};

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Cumulus Imports
use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;

// Polkadot Imports
use polkadot_runtime_common::BlockHashCount;

// XCM configurations.
pub mod xcm_config;
use xcm_config::{FeePerSecondProvider, ToTreasury, TokenIdConvert};

pub mod weights;

// Common imports
use common_runtime::{
	constants::{
		currency::{deposit, CENT, DOLLAR, EXISTENTIAL_DEPOSIT, UNIT},
		fees::SlowAdjustingFeeUpdate,
		time::{DAYS, HOURS, SLOT_DURATION},
		weight_ratios::{
			AVERAGE_ON_INITIALIZE_RATIO, MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO,
			SCHEDULED_TASKS_INITIALIZE_RATIO,
		},
	},
	fees::{DealWithExecutionFees, DealWithInclusionFees},
	CurrencyHooks,
};
use primitives::{
	AccountId, Address, Amount, AuraId, Balance, BlockNumber, Hash, Header, Index, Signature,
};

// Custom pallet imports
pub use pallet_automation_price;
pub use pallet_automation_time;

use primitives::EnsureProxy;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

// All migrations executed on runtime upgrade as a nested tuple of types implementing
// `OnRuntimeUpgrade`.
type Migrations = ();

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use sp_runtime::{generic, traits::BlakeTwo256};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("turing"),
	impl_name: create_runtime_str!("turing"),
	authoring_version: 1,
	spec_version: 294,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 17,
	state_version: 0,
};

pub const NATIVE_TOKEN_ID: TokenId = 0;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO + SCHEDULED_TASKS_INITIALIZE_RATIO)
		.build_or_panic();
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Valve;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix. OAK is 51.
	type SS58Prefix = ConstU16<51>;
	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = ConstU32<100>;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = ParachainStaking;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const BasicDeposit:  Balance = 10 * DOLLAR; // 258 bytes on-chain
	pub const FieldDeposit:  Balance = 1 * DOLLAR; // 66 bytes on-chain
	pub const SubAccountDeposit:  Balance = 2 * DOLLAR; // 53 bytes on-chain
}

type ForceOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
>;
type RegistrarOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
>;

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;

	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;

	type MaxSubAccounts = ConstU32<50>;
	type MaxAdditionalFields = ConstU32<0>;
	type MaxRegistrars = ConstU32<10>;

	type Slashed = Treasury;
	type ForceOrigin = ForceOrigin;
	type RegistrarOrigin = RegistrarOrigin;

	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	Any = 0,
	Session = 1,
	Staking = 2,
	Automation = 3,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::Session => {
				matches!(c, RuntimeCall::Session(..))
			},
			ProxyType::Staking => {
				matches!(c, RuntimeCall::ParachainStaking(..) | RuntimeCall::Session(..))
			},
			ProxyType::Automation => {
				matches!(c, RuntimeCall::AutomationTime(..))
			},
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = ConstU32<32>;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
	pub TreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
	// Until we can codify how to handle forgien tokens that we collect in XCMP fees
	// we will send the tokens to a special account to be dealt with.
	pub TemporaryForeignTreasuryAccount: AccountId = hex!["8acc2955e592588af0eeec40384bf3b498335ecc90df5e6980f0141e1314eb37"].into();
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		*a == TreasuryAccount::get() || *a == TemporaryForeignTreasuryAccount::get()
	}
}

parameter_types! {
	pub TuringTreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
}

impl orml_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = TokenId;
	type WeightInfo = ();
	type ExistentialDeposits = orml_asset_registry::ExistentialDeposits<Runtime>;
	type CurrencyHooks = CurrencyHooks<Runtime, TuringTreasuryAccount>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type DustRemovalWhitelist = DustRemovalWhitelist;
}

parameter_types! {
	pub const GetNativeCurrencyId: TokenId = NATIVE_TOKEN_ID;
}

impl orml_currencies::Config for Runtime {
	type MultiCurrency = Tokens;
	type NativeCurrency =
		orml_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

pub struct AssetAuthority;
impl EnsureOriginWithArg<RuntimeOrigin, Option<u32>> for AssetAuthority {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		_asset_id: &Option<u32>,
	) -> Result<Self::Success, RuntimeOrigin> {
		EnsureRoot::try_origin(origin)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_asset_id: &Option<u32>) -> Result<RuntimeOrigin, ()> {
		EnsureRoot::try_successful_origin()
	}
}

impl orml_asset_registry::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CustomMetadata = CustomMetadata;
	type AssetId = TokenId;
	type AuthorityOrigin = AssetAuthority;
	type AssetProcessor = orml_asset_registry::SequentialId<Runtime>;
	type Balance = Balance;
	type WeightInfo = weights::asset_registry_weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 0;
	pub const WeightToFeeScalar: Balance = 6;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction =
		pallet_transaction_payment::CurrencyAdapter<Balances, DealWithInclusionFees<Runtime>>;
	type WeightToFee = ConstantMultiplier<Balance, WeightToFeeScalar>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = ParachainStaking;
	type NextSessionRotation = ParachainStaking;
	type SessionManager = ParachainStaking;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
}

parameter_types! {
	/// Minimum stake required to become a collator
	pub const MinCollatorStk: u128 = 400_000 * DOLLAR;
	/// Minimum stake required to be reserved to be a candidate
	pub const MinCandidateStk: u128 = 1_000_000 * DOLLAR;
}
impl pallet_parachain_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MonetaryGovernanceOrigin = EnsureRoot<AccountId>;
	/// Minimum round length is 2 minutes (10 * 12 second block times)
	type MinBlocksPerRound = ConstU32<10>;
	/// Rounds before the collator leaving the candidates request can be executed
	type LeaveCandidatesDelay = ConstU32<24>;
	/// Rounds before the candidate bond increase/decrease can be executed
	type CandidateBondLessDelay = ConstU32<24>;
	/// Rounds before the delegator exit can be executed
	type LeaveDelegatorsDelay = ConstU32<24>;
	/// Rounds before the delegator revocation can be executed
	type RevokeDelegationDelay = ConstU32<24>;
	/// Rounds before the delegator bond increase/decrease can be executed
	type DelegationBondLessDelay = ConstU32<24>;
	/// Rounds before the reward is paid
	type RewardPaymentDelay = ConstU32<2>;
	/// Minimum collators selected per round, default at genesis and minimum forever after
	type MinSelectedCandidates = ConstU32<5>;
	/// Maximum top delegations per candidate
	type MaxTopDelegationsPerCandidate = ConstU32<300>;
	/// Maximum bottom delegations per candidate
	type MaxBottomDelegationsPerCandidate = ConstU32<50>;
	/// Maximum delegations per delegator
	type MaxDelegationsPerDelegator = ConstU32<100>;
	type MinCollatorStk = MinCollatorStk;
	type MinCandidateStk = MinCandidateStk;
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
	type AdditionalIssuance = Vesting;
	type WeightInfo = pallet_parachain_staking::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
}

impl pallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type BountyValueMinimum = BountyValueMinimum;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = ConstU32<16384>;
	type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
	type ChildBountyManager = ();
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = ConstU32<100>;
	type MaxMembers = ConstU32<100>;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub TechnicalMotionDuration: BlockNumber = 3 * DAYS;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = ConstU32<100>;
	type MaxMembers = ConstU32<100>;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

type MoreThanHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = MoreThanHalfCouncil;
	type RemoveOrigin = MoreThanHalfCouncil;
	type SwapOrigin = MoreThanHalfCouncil;
	type ResetOrigin = MoreThanHalfCouncil;
	type PrimeOrigin = MoreThanHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = ConstU32<100>;
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 1 * DOLLAR;
	pub const ProposalBondMaximum: Balance = 5 * DOLLAR;
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * UNIT;
	pub const DataDepositPerByte: Balance = 1 * CENT;
	pub const BountyDepositBase: Balance = 1 * UNIT;
	pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub CuratorDepositMin: Balance = DOLLAR;
	pub CuratorDepositMax: Balance = 100 * DOLLAR;
	pub const BountyValueMinimum: Balance = 5 * UNIT;
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
	>;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
	>;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ProposalBondMaximum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = Bounties;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>; // Same as Polkadot
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(10) * RuntimeBlockWeights::get().max_block;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

type ScheduleOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
>;

/// Used the compare the privilege of an origin inside the scheduler.
pub struct OriginPrivilegeCmp;

impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
	fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
		if left == right {
			return Some(Ordering::Equal)
		}

		match (left, right) {
			// Root is greater than anything.
			(OriginCaller::system(frame_system::RawOrigin::Root), _) => Some(Ordering::Greater),
			// Check which one has more yes votes.
			(
				OriginCaller::Council(pallet_collective::RawOrigin::Members(l_yes_votes, l_count)),
				OriginCaller::Council(pallet_collective::RawOrigin::Members(r_yes_votes, r_count)),
			) => Some((l_yes_votes * r_count).cmp(&(r_yes_votes * l_count))),
			// For every other origin we don't care, as they are not used for `ScheduleOrigin`.
			_ => None,
		}
	}
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = ScheduleOrigin;
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = OriginPrivilegeCmp;
	type Preimages = Preimage;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 5 * DAYS;
	pub const VotingPeriod: BlockNumber = 5 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 1 * HOURS;
	pub const MinimumDeposit: Balance = 40 * DOLLAR;
	pub const EnactmentPeriod: BlockNumber = 2 * DAYS;
	pub const VoteLockingPeriod: BlockNumber = 5 * DAYS;
	pub const CooloffPeriod: BlockNumber = 5 * DAYS;
	pub const InstantAllowed: bool = true;
}

impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type VoteLockingPeriod = VoteLockingPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	/// Two thirds of the technical committee can have an `ExternalMajority/ExternalDefault` vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type InstantOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cooloff period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = ConstU32<100>;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type MaxProposals = ConstU32<100>;
	type Preimages = Preimage;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
}

parameter_types! {
	pub const MaxScheduleSeconds: u64 = 6 * 30 * 24 * 60 * 60;
	pub const MaxBlockWeight: u64 = MAXIMUM_BLOCK_WEIGHT.ref_time();
	pub const MaxWeightPercentage: Perbill = SCHEDULED_TASKS_INITIALIZE_RATIO;
	pub const UpdateQueueRatio: Perbill = Perbill::from_percent(50);
	pub const ExecutionWeightFee: Balance = 12;
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

pub struct AutomationEnsureProxy;
impl EnsureProxy<AccountId> for AutomationEnsureProxy {
	fn ensure_ok(delegator: AccountId, delegatee: AccountId) -> Result<(), &'static str> {
		// We only allow for "Any" proxies
		let def: pallet_proxy::ProxyDefinition<AccountId, ProxyType, BlockNumber> =
			pallet_proxy::Pallet::<Runtime>::find_proxy(
				&delegator,
				&delegatee,
				Some(ProxyType::Any),
			)
			.map_err(|_| "proxy error: expected `ProxyType::Any`")?;
		// We only allow to use it for delay zero proxies, as the call will immediatly be executed
		ensure!(def.delay.is_zero(), "proxy delay is Non-zero`");
		Ok(())
	}
}

impl pallet_automation_time::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxTasksPerSlot = ConstU32<256>;
	type MaxExecutionTimes = ConstU32<36>;
	type MaxScheduleSeconds = MaxScheduleSeconds;
	type MaxBlockWeight = MaxBlockWeight;
	type MaxWeightPercentage = MaxWeightPercentage;
	// Roughly .125% of parachain block weight per hour
	// ≈ 500_000_000_000 (MaxBlockWeight) * 300 (Blocks/Hour) * .00125
	type MaxWeightPerSlot = ConstU128<150_000_000_000>;
	type UpdateQueueRatio = UpdateQueueRatio;
	type WeightInfo = pallet_automation_time::weights::SubstrateWeight<Runtime>;
	type ExecutionWeightFee = ExecutionWeightFee;
	type Currency = Balances;
	type MultiCurrency = Currencies;
	type CurrencyId = TokenId;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type XcmpTransactor = XcmpHandler;
	type FeeHandler = pallet_automation_time::FeeHandler<Runtime, ToTreasury>;
	type DelegatorActions = ParachainStaking;
	type CurrencyIdConvert = TokenIdConvert;
	type FeeConversionRateProvider = FeePerSecondProvider;
	type Call = RuntimeCall;
	type ScheduleAllowList = ScheduleAllowList;
	type EnsureProxy = AutomationEnsureProxy;
}

impl pallet_automation_price::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxTasksPerSlot = ConstU32<1>;
	type MaxBlockWeight = MaxBlockWeight;
	type MaxWeightPercentage = MaxWeightPercentage;
	type WeightInfo = ();
	type ExecutionWeightFee = ExecutionWeightFee;
	type Currency = Balances;
	type FeeHandler = pallet_automation_price::FeeHandler<DealWithExecutionFees<Runtime>>;
	type Origin = RuntimeOrigin;
}

pub struct ClosedCallFilter;
impl Contains<RuntimeCall> for ClosedCallFilter {
	fn contains(c: &RuntimeCall) -> bool {
		match c {
			RuntimeCall::AssetRegistry(_) => false,
			RuntimeCall::AutomationTime(_) => false,
			RuntimeCall::Balances(_) => false,
			RuntimeCall::Bounties(_) => false,
			RuntimeCall::Currencies(_) => false,
			RuntimeCall::ParachainStaking(_) => false,
			RuntimeCall::PolkadotXcm(_) => false,
			RuntimeCall::Treasury(_) => false,
			RuntimeCall::XTokens(_) => false,
			RuntimeCall::AutomationPrice(_) => false,
			_ => true,
		}
	}
}

impl pallet_valve::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_valve::weights::SubstrateWeight<Runtime>;
	type ClosedCallFilter = ClosedCallFilter;
	type AutomationTime = AutomationTime;
	type AutomationPrice = AutomationPrice;
	type CallAccessFilter = TechnicalMembership;
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		// System support stuff.
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		ParachainSystem: cumulus_pallet_parachain_system::{
			Pallet, Call, Config, Storage, Inherent, Event<T>, ValidateUnsigned,
		} = 1,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 2,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 3,

		// Monetary stuff.
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>} = 11,
		Currencies: orml_currencies::{Pallet, Call} = 12,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 13,
		AssetRegistry: orml_asset_registry::{Pallet, Call, Storage, Event<T>, Config<T>} = 14,

		// Collator support. The order of these 4 are important and shall not change.
		Authorship: pallet_authorship::{Pallet, Storage} = 20,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 21,
		ParachainStaking: pallet_parachain_staking::{Pallet, Call, Storage, Event<T>, Config<T>} = 22,
		Aura: pallet_aura::{Pallet, Storage, Config<T>} = 23,
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 24,

		// Utilities
		Valve: pallet_valve::{Pallet, Call, Config, Storage, Event<T>} = 30,
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 31,
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 32,
		Utility: pallet_utility::{Pallet, Call, Event} = 33,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 40,
		PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config} = 41,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 42,
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 43,
		XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>} = 44,
		UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event} = 45,

		// Support pallets.
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 51,
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Event<T>, Origin<T>, Config<T>} = 52,
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Event<T>, Origin<T>, Config<T>} = 53,
		TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 54,
		Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 55,
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 56,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 57,
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 58,
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 59,

		//custom pallets
		AutomationTime: pallet_automation_time::{Pallet, Call, Storage, Event<T>} = 60,
		Vesting: pallet_vesting::{Pallet, Storage, Config<T>, Event<T>} = 61,
		XcmpHandler: pallet_xcmp_handler::{Pallet, Call, Storage, Config, Event<T>} = 62,
		AutomationPrice: pallet_automation_price::{Pallet, Call, Storage, Event<T>} = 200,
	}
);

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}


	impl pallet_xcmp_handler_rpc_runtime_api::XcmpHandlerApi<Block, Balance> for Runtime {
		fn cross_chain_account(account_id: AccountId32) -> Result<AccountId32, Vec<u8>> {
			let parachain_id: u32 = ParachainInfo::parachain_id().into();

			let multiloc = MultiLocation::new(
				1,
				X2(
					Parachain(parachain_id),
					Junction::AccountId32 { network: None, id: account_id.into() },
				),
			);

			Account32Hash::<RelayNetwork, sp_runtime::AccountId32>::convert_ref(multiloc)
				.map_err(|_| "unable to convert account".into())
		}
	}

	impl pallet_automation_time_rpc_runtime_api::AutomationTimeApi<Block, AccountId, Hash, Balance> for Runtime {
		fn generate_task_id(account_id: AccountId, provided_id: Vec<u8>) -> Hash {
			AutomationTime::generate_task_id(account_id, provided_id)
		}

		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
		) -> Result<AutomationFeeDetails<Balance>, Vec<u8>> {
			use pallet_automation_time::Action;

			let (action, executions) = match uxt.function {
				RuntimeCall::AutomationTime(pallet_automation_time::Call::schedule_xcmp_task{
					destination, currency_id, fee, encoded_call, encoded_call_weight, overall_weight, schedule, ..
				}) => {
					let destination = MultiLocation::try_from(*destination).map_err(|()| "Unable to convert VersionedMultiLocation".as_bytes())?;
					let action = Action::XCMP { destination, currency_id, fee: *fee, encoded_call, encoded_call_weight, overall_weight, schedule_as: None };
					Ok((action, schedule.number_of_executions()))
				},
				RuntimeCall::AutomationTime(pallet_automation_time::Call::schedule_xcmp_task_through_proxy{
					destination, currency_id, fee, encoded_call, encoded_call_weight, overall_weight, schedule, schedule_as, ..
				}) => {
					let destination = MultiLocation::try_from(*destination).map_err(|()| "Unable to convert VersionedMultiLocation".as_bytes())?;
					let action = Action::XCMP { destination, currency_id, fee: *fee, encoded_call, encoded_call_weight, overall_weight, schedule_as: Some(schedule_as) };
					Ok((action, schedule.number_of_executions()))
				},
				RuntimeCall::AutomationTime(pallet_automation_time::Call::schedule_dynamic_dispatch_task{
					call, schedule, ..
				}) => {
					let action = Action::DynamicDispatch { encoded_call: call.encode() };
					Ok((action, schedule.number_of_executions()))
				},
				_ => Err("Unsupported Extrinsic".as_bytes())
			}?;

			let nobody = AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).expect("always works");
			let fee_handler = <Self as pallet_automation_time::Config>::FeeHandler::new(&nobody, &action, executions)
				.map_err(|_| "Unable to parse fee".as_bytes())?;

			Ok(AutomationFeeDetails {
				execution_fee: fee_handler.execution_fee.into(),
				xcmp_fee: fee_handler.xcmp_fee.into()
			})
		}

		/**
		 * The get_time_automation_fees RPC function is used to get the execution fee of scheduling a time-automation task.
		 * This function requires the action type and the number of executions in order to generate an estimate.
		 * However, the AutomationTime::calculate_execution_fee requires an Action enum from the automation time pallet,
		 * which requires more information than is necessary for this calculation.
		 * Therefore, for ease of use, this function will just require an integer representing the action type and an integer
		 * representing the number of executions. For all of the extraneous information, the function will provide faux inputs for it.
		 *
		 */
		fn get_time_automation_fees(
			action: AutomationAction,
			executions: u32,
		) -> Balance {
			AutomationTime::calculate_execution_fee(&(action.into()), executions).expect("Can only fail for DynamicDispatch which is not an option here")
		}

		fn calculate_optimal_autostaking(
			principal: i128,
			collator: AccountId
		) -> Result<AutostakingResult, Vec<u8>> {
			let candidate_info = ParachainStaking::candidate_info(collator);
			let money_supply = Balances::total_issuance() + Vesting::total_unvested_allocation();

			let collator_stake =
				candidate_info.ok_or("collator does not exist")?.total_counted as i128;
			let fee = AutomationTime::calculate_execution_fee(&(AutomationAction::AutoCompoundDelegatedStake.into()), 1).expect("Can only fail for DynamicDispatch and this is always AutoCompoundDelegatedStake") as i128;

			let duration = 90;
			let total_collators = ParachainStaking::total_selected();
			let daily_collator_rewards =
				(money_supply as f64 * 0.025) as i128 / total_collators as i128 / 365;

			let res = pallet_automation_time::do_calculate_optimal_autostaking(
				principal,
				collator_stake,
				fee,
				duration,
				daily_collator_rewards,
			);

			Ok(AutostakingResult{period: res.0, apy: res.1})
		}

		fn get_auto_compound_delegated_stake_task_ids(account_id: AccountId) -> Vec<Hash> {
			ParachainStaking::delegator_state(account_id.clone())
				.map_or(vec![], |s| s.delegations.0).into_iter()
				.map(|d| d.owner)
				.map(|collator_id| {
					AutomationTime::generate_auto_compound_delegated_stake_provided_id(&account_id, &collator_id)
				})
				.map(|provided_id| {
					AutomationTime::generate_task_id(account_id.clone(), provided_id)
				})
				.filter(|task_id| {
					AutomationTime::get_account_task(account_id.clone(), task_id).is_some()
				}).collect()
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade");
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
				log::info!(
					"try-runtime: executing block #{} ({:?}) / root checks: {:?} / sanity-checks: {:?}",
					block.header.number,
					block.header.hash(),
					state_root_check,
					select,
				);
				Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use pallet_automation_time::Pallet as AutomationTime;
			use pallet_valve::Pallet as Valve;
			use pallet_vesting::Pallet as Vesting;
			use pallet_parachain_staking::Pallet as ParachainStaking;
			use pallet_xcmp_handler::Pallet as XcmpHandler;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmark!(list, extra, pallet_automation_time, AutomationTime::<Runtime>);
			list_benchmark!(list, extra, pallet_valve, Valve::<Runtime>);
			list_benchmark!(list, extra, pallet_vesting, Vesting::<Runtime>);
			list_benchmark!(list, extra, pallet_parachain_staking, ParachainStaking::<Runtime>);
			list_benchmark!(list, extra, pallet_xcmp_handler, XcmpHandler::<Runtime>);

			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use hex_literal::hex;
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

			use pallet_automation_time::Pallet as AutomationTime;
			use pallet_valve::Pallet as Valve;
			use pallet_vesting::Pallet as Vesting;
			use pallet_parachain_staking::Pallet as ParachainStaking;
			use pallet_xcmp_handler::Pallet as XcmpHandler;

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, pallet_automation_time, AutomationTime::<Runtime>);
			add_benchmark!(params, batches, pallet_valve, Valve::<Runtime>);
			add_benchmark!(params, batches, pallet_vesting, Vesting::<Runtime>);
			add_benchmark!(params, batches, pallet_parachain_staking, ParachainStaking::<Runtime>);
			add_benchmark!(params, batches, pallet_xcmp_handler, XcmpHandler::<Runtime>);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block,
		relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot = relay_state_proof
			.read_slot()
			.expect("Could not read the relay chain slot from the proof");

		let inherent_data =
			cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
				relay_chain_slot,
				sp_std::time::Duration::from_secs(6),
			)
			.create_inherent_data()
			.expect("Could not create the timestamp inherent data");

		inherent_data.check_extrinsics(block)
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
}
