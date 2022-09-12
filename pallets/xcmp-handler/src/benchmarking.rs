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

#[allow(unused)]
use crate::Pallet as XcmpHandler;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_support::pallet_prelude::*;
use frame_system::RawOrigin;

benchmarks! {
	add_chain_currency_data {
		let currency_id = T::GetNativeCurrencyId::get();
		let para_id: u32 = 1000;
		let xcm_data =
			XcmCurrencyData { native: false, fee_per_second: 100, instruction_weight: 1_000 };

	}: add_chain_currency_data(RawOrigin::Root, para_id, currency_id, xcm_data.clone())
	verify {
		assert_eq!(XcmChainCurrencyData::<T>::get(para_id, currency_id).unwrap(), xcm_data);
	}

	remove_chain_currency_data {
		let currency_id = T::GetNativeCurrencyId::get();
		let para_id: u32 = 1000;
		let xcm_data =
			XcmCurrencyData { native: false, fee_per_second: 100, instruction_weight: 1_000 };
		XcmChainCurrencyData::<T>::insert(para_id, currency_id, xcm_data);

	}: remove_chain_currency_data(RawOrigin::Root, para_id, currency_id)
	verify {
		if let Some(_) = XcmChainCurrencyData::<T>::get(para_id, currency_id) {
			panic!("There should be no data set")
		};
	}
}

impl_benchmark_test_suite!(XcmpHandler, crate::mock::new_test_ext(None), crate::mock::Test,);
