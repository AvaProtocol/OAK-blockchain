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

pub mod migrations;

use cumulus_primitives_core::ParaId;
use frame_support::pallet_prelude::*;
use xcm::latest::prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use orml_traits::{location::Reserve, MultiCurrency};
	use polkadot_parachain::primitives::Sibling;
	use sp_runtime::{
		traits::{AccountIdConversion, CheckedSub, Convert, SaturatedConversion},
		TokenError::BelowMinimum,
	};
	use sp_std::prelude::*;
	use xcm_executor::traits::WeightBounds;

	pub type MultiCurrencyId<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::CurrencyId;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type RuntimeCall: From<Call<Self>> + Encode;

		/// The MultiCurrency type for interacting with balances
		type MultiCurrency: MultiCurrency<Self::AccountId>;

		/// The currencyIds that our chain supports.
		type CurrencyId: Parameter
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ Ord
			+ TypeInfo
			+ MaxEncodedLen
			+ From<MultiCurrencyId<Self>>
			+ Into<MultiCurrencyId<Self>>;

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

		/// Utility for sending XCM messages.
		type XcmSender: SendXcm;

		/// Utility for executing XCM instructions.
		type XcmExecutor: ExecuteXcm<<Self as pallet::Config>::RuntimeCall>;

		/// Utility for determining XCM instruction weights.
		type Weigher: WeightBounds<<Self as pallet::Config>::RuntimeCall>;

		/// The way to retreave the reserve of a MultiAsset. This can be
		/// configured to accept absolute or relative paths for self tokens
		type ReserveProvider: Reserve;

		/// Self chain location.
		#[pallet::constant]
		type SelfLocation: Get<MultiLocation>;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
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
		TransactInfoChanged {
			destination: MultiLocation,
		},
		TransactInfoRemoved {
			destination: MultiLocation,
		},
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
		TransactInfoNotFound,
		// Invalid asset location.
		InvalidAssetLocation,
		// The fee payment asset location is not supported.
		UnsupportedFeePayment,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		/// Get the instructions for a transact xcm.
		/// Currently we only support instructions if the currency is the local chain's.
		///
		/// Returns two instructions sets.
		/// The first is to execute locally.
		/// The second is to execute on the target chain.
		pub fn get_instruction_set(
			destination: MultiLocation,
			asset_location: MultiLocation,
			fee: u128,
			caller: T::AccountId,
			transact_encoded_call: Vec<u8>,
			transact_encoded_call_weight: Weight,
			overall_weight: Weight,
			flow: InstructionSequence,
		) -> Result<
			(xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>, xcm::latest::Xcm<()>),
			DispatchError,
		> {
			let descend_location: Junctions = T::AccountIdToMultiLocation::convert(caller)
				.try_into()
				.map_err(|_| Error::<T>::FailedMultiLocationToJunction)?;

			let instructions = match flow {
				InstructionSequence::PayThroughSovereignAccount =>
					Self::get_local_currency_instructions(
						destination,
						asset_location,
						descend_location,
						transact_encoded_call,
						transact_encoded_call_weight,
						overall_weight,
						fee,
					)?,
				InstructionSequence::PayThroughRemoteDerivativeAccount =>
					Self::get_alternate_flow_instructions(
						destination,
						asset_location,
						descend_location,
						transact_encoded_call,
						transact_encoded_call_weight,
						overall_weight,
						fee,
					)?,
			};

			Ok(instructions)
		}

		fn get_local_xcm(
			asset: MultiAsset,
			destination: MultiLocation,
		) -> Result<xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>, DispatchError> {
			let reserve =
				T::ReserveProvider::reserve(&asset).ok_or(Error::<T>::InvalidAssetLocation)?;
			let local_xcm = if reserve == MultiLocation::here() {
				Xcm(vec![
					WithdrawAsset::<<T as pallet::Config>::RuntimeCall>(asset.into()),
					DepositAsset::<<T as pallet::Config>::RuntimeCall> {
						assets: Wild(All),
						beneficiary: destination,
					},
				])
			} else if reserve == destination {
				Xcm(vec![
					WithdrawAsset::<<T as pallet::Config>::RuntimeCall>(asset.clone().into()),
					BurnAsset::<<T as pallet::Config>::RuntimeCall>(asset.into()),
				])
			} else {
				return Err(Error::<T>::UnsupportedFeePayment.into())
			};

			Ok(local_xcm)
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
			overall_weight: Weight,
			fee: u128,
		) -> Result<
			(xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>, xcm::latest::Xcm<()>),
			DispatchError,
		> {
			let local_asset =
				MultiAsset { id: Concrete(asset_location), fun: Fungibility::Fungible(fee) };

			let target_asset = local_asset
				.clone()
				.reanchored(&destination, T::UniversalLocation::get())
				.map_err(|_| Error::<T>::CannotReanchor)?;

			let reserve = T::ReserveProvider::reserve(&local_asset)
				.ok_or(Error::<T>::InvalidAssetLocation)?;

			let (local_xcm, target_xcm) = if reserve == MultiLocation::here() {
				let local_xcm = Xcm(vec![
					WithdrawAsset::<<T as pallet::Config>::RuntimeCall>(local_asset.into()),
					DepositAsset::<<T as pallet::Config>::RuntimeCall> {
						assets: Wild(All),
						beneficiary: destination,
					},
				]);
				let target_xcm = Xcm(vec![
					ReserveAssetDeposited::<()>(target_asset.clone().into()),
					BuyExecution::<()> {
						fees: target_asset,
						weight_limit: Limited(overall_weight),
					},
					DescendOrigin::<()>(descend_location),
					Transact::<()> {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: transact_encoded_call_weight,
						call: transact_encoded_call.into(),
					},
					RefundSurplus::<()>,
					DepositAsset::<()> {
						assets: Wild(AllCounted(1)),
						beneficiary: T::SelfLocation::get(),
					},
				]);
				(local_xcm, target_xcm)
			} else if reserve == destination {
				let local_xcm = Xcm(vec![
					WithdrawAsset::<<T as pallet::Config>::RuntimeCall>(local_asset.clone().into()),
					BurnAsset::<<T as pallet::Config>::RuntimeCall>(local_asset.into()),
				]);
				let target_xcm = Xcm(vec![
					WithdrawAsset::<()>(target_asset.clone().into()),
					BuyExecution::<()> {
						fees: target_asset,
						weight_limit: Limited(overall_weight),
					},
					DescendOrigin::<()>(descend_location),
					Transact::<()> {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: transact_encoded_call_weight,
						call: transact_encoded_call.into(),
					},
					RefundSurplus::<()>,
					DepositAsset::<()> {
						assets: Wild(AllCounted(1)),
						beneficiary: T::SelfLocation::get(),
					},
				]);
				(local_xcm, target_xcm)
			} else {
				return Err(Error::<T>::UnsupportedFeePayment.into())
			};

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
					.reanchored(&destination, T::UniversalLocation::get())
					.map_err(|_| Error::<T>::CannotReanchor)?;

			let target_xcm = Xcm(vec![
				DescendOrigin::<()>(descend_location),
				WithdrawAsset::<()>(target_asset.clone().into()),
				BuyExecution::<()> { fees: target_asset, weight_limit: Limited(xcm_weight) },
				Transact::<()> {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: transact_encoded_call_weight,
					call: transact_encoded_call.into(),
				},
			]);

			Ok((Xcm(vec![]), target_xcm))
		}

		/// Transact XCM instructions on local chain
		///
		pub fn transact_in_local_chain(
			internal_instructions: xcm::latest::Xcm<<T as pallet::Config>::RuntimeCall>,
		) -> Result<(), DispatchError> {
			let local_sovereign_account = T::SelfLocation::get();
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

			Ok(())
		}

		/// Send XCM instructions to parachain.
		///
		pub fn transact_in_target_chain(
			destination: MultiLocation,
			target_instructions: xcm::latest::Xcm<()>,
		) -> Result<(), DispatchError> {
			#[allow(unused_variables)]
			let destination_location = destination;

			#[cfg(all(not(test), feature = "runtime-benchmarks"))]
			let destination_location = MultiLocation::new(1, Here);

			// Send to target chain
			send_xcm::<T::XcmSender>(destination_location, target_instructions).map_err(
				|error| {
					log::error!("Failed to send xcm to {:?} with {:?}", destination, error);
					Error::<T>::ErrorSendingXcmToTarget
				},
			)?;

			Self::deposit_event(Event::XcmSent { destination });

			Ok(())
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
			fee: u128,
			caller: T::AccountId,
			transact_encoded_call: Vec<u8>,
			transact_encoded_call_weight: Weight,
			overall_weight: Weight,
			flow: InstructionSequence,
		) -> Result<(), DispatchError> {
			let (local_instructions, target_instructions) = Self::get_instruction_set(
				destination,
				asset_location,
				fee,
				caller,
				transact_encoded_call,
				transact_encoded_call_weight,
				overall_weight,
				flow,
			)?;

			Self::transact_in_local_chain(local_instructions)?;
			Self::transact_in_target_chain(destination, target_instructions)?;

			Ok(())
		}

		fn do_pay_xcm_fee(
			currency_id: T::CurrencyId,
			source: T::AccountId,
			dest: T::AccountId,
			fee: u128,
		) -> Result<(), DispatchError> {
			let free_balance = T::MultiCurrency::free_balance(currency_id.into(), &source);
			let min_balance = T::MultiCurrency::minimum_balance(currency_id.into());

			free_balance
				.checked_sub(&fee.saturated_into())
				.and_then(|balance_minus_fee| balance_minus_fee.checked_sub(&min_balance))
				.ok_or(DispatchError::Token(BelowMinimum))?;

			T::MultiCurrency::ensure_can_withdraw(
				currency_id.into(),
				&source,
				fee.saturated_into(),
			)?;

			T::MultiCurrency::transfer(currency_id.into(), &source, &dest, fee.saturated_into())?;

			Ok(())
		}

		/// Pay for XCMP fees.
		/// Transfers fee from payer account to the local chain sovereign account.
		///
		pub fn pay_xcm_fee(
			currency_id: T::CurrencyId,
			source: T::AccountId,
			fee: u128,
		) -> Result<(), DispatchError> {
			let local_sovereign_account: T::AccountId =
				Sibling::from(T::SelfParaId::get()).into_account_truncating();
			match Self::do_pay_xcm_fee(
				currency_id,
				source.clone(),
				local_sovereign_account.clone(),
				fee,
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

			Ok(())
		}
	}
}

pub trait XcmpTransactor<AccountId, CurrencyId> {
	fn transact_xcm(
		destination: MultiLocation,
		asset_location: MultiLocation,
		fee: u128,
		caller: AccountId,
		transact_encoded_call: sp_std::vec::Vec<u8>,
		transact_encoded_call_weight: Weight,
		overall_weight: Weight,
		flow: InstructionSequence,
	) -> Result<(), sp_runtime::DispatchError>;

	fn pay_xcm_fee(
		currency_id: CurrencyId,
		source: AccountId,
		fee: u128,
	) -> Result<(), sp_runtime::DispatchError>;
}

impl<T: Config> XcmpTransactor<T::AccountId, T::CurrencyId> for Pallet<T> {
	fn transact_xcm(
		destination: MultiLocation,
		asset_location: MultiLocation,
		fee: u128,
		caller: T::AccountId,
		transact_encoded_call: sp_std::vec::Vec<u8>,
		transact_encoded_call_weight: Weight,
		overall_weight: Weight,
		flow: InstructionSequence,
	) -> Result<(), sp_runtime::DispatchError> {
		Self::transact_xcm(
			destination,
			asset_location,
			fee,
			caller,
			transact_encoded_call,
			transact_encoded_call_weight,
			overall_weight,
			flow,
		)?;

		Ok(())
	}

	fn pay_xcm_fee(
		currency_id: T::CurrencyId,
		source: T::AccountId,
		fee: u128,
	) -> Result<(), sp_runtime::DispatchError> {
		Self::pay_xcm_fee(currency_id, source, fee)?;

		Ok(())
	}
}

#[derive(Clone, Copy, Debug, Encode, Eq, Decode, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum InstructionSequence {
	PayThroughSovereignAccount,
	PayThroughRemoteDerivativeAccount,
}
