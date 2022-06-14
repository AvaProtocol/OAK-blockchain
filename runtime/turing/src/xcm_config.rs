use super::{
	AccountId, Balance, Call, Currencies, CurrencyId, Event, Origin, ParachainInfo,
	ParachainSystem, PolkadotXcm, Runtime, TemporaryForeignTreasuryAccount, TreasuryAccount,
	UnknownTokens, XcmpQueue, MAXIMUM_BLOCK_WEIGHT,
};

use core::marker::PhantomData;
use frame_support::{
	match_types, parameter_types,
	traits::{Everything, Nothing},
	weights::Weight,
};
use frame_system::EnsureRoot;
use sp_runtime::{traits::Convert, Percent};

// Polkadot Imports
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;

// XCM Imports
use xcm::{latest::prelude::*, v1::Junction::Parachain};
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, EnsureXcmOrigin, FixedRateOfFungible,
	FixedWeightBounds, LocationInverter, ParentIsPreset, RelayChainAsNative,
	SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
	SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue, TakeWeightCredit,
};
use xcm_executor::{traits::ShouldExecute, Config, XcmExecutor};

// ORML imports
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use orml_xcm_support::{
	DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset,
};

// Common imports
use primitives::tokens::{convert_to_token, TokenInfo};

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Any;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type LocalAssetTransactor = MultiCurrencyAdapter<
	Currencies,
	UnknownTokens,
	IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
	AccountId,
	LocationToAccountId,
	CurrencyId,
	CurrencyIdConvert,
	DepositToAlternative<TreasuryAccount, Currencies, CurrencyId, AccountId, Balance>,
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

//TODO: move DenyThenTry to polkadot's xcm module.
/// Deny executing the xcm message if it matches any of the Deny filter regardless of anything else.
/// If it passes the Deny, and matches one of the Allow cases then it is let through.
pub struct DenyThenTry<Deny, Allow>(PhantomData<Deny>, PhantomData<Allow>)
where
	Deny: ShouldExecute,
	Allow: ShouldExecute;

impl<Deny, Allow> ShouldExecute for DenyThenTry<Deny, Allow>
where
	Deny: ShouldExecute,
	Allow: ShouldExecute,
{
	fn should_execute<Call>(
		origin: &MultiLocation,
		message: &mut Xcm<Call>,
		max_weight: Weight,
		weight_credit: &mut Weight,
	) -> Result<(), ()> {
		Deny::should_execute(origin, message, max_weight, weight_credit)?;
		Allow::should_execute(origin, message, max_weight, weight_credit)
	}
}

// See issue #5233
pub struct DenyReserveTransferToRelayChain;
impl ShouldExecute for DenyReserveTransferToRelayChain {
	fn should_execute<Call>(
		origin: &MultiLocation,
		message: &mut Xcm<Call>,
		_max_weight: Weight,
		_weight_credit: &mut Weight,
	) -> Result<(), ()> {
		if message.0.iter().any(|inst| {
			matches!(
				inst,
				InitiateReserveWithdraw {
					reserve: MultiLocation { parents: 1, interior: Here },
					..
				} | DepositReserveAsset { dest: MultiLocation { parents: 1, interior: Here }, .. } |
					TransferReserveAsset {
						dest: MultiLocation { parents: 1, interior: Here },
						..
					}
			)
		}) {
			return Err(()) // Deny
		}

		// allow reserve transfers to arrive from relay chain
		if matches!(origin, MultiLocation { parents: 1, interior: Here }) &&
			message.0.iter().any(|inst| matches!(inst, ReserveAssetDeposited { .. }))
		{
			log::warn!(
				target: "xcm::barriers",
				"Unexpected ReserveAssetDeposited from the relay chain",
			);
		}
		// Permit everything else
		Ok(())
	}
}

pub type Barrier = DenyThenTry<
	DenyReserveTransferToRelayChain,
	(
		TakeWeightCredit,
		AllowTopLevelPaidExecutionFrom<Everything>,
		// Expected responses are OK.
		AllowKnownQueryResponses<PolkadotXcm>,
		// Subscriptions for version tracking are OK.
		AllowSubscriptionsFrom<Everything>,
		// Parent and its exec plurality get free execution
		AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
	),
>;

/// Based on the precedent set by other projects. This will need to be changed.
pub fn ksm_per_second() -> u128 {
	CurrencyId::KSM.cent() * 16
}

/// Assuming ~ $0.50 TUR price.
pub fn tur_per_second() -> u128 {
	let tur_equivalent = convert_to_token(CurrencyId::KSM, CurrencyId::Native, ksm_per_second());
	// Assuming KSM ~ $130.00.
	tur_equivalent * 260
}

parameter_types! {
	pub TurPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X1(Parachain(u32::from(ParachainInfo::parachain_id()))),
		).into(),
		tur_per_second()
	);

	pub TurCanonicalPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			0,
			Here,
		).into(),
		tur_per_second()
	);

	pub KsmPerSecond: (AssetId, u128) = (MultiLocation::parent().into(), ksm_per_second());

	pub KarPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), GeneralKey(parachains::karura::KAR_KEY.to_vec())),
		).into(),
		//KAR:KSM 50:1
		ksm_per_second() * 50
	);

	pub AusdPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), GeneralKey(parachains::karura::AUSD_KEY.to_vec())),
		).into(),
		// AUSD:KSM = 400:1
		ksm_per_second() * 400
	);

	pub LksmPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), GeneralKey(parachains::karura::LKSM_KEY.to_vec())),
		).into(),
		// LKSM:KSM = 10:1
		ksm_per_second() * 10
	);

	pub HkoPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::heiko::ID), GeneralKey(parachains::heiko::HKO_KEY.to_vec())),
		).into(),
		// HKO:KSM = 30:1
		ksm_per_second() * 30
	);

	pub SksmPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::heiko::ID), GeneralKey(parachains::heiko::SKSM_KEY.to_vec())),
		).into(),
		// sKSM:KSM = 1:1
		ksm_per_second()
	);

	pub PhaPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X1(Parachain(parachains::khala::ID)),
		).into(),
		// PHA:KSM = 400:1
		ksm_per_second() * 400
	);
}

pub struct ToNativeTreasury;
impl TakeRevenue for ToNativeTreasury {
	fn take_revenue(revenue: MultiAsset) {
		if let MultiAsset { id: AssetId::Concrete(id), fun: Fungibility::Fungible(amount) } =
			revenue
		{
			if let Some(currency_id) = CurrencyIdConvert::convert(id) {
				// 80% burned, 20% to the treasury
				let to_treasury = Percent::from_percent(20).mul_floor(amount);
				// Due to the way XCM works the amount has already been taken off the total allocation balance.
				// Thus whatever we deposit here gets added back to the total allocation, and the rest is burned.
				let _ = Currencies::deposit(currency_id, &TreasuryAccount::get(), to_treasury);
			}
		}
	}
}

pub struct ToForeignTreasury;
impl TakeRevenue for ToForeignTreasury {
	fn take_revenue(revenue: MultiAsset) {
		if let MultiAsset { id: AssetId::Concrete(id), fun: Fungibility::Fungible(amount) } =
			revenue
		{
			if let Some(currency_id) = CurrencyIdConvert::convert(id) {
				let _ = Currencies::deposit(
					currency_id,
					&TemporaryForeignTreasuryAccount::get(),
					amount,
				);
			}
		}
	}
}

pub type Trader = (
	FixedRateOfFungible<TurPerSecond, ToNativeTreasury>,
	FixedRateOfFungible<TurCanonicalPerSecond, ToNativeTreasury>,
	FixedRateOfFungible<KsmPerSecond, ToForeignTreasury>,
	FixedRateOfFungible<KarPerSecond, ToForeignTreasury>,
	FixedRateOfFungible<AusdPerSecond, ToForeignTreasury>,
	FixedRateOfFungible<LksmPerSecond, ToForeignTreasury>,
	FixedRateOfFungible<HkoPerSecond, ToForeignTreasury>,
	FixedRateOfFungible<SksmPerSecond, ToForeignTreasury>,
	FixedRateOfFungible<PhaPerSecond, ToForeignTreasury>,
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = (); // Teleporting is disabled.
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader = Trader;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
}

parameter_types! {
	pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	// ^ Disable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing.
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Nothing;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type Origin = Origin;
	type Call = Call;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	// ^ Override for AdvertisedXcmVersion default
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = ();
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Self>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::parachain_id().into())));
	pub const BaseXcmWeight: Weight = 100_000_000;
	pub const MaxAssetsForTransfer: usize = 1;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> u128 {
		u128::MAX
	};
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = SelfLocation;
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type MultiLocationsFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type LocationInverter = LocationInverter<Ancestry>;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = AbsoluteReserveProvider;
}

impl orml_unknown_tokens::Config for Runtime {
	type Event = Event;
}

pub mod parachains {

	pub mod heiko {
		pub const ID: u32 = 2085;
		pub const HKO_KEY: &[u8] = b"HKO";
		pub const SKSM_KEY: &[u8] = b"sKSM";
	}

	pub mod karura {
		pub const ID: u32 = 2000;
		pub const KAR_KEY: &[u8] = &[0, 128];
		pub const AUSD_KEY: &[u8] = &[0, 129];
		pub const LKSM_KEY: &[u8] = &[0, 131];
	}

	pub mod mangata {
		pub const ID: u32 = 2110;
	}

	pub mod khala {
		pub const ID: u32 = 2004;
	}
}

pub struct CurrencyIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		match id {
			CurrencyId::Native =>
				Some(MultiLocation::new(1, X1(Parachain(ParachainInfo::parachain_id().into())))),
			CurrencyId::KSM => Some(MultiLocation::parent()),
			CurrencyId::AUSD => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey(parachains::karura::AUSD_KEY.to_vec()),
				),
			)),
			CurrencyId::KAR => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey(parachains::karura::KAR_KEY.to_vec()),
				),
			)),
			CurrencyId::LKSM => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey(parachains::karura::LKSM_KEY.to_vec()),
				),
			)),
			CurrencyId::HKO => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::heiko::ID),
					GeneralKey(parachains::heiko::HKO_KEY.to_vec()),
				),
			)),
			CurrencyId::SKSM => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::heiko::ID),
					GeneralKey(parachains::heiko::SKSM_KEY.to_vec()),
				),
			)),
			CurrencyId::PHA => Some(MultiLocation::new(1, X1(Parachain(parachains::khala::ID))))
		}
	}
}

impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		if location == MultiLocation::parent() {
			return Some(CurrencyId::KSM)
		}

		match location {
			MultiLocation { parents: 1, interior: X2(Parachain(para_id), GeneralKey(key)) } =>
				match (para_id, &key[..]) {
					(parachains::karura::ID, parachains::karura::KAR_KEY) => Some(CurrencyId::KAR),
					(parachains::karura::ID, parachains::karura::AUSD_KEY) =>
						Some(CurrencyId::AUSD),
					(parachains::karura::ID, parachains::karura::LKSM_KEY) =>
						Some(CurrencyId::LKSM),
					(parachains::heiko::ID, parachains::heiko::HKO_KEY) => Some(CurrencyId::HKO),
					(parachains::heiko::ID, parachains::heiko::SKSM_KEY) => Some(CurrencyId::SKSM),
					_ => None,
				},
			MultiLocation { parents: 1, interior: X1(Parachain(para_id)) } => {
				match para_id {
					// If it's TUR
					id if id == u32::from(ParachainInfo::parachain_id()) =>
						Some(CurrencyId::Native),
					id if id == parachains::khala::ID => 
						Some(CurrencyId::PHA),
					_ => None,
				}
			},
			// adapt for re-anchor canonical location: https://github.com/paritytech/polkadot/pull/4470
			MultiLocation { parents: 0, interior: Here } => Some(CurrencyId::Native),
			_ => None,
		}
	}
}

impl Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(asset: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset { id: Concrete(location), .. } = asset {
			Self::convert(location)
		} else {
			None
		}
	}
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 { network: NetworkId::Any, id: account.into() }).into()
	}
}
