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
use crate::{mock::*, Error, XcmChainCurrencyData, XcmCurrencyData};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

//*****************
//Extrinsics
//*****************

// add_chain_currency_data
#[test]
fn can_add_new_data() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;

		if XcmChainCurrencyData::<Test>::get(para_id, currency_id).is_some() {
			panic!("There should be no data set")
		};

		let xcm_data =
			XcmCurrencyData { native: false, fee_per_second: 100, instruction_weight: 1_000 };

		assert_ok!(XcmpHandler::add_chain_currency_data(
			RawOrigin::Root.into(),
			para_id,
			currency_id,
			xcm_data.clone()
		));
		assert_eq!(XcmChainCurrencyData::<Test>::get(para_id, currency_id).unwrap(), xcm_data);
	});
}

#[test]
fn can_update_data() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;
		let xcm_data_old =
			XcmCurrencyData { native: false, fee_per_second: 100, instruction_weight: 1_000 };
		XcmChainCurrencyData::<Test>::insert(para_id, currency_id, xcm_data_old.clone());

		assert_eq!(XcmChainCurrencyData::<Test>::get(para_id, currency_id).unwrap(), xcm_data_old);

		let xcm_data_new =
			XcmCurrencyData { native: false, fee_per_second: 200, instruction_weight: 3_000 };

		assert_ok!(XcmpHandler::add_chain_currency_data(
			RawOrigin::Root.into(),
			para_id,
			currency_id,
			xcm_data_new.clone()
		));
		assert_eq!(XcmChainCurrencyData::<Test>::get(para_id, currency_id).unwrap(), xcm_data_new);
	});
}

#[test]
fn can_only_use_native_currency() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::ROC;
		let para_id: u32 = 1000;

		let xcm_data =
			XcmCurrencyData { native: false, fee_per_second: 100, instruction_weight: 1_000 };

		assert_noop!(
			XcmpHandler::add_chain_currency_data(
				RawOrigin::Root.into(),
				para_id,
				currency_id,
				xcm_data.clone()
			),
			Error::<Test>::CurrencyChainComboNotSupported
		);
	});
}

// remove_chain_currency_data
#[test]
fn can_remove_data() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;
		let xcm_data =
			XcmCurrencyData { native: false, fee_per_second: 100, instruction_weight: 1_000 };
		XcmChainCurrencyData::<Test>::insert(para_id, currency_id, xcm_data.clone());

		assert_eq!(XcmChainCurrencyData::<Test>::get(para_id, currency_id).unwrap(), xcm_data);
		assert_ok!(XcmpHandler::remove_chain_currency_data(
			RawOrigin::Root.into(),
			para_id,
			currency_id
		));
		if XcmChainCurrencyData::<Test>::get(para_id, currency_id).is_some() {
			panic!("There should be no data set")
		};
	});
}

#[test]
fn errors_if_not_found() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;

		if XcmChainCurrencyData::<Test>::get(para_id, currency_id).is_some() {
			panic!("There should be no data set")
		};
		assert_noop!(
			XcmpHandler::remove_chain_currency_data(RawOrigin::Root.into(), para_id, currency_id,),
			Error::<Test>::CurrencyChainComboNotFound
		);
	});
}
