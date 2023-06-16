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

//! # XCMP Handler pallet
//!
//! This pallet is used to send XCM Transact messages to other chains.
//! In order to do that it needs to keep track of what tokens other chains accept,
//! and the relevant rates.
//!
//! At this moment we only support using our native currency. We are looking into supporting
//! other chain's native currency and then any currency.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod migrations;
pub mod weights;
pub use weights::WeightInfo;

use cumulus_primitives_core::ParaId;
use frame_support::pallet_prelude::*;
use xcm::{latest::prelude::*, VersionedMultiLocation};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo, traits::Currency,
		weights::constants::WEIGHT_REF_TIME_PER_SECOND,
	};
	use frame_system::pallet_prelude::*;
	use polkadot_parachain::primitives::Sibling;
	use sp_runtime::traits::{AccountIdConversion, Convert, SaturatedConversion};
	use sp_std::prelude::*;
	use xcm_executor::traits::WeightBounds;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type RuntimeCall: From<Call<Self>> + Encode;

		/// The Currency type for interacting with balances
		type Currency: Currency<Self::AccountId>;

		/// The currencyIds that our chain supports.
		type CurrencyId: Parameter
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ Ord
			+ TypeInfo
			+ MaxEncodedLen;

		/// The currencyId for the native currency.
		#[pallet::constant]
		type GetNativeCurrencyId: Get<Self::CurrencyId>;

		//The paraId of this chain.
		type SelfParaId: Get<ParaId>;

		/// Convert an accountId to a multilocation.
		type AccountIdToMultiLocation: Convert<Self::AccountId, MultiLocation>;

		/// Convert a CurrencyId to a MultiLocation.
		type CurrencyIdToMultiLocation: Convert<Self::CurrencyId, Option<MultiLocation>>;

		/// This chain's Universal Location.
		type UniversalLocation: Get<InteriorMultiLocation>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// Utility for sending XCM messages.
		type XcmSender: SendXcm;

		/// Utility for executing XCM instructions.
		type XcmExecutor: ExecuteXcm<<Self as pallet::Config>::RuntimeCall>;

		/// Utility for determining XCM instruction weights.
		type Weigher: WeightBounds<<Self as pallet::Config>::RuntimeCall>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// XCM sent to target chain.
		XcmSent {
			destination: MultiLocation,
		},
		/// XCM transacted in local chain.
		XcmTransactedLocally,
		/// XCM fees successfully paid.
		XcmFeesPaid {
			source: T::AccountId,
			dest: T::AccountId,
		},
		/// XCM fees failed to transfer.
		XcmFeesFailed {
			source: T::AccountId,
			dest: T::AccountId,
			error: DispatchError,
		},
		DestAssetConfigChanged {
			asset_location: MultiLocation,
		},
		DestAssetConfigRemoved {
			asset_location: MultiLocation,
		},
		TransactInfoChanged {
			asset_location: MultiLocation,
		},
		TransactInfoRemoved {
			asset_location: MultiLocation,
		},
		DestinationAssetFeePerSecondChanged {
			destination: MultiLocation,
			asset_location: MultiLocation,
		},
		DestinationAssetFeePerSecondRemoved {
			destination: MultiLocation,
			asset_location: MultiLocation,
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Either the weight or fee per second is too large.
		FeeOverflow,
		/// Either the instruction weight or the transact weight is too large.
		WeightOverflow,
		/// Failed when creating the multilocation for descend origin.
		FailedMultiLocationToJunction,
		/// Unable to reanchor the asset.
		CannotReanchor,
		/// Failed to send XCM to target.
		ErrorSendingXcmToTarget,
		/// Failed to execute XCM in local chain.
		XcmExecutionFailed,
		/// Failed to get weight of call.
		ErrorGettingCallWeight,
		/// The version of the `VersionedMultiLocation` value used is not able
		/// to be interpreted.
		BadVersion,
		// Asset not found
		AssetNotFound,
	}

	/// Stores all configuration needed to send an XCM message for a given asset location.
	#[derive(Clone, Debug, Encode, Decode, PartialEq, TypeInfo)]
	pub struct XcmAssetConfig {
		pub fee_per_second: u128,
		/// The UnitWeightCost of a single instruction on the target chain
		pub instruction_weight: Weight,
		/// The desired instruction flow for the target chain
		pub flow: XcmFlow,
	}

	/// Stores all configuration needed to send an XCM message for a given asset location.
	#[derive(Clone, Debug, Encode, Decode, PartialEq, TypeInfo)]
	pub struct TransactInfo {
		/// The UnitWeightCost of a single instruction on the target chain
		pub instruction_weight: Weight,
		/// The desired instruction flow for the target chain
		pub flow: XcmFlow,
	}


	/// Stores the config for an asset in its reserve chain.
	#[pallet::storage]
	#[pallet::getter(fn dest_asset_config)]
	pub type DestinationAssetConfig<T: Config> =
		StorageMap<_, Twox64Concat, MultiLocation, XcmAssetConfig>;

	#[pallet::storage]
	#[pallet::getter(fn transact_info_with_weight_limit)]
	pub type TransactInfoWithWeightLimit<T: Config> =
		StorageMap<_, Twox64Concat, MultiLocation, TransactInfo>;

	#[pallet::storage]
	#[pallet::getter(fn dest_asset_fee_per_second)]
	pub type DestinationAssetFeePerSecond<T: Config> =
		StorageDoubleMap<_, Twox64Concat, MultiLocation, Twox64Concat, MultiLocation, u128>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set transact info for a given asset location
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn set_transact_info(
			origin: OriginFor<T>,
			asset_location: Box<VersionedMultiLocation>,
			transact_info: TransactInfo,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let asset_location =
				MultiLocation::try_from(*asset_location).map_err(|()| Error::<T>::BadVersion)?;

			TransactInfoWithWeightLimit::<T>::insert(&asset_location, &transact_info);

			Self::deposit_event(Event::TransactInfoChanged { asset_location });

			Ok(().into())
		}

		/// Remove transact info for a given asset location
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn remove_transact_info(
			origin: OriginFor<T>,
			asset_location: Box<VersionedMultiLocation>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let asset_location =
				MultiLocation::try_from(*asset_location).map_err(|()| Error::<T>::BadVersion)?;

			TransactInfoWithWeightLimit::<T>::take(&asset_location).ok_or(Error::<T>::AssetNotFound)?;

			Self::deposit_event(Event::TransactInfoRemoved { asset_location });

			Ok(().into())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn set_destination_asset_fee_per_second(
			origin: OriginFor<T>,
			destination: Box<VersionedMultiLocation>,
			asset_location: Box<VersionedMultiLocation>,
			fee_per_second: u128,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let destination =
				MultiLocation::try_from(*destination).map_err(|()| Error::<T>::BadVersion)?;
			let asset_location =
				MultiLocation::try_from(*asset_location).map_err(|()| Error::<T>::BadVersion)?;

			DestinationAssetFeePerSecond::<T>::insert(destination, asset_location, fee_per_second);

			Self::deposit_event(Event::DestinationAssetFeePerSecondChanged { destination, asset_location });

			Ok(().into())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn remove_destination_asset_fee_per_second(
			origin: OriginFor<T>,
			destination: Box<VersionedMultiLocation>,
			asset_location: Box<VersionedMultiLocation>
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let destination =
				MultiLocation::try_from(*destination).map_err(|()| Error::<T>::BadVersion)?;
			let asset_location =
				MultiLocation::try_from(*asset_location).map_err(|()| Error::<T>::BadVersion)?;

			DestinationAssetFeePerSecond::<T>::take(&destination, &asset_location).ok_or(Error::<T>::AssetNotFound)?;

			Self::deposit_event(Event::DestinationAssetFeePerSecondRemoved { destination, asset_location });

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Get the xcm fee and weight for a transact xcm for a given asset location.
		pub fn calculate_xcm_fee_and_weight(
			destination: MultiLocation,
			xcm_asset_location: MultiLocation,
			transact_encoded_call_weight: Weight,
		) -> Result<(u128, Weight, TransactInfo), DispatchError> {
			let transact_info = TransactInfoWithWeightLimit::<T>::get(destination.clone())
				.ok_or(Error::<T>::AssetNotFound)?;

			let fee_per_second = DestinationAssetFeePerSecond::<T>::get(destination.clone(), xcm_asset_location.clone())
				.ok_or(Error::<T>::AssetNotFound)?;

			let (_, target_instructions) =
				Self::xcm_instruction_skeleton(destination.clone(), xcm_asset_location, transact_info.clone())?;
			let weight = transact_info
				.instruction_weight
				.checked_mul(target_instructions.len() as u64)
				.ok_or(Error::<T>::WeightOverflow)?
				.checked_add(&transact_encoded_call_weight)
				.ok_or(Error::<T>::WeightOverflow)?;

			let fee = fee_per_second
				.checked_mul(weight.ref_time() as u128)
				.ok_or(Error::<T>::FeeOverflow)
				.map(|raw_fee| raw_fee / (WEIGHT_REF_TIME_PER_SECOND as u128))?;

			Ok((fee, weight, transact_info))
		}

		/// Get the instructions for a transact xcm.
		/// Currently we only support instructions if the currency is the local chain's.
		///
		/// Returns two instructions sets.
		/// The first is to execute locally.
		/// The second is to execute on the target chain.
		pub fn get_instruction_set(
			destination: MultiLocation,
			asset_location: MultiLocation,
			caller: T::AccountId,
			transact_encoded_call: Vec<u8>,
			transact_encoded_call_weight: Weight,
		) -> Result<
			(xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>, xcm::latest::Xcm<()>),
			DispatchError,
		> {
			let (fee, weight, xcm_data) = Self::calculate_xcm_fee_and_weight(
				destination,
				asset_location.clone(),
				transact_encoded_call_weight,
			)?;

			let descend_location: Junctions = T::AccountIdToMultiLocation::convert(caller)
				.try_into()
				.map_err(|_| Error::<T>::FailedMultiLocationToJunction)?;

			let instructions = match xcm_data.flow {
				XcmFlow::Normal => Self::get_local_currency_instructions(
					destination,
					asset_location,
					descend_location,
					transact_encoded_call,
					transact_encoded_call_weight,
					weight,
					fee,
				)?,
				XcmFlow::Alternate => Self::get_alternate_flow_instructions(
					destination,
					asset_location,
					descend_location,
					transact_encoded_call,
					transact_encoded_call_weight,
					weight,
					fee,
				)?,
			};

			Ok(instructions)
		}

		/// Construct the instructions for a transact xcm with our local currency.
		///
		/// Local instructions
		/// 	- WithdrawAsset
		/// 	- DepositAsset
		///
		/// Target instructions
		/// 	- ReserveAssetDeposited
		/// 	- BuyExecution
		/// 	- DescendOrigin
		/// 	- Transact
		/// 	- RefundSurplus
		/// 	- DepositAsset
		pub fn get_local_currency_instructions(
			destination: MultiLocation,
			asset_location: MultiLocation,
			descend_location: Junctions,
			transact_encoded_call: Vec<u8>,
			transact_encoded_call_weight: Weight,
			xcm_weight: Weight,
			fee: u128,
		) -> Result<
			(xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>, xcm::latest::Xcm<()>),
			DispatchError,
		> {
			// XCM for local chain
			let local_asset = MultiAsset {
				id: Concrete(asset_location),
				fun: Fungibility::Fungible(fee),
			};

			let local_xcm = Xcm(vec![
				WithdrawAsset::<<T as pallet::Config>::RuntimeCall>(local_asset.clone().into()),
				DepositAsset::<<T as pallet::Config>::RuntimeCall> {
					assets: Wild(All),
					beneficiary: destination,
				},
			]);

			// XCM for target chain
			let target_asset = local_asset
				.reanchored(
					&destination,
					T::UniversalLocation::get(),
				)
				.map_err(|_| Error::<T>::CannotReanchor)?;

			let target_xcm = Xcm(vec![
				ReserveAssetDeposited::<()>(target_asset.clone().into()),
				BuyExecution::<()> { fees: target_asset, weight_limit: Limited(xcm_weight) },
				DescendOrigin::<()>(descend_location),
				Transact::<()> {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: transact_encoded_call_weight,
					call: transact_encoded_call.into(),
				},
				RefundSurplus::<()>,
				DepositAsset::<()> {
					assets: Wild(AllCounted(1)),
					beneficiary: MultiLocation {
						parents: 1,
						interior: X1(Parachain(T::SelfParaId::get().into())),
					},
				},
			]);

			Ok((local_xcm, target_xcm))
		}

		/// Construct the alternate xcm flow instructions
		///
		/// There are no local instructions since the user's account is already funded on the target chain
		///
		/// Target instructions
		/// 	- DescendOrigin
		///     - WithdrawAsset
		/// 	- BuyExecution
		/// 	- Transact
		/// 	- RefundSurplus
		/// 	- DepositAsset
		fn get_alternate_flow_instructions(
			destination: MultiLocation,
			asset_location: MultiLocation,
			descend_location: Junctions,
			transact_encoded_call: Vec<u8>,
			transact_encoded_call_weight: Weight,
			xcm_weight: Weight,
			fee: u128,
		) -> Result<
			(xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>, xcm::latest::Xcm<()>),
			DispatchError,
		> {
			// XCM for target chain
			let target_asset =
				MultiAsset { id: Concrete(asset_location), fun: Fungibility::Fungible(fee) }
					.reanchored(
						&destination,
						T::UniversalLocation::get(),
					)
					.map_err(|_| Error::<T>::CannotReanchor)?;

			let target_xcm = Xcm(vec![
				DescendOrigin::<()>(descend_location.clone()),
				WithdrawAsset::<()>(target_asset.clone().into()),
				BuyExecution::<()> { fees: target_asset, weight_limit: Limited(xcm_weight) },
				Transact::<()> {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: transact_encoded_call_weight,
					call: transact_encoded_call.into(),
				},
				RefundSurplus::<()>,
				DepositAsset::<()> {
					assets: Wild(AllCounted(1)),
					beneficiary: MultiLocation { parents: 1, interior: descend_location },
				},
			]);

			Ok((Xcm(vec![]), target_xcm))
		}

		/// Transact XCM instructions on local chain
		///
		pub fn transact_in_local_chain(
			internal_instructions: xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>,
		) -> Result<(), DispatchError> {
			let local_sovereign_account =
				MultiLocation::new(1, X1(Parachain(T::SelfParaId::get().into())));
			let weight = T::Weigher::weight(&mut internal_instructions.clone().into())
				.map_err(|_| Error::<T>::ErrorGettingCallWeight)?;
			let hash = internal_instructions.using_encoded(sp_io::hashing::blake2_256);

			// Execute instruction on local chain
			T::XcmExecutor::execute_xcm_in_credit(
				local_sovereign_account,
				internal_instructions.into(),
				hash,
				weight,
				weight,
			)
			.ensure_complete()
			.map_err(|error| {
				log::error!("Failed execute in credit with {:?}", error);
				Error::<T>::XcmExecutionFailed
			})?;

			Self::deposit_event(Event::XcmTransactedLocally);

			Ok(().into())
		}

		/// Send XCM instructions to parachain.
		///
		pub fn transact_in_target_chain(
			destination: MultiLocation,
			target_instructions: xcm::latest::Xcm<()>,
		) -> Result<(), DispatchError> {
			#[allow(unused_variables)]
			let destination_location = destination.clone();

			#[cfg(all(not(test), feature = "runtime-benchmarks"))]
			let destination_location = MultiLocation::new(1, Here);

			// Send to target chain
			send_xcm::<T::XcmSender>(
				destination_location,
				target_instructions,
			)
			.map_err(|error| {
				log::error!("Failed to send xcm to {:?} with {:?}", destination, error);
				Error::<T>::ErrorSendingXcmToTarget
			})?;

			Self::deposit_event(Event::XcmSent { destination });

			Ok(().into())
		}

		/// Create and transact instructions.
		/// Currently we only support if the currency is the local chain's.
		///
		/// Get the instructions for a transact xcm.
		/// Execute local transact instructions.
		/// Send target transact instructions.
		pub fn transact_xcm(
			destination: MultiLocation,
			asset_location: MultiLocation,
			caller: T::AccountId,
			transact_encoded_call: Vec<u8>,
			transact_encoded_call_weight: Weight,
		) -> Result<(), DispatchError> {
			let (local_instructions, target_instructions) = Self::get_instruction_set(
				destination,
				asset_location,
				caller,
				transact_encoded_call,
				transact_encoded_call_weight,
			)?;

			Self::transact_in_local_chain(local_instructions)?;
			Self::transact_in_target_chain(destination, target_instructions)?;

			Ok(().into())
		}

		/// Pay for XCMP fees.
		/// Transfers fee from payer account to the local chain sovereign account.
		///
		pub fn pay_xcm_fee(source: T::AccountId, fee: u128) -> Result<(), DispatchError> {
			let local_sovereign_account =
				Sibling::from(T::SelfParaId::get()).into_account_truncating();

			match T::Currency::transfer(
				&source,
				&local_sovereign_account,
				<BalanceOf<T>>::saturated_from(fee),
				frame_support::traits::ExistenceRequirement::KeepAlive,
			) {
				Ok(_number) => Self::deposit_event(Event::XcmFeesPaid {
					source,
					dest: local_sovereign_account,
				}),
				Err(e) => Self::deposit_event(Event::XcmFeesFailed {
					source,
					dest: local_sovereign_account,
					error: e,
				}),
			};

			Ok(().into())
		}

		/// Generates a skeleton of the instruction set for fee calculation
		fn xcm_instruction_skeleton(
			destination: MultiLocation,
			asset_location: MultiLocation,
			transact_info: TransactInfo,
		) -> Result<
			(xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>, xcm::latest::Xcm<()>),
			DispatchError,
		> {
			let nobody: Junctions = T::AccountIdToMultiLocation::convert(
				T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
					.expect("always works"),
			)
			.try_into()
			.map_err(|_| Error::<T>::FailedMultiLocationToJunction)?;

			match transact_info.flow {
				XcmFlow::Normal => Self::get_local_currency_instructions(
					destination,
					asset_location,
					nobody,
					Default::default(),
					Weight::zero(),
					Weight::zero(),
					0u128,
				),
				XcmFlow::Alternate => Self::get_alternate_flow_instructions(
					destination,
					asset_location,
					nobody,
					Default::default(),
					Weight::zero(),
					Weight::zero(),
					0u128,
				),
			}
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub asset_data: Vec<(Vec<u8>, Weight, XcmFlow)>,
		pub fees: Vec<(Vec<u8>, Vec<u8>, u128)>,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self { asset_data: Default::default(), fees: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			for (location_encoded, instruction_weight, flow) in
				self.asset_data.iter()
			{
				let location = <VersionedMultiLocation>::decode(&mut &location_encoded[..])
					.expect("Error decoding VersionedMultiLocation");
				let location = MultiLocation::try_from(location)
					.expect("Error converting VersionedMultiLocation");

				TransactInfoWithWeightLimit::<T>::insert(
					location,
					TransactInfo {
						instruction_weight: *instruction_weight,
						flow: *flow,
					},
				);
			}

			for (destination_encoded, asset_location_encoded, fee_per_second) in
				self.fees.iter()
			{
				let destination = <VersionedMultiLocation>::decode(&mut &destination_encoded[..])
					.expect("Error decoding VersionedMultiLocation");
				let destination = MultiLocation::try_from(destination)
					.expect("Error converting VersionedMultiLocation");

				let asset_location = <VersionedMultiLocation>::decode(&mut &asset_location_encoded[..])
					.expect("Error decoding VersionedMultiLocation");
				let asset_location = MultiLocation::try_from(asset_location)
					.expect("Error converting VersionedMultiLocation");

				DestinationAssetFeePerSecond::<T>::insert(
					destination,
					asset_location,
					fee_per_second,
				);
			}
		}
	}
}

pub trait XcmpTransactor<AccountId, CurrencyId> {
	fn transact_xcm(
		destination: MultiLocation,
		asset_location: MultiLocation,
		caller: AccountId,
		transact_encoded_call: sp_std::vec::Vec<u8>,
		transact_encoded_call_weight: Weight,
	) -> Result<(), sp_runtime::DispatchError>;

	fn get_xcm_fee(
		destination: MultiLocation,
		asset_location: MultiLocation,
		transact_encoded_call_weight: Weight,
	) -> Result<u128, sp_runtime::DispatchError>;

	fn pay_xcm_fee(source: AccountId, fee: u128) -> Result<(), sp_runtime::DispatchError>;

	#[cfg(feature = "runtime-benchmarks")]
	fn setup_chain_asset_data(
		asset_location: MultiLocation,
	) -> Result<(), sp_runtime::DispatchError>;
}

impl<T: Config> XcmpTransactor<T::AccountId, T::CurrencyId> for Pallet<T> {
	fn transact_xcm(
		destination: MultiLocation,
		asset_location: MultiLocation,
		caller: T::AccountId,
		transact_encoded_call: sp_std::vec::Vec<u8>,
		transact_encoded_call_weight: Weight,
	) -> Result<(), sp_runtime::DispatchError> {
		Self::transact_xcm(
			destination,
			asset_location,
			caller,
			transact_encoded_call,
			transact_encoded_call_weight,
		)?;

		Ok(()).into()
	}

	fn get_xcm_fee(
		destination: MultiLocation,
		asset_location: MultiLocation,
		transact_encoded_call_weight: Weight,
	) -> Result<u128, sp_runtime::DispatchError> {
		let (fee, _weight, xcm_data) =
			Self::calculate_xcm_fee_and_weight(destination, asset_location, transact_encoded_call_weight)?;

		match xcm_data.flow {
			XcmFlow::Alternate => {
				// In the alternate flow the fee is paid directly from the
				// DescendOrigin derived account on the target chain
				Ok(0u128)
			},
			XcmFlow::Normal => Ok(fee),
		}
	}

	fn pay_xcm_fee(source: T::AccountId, fee: u128) -> Result<(), sp_runtime::DispatchError> {
		Self::pay_xcm_fee(source, fee)?;

		Ok(()).into()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn setup_chain_asset_data(
		asset_location: MultiLocation,
	) -> Result<(), sp_runtime::DispatchError> {
		let asset_data = XcmAssetConfig {
			fee_per_second: 416_000_000_000,
			instruction_weight: Weight::from_ref_time(600_000_000),
			flow: XcmFlow::Normal,
		};

		DestinationAssetConfig::<T>::insert(asset_location.clone(), asset_data);
		Self::deposit_event(Event::DestAssetConfigChanged { asset_location });

		Ok(().into())
	}
}

#[derive(Clone, Copy, Debug, Encode, Decode, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum XcmFlow {
	Normal,
	Alternate,
}
