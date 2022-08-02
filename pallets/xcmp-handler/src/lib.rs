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

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use codec::Decode;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo, pallet_prelude::*,
		weights::constants::WEIGHT_PER_SECOND,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::prelude::*;

	type ParachainId = u32;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

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

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// XCM data was added for a chain/currency pair.
		XcmDataAdded { para_id: ParachainId, currency_id: T::CurrencyId },
		/// XCM data was removed for a chain/currency pair.
		XcmDataRemoved { para_id: ParachainId, currency_id: T::CurrencyId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// We only support certain currency/chain combinations.
		CurrencyChainComboNotSupported,
		/// There is no entry for that currency/chain combination.
		CurrencyChainComboNotFound,
		/// Either the weight or fee per second is too large.
		FeeOverflow,
		/// Either the instruction weight or the transact weight is too large.
		WeightOverflow,
	}

	/// Stores all data needed to send an XCM message for chain/currency pair.
	#[derive(Clone, Debug, Encode, Decode, PartialEq, TypeInfo)]
	pub struct XcmCurrencyData {
		/// Is the token native to the chain?
		pub native: bool,
		pub fee_per_second: u128,
		/// The weight of the instructions for the chain/currency pair minus the Transact encoded call.
		/// For example, if the chain is using FixedWeightBounds then the weight is the
		/// number of instructions times the UnitWeightCost. The number of instructions inlcudes the Transact instruction.
		///
		/// FixedWeightBounds link:
		/// (https://github.com/paritytech/polkadot/blob/63b611e8b1c332e4d7aaaa9ebd99d8d40f2a6f49/xcm/xcm-builder/src/weight.rs#L30)
		pub instruction_weight: u64,
	}

	/// Stores XCM data for a chain/currency pair.
	#[pallet::storage]
	#[pallet::getter(fn get_xcm_chain_data)]
	pub type XcmChainCurrencyData<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		ParachainId,
		Twox64Concat,
		T::CurrencyId,
		XcmCurrencyData,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add or update XCM data for a chain/currency pair.
		/// For now we only support our native currency.
		#[pallet::weight(T::WeightInfo::add_chain_currency_data())]
		pub fn add_chain_currency_data(
			origin: OriginFor<T>,
			para_id: ParachainId,
			currency_id: T::CurrencyId,
			xcm_data: XcmCurrencyData,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			if currency_id != T::GetNativeCurrencyId::get() {
				Err(Error::<T>::CurrencyChainComboNotSupported)?
			}

			XcmChainCurrencyData::<T>::insert(para_id, currency_id, xcm_data);
			Self::deposit_event(Event::XcmDataAdded { para_id, currency_id });

			Ok(().into())
		}

		/// Remove XCM data for a chain/currency pair.
		#[pallet::weight(T::WeightInfo::remove_chain_currency_data())]
		pub fn remove_chain_currency_data(
			origin: OriginFor<T>,
			para_id: ParachainId,
			currency_id: T::CurrencyId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			XcmChainCurrencyData::<T>::take(para_id, currency_id)
				.ok_or(Error::<T>::CurrencyChainComboNotFound)?;
			Self::deposit_event(Event::XcmDataRemoved { para_id, currency_id });

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn calculate_xcm_fee(
			para_id: ParachainId,
			currency_id: T::CurrencyId,
			transact_encoded_call_weight: u64,
		) -> Result<u128, DispatchError> {
			let xcm_data = XcmChainCurrencyData::<T>::get(para_id, currency_id)
				.ok_or(Error::<T>::CurrencyChainComboNotFound)?;
			let xcm_weight = xcm_data
				.instruction_weight
				.checked_add(transact_encoded_call_weight)
				.ok_or(Error::<T>::WeightOverflow)?;
			let fee_or_err = xcm_data
				.fee_per_second
				.checked_mul(xcm_weight as u128)
				.ok_or(Error::<T>::FeeOverflow.into())
				.map(|raw_fee| raw_fee / (WEIGHT_PER_SECOND as u128));

			fee_or_err
		}
	}
}
