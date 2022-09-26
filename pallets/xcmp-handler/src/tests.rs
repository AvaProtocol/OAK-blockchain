// This file is part of OAK Blockchain.

// Copyright (C) 2022 OAK Network
// SPDX-License-Identifier: Apache-2.0

use core::convert::TryInto;

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
use crate::{mock::*, Config, Error, XcmChainCurrencyData, XcmCurrencyData};
use frame_support::{assert_noop, assert_ok, weights::constants::WEIGHT_PER_SECOND};
use frame_system::RawOrigin;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::{AccountIdConversion, Convert};
use xcm::latest::prelude::*;

//*****************
//Extrinsics
//*****************

const PARA_ID: u32 = 1000;
const XCM_DATA_NATIVE: XcmCurrencyData = XcmCurrencyData {
	native: true,
	fee_per_second: 50_000_000_000,
	instruction_weight: 100_000_000,
};
const XCM_DATA_NON_NATIVE: XcmCurrencyData = XcmCurrencyData {
	native: false,
	fee_per_second: 50_000_000_000,
	instruction_weight: 100_000_000,
};

// add_chain_currency_data
#[test]
fn add_chain_currency_data_new_data() {
	new_test_ext(None).execute_with(|| {
		let currency_id = NATIVE;

		if XcmChainCurrencyData::<Test>::get(PARA_ID, currency_id).is_some() {
			panic!("There should be no data set")
		};

		assert_ok!(XcmpHandler::add_chain_currency_data(
			RawOrigin::Root.into(),
			PARA_ID,
			currency_id,
			XCM_DATA_NON_NATIVE
		));
		assert_eq!(
			XcmChainCurrencyData::<Test>::get(PARA_ID, currency_id).unwrap(),
			XCM_DATA_NON_NATIVE
		);
	});
}

#[test]
fn add_chain_currency_data_update_data() {
	let genesis_config = vec![(
		PARA_ID,
		NATIVE,
		XCM_DATA_NON_NATIVE.native,
		XCM_DATA_NON_NATIVE.fee_per_second,
		XCM_DATA_NON_NATIVE.instruction_weight,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		assert_eq!(
			XcmChainCurrencyData::<Test>::get(PARA_ID, NATIVE).unwrap(),
			XCM_DATA_NON_NATIVE
		);

		let xcm_data_new =
			XcmCurrencyData { native: false, fee_per_second: 200, instruction_weight: 3_000 };

		assert_ok!(XcmpHandler::add_chain_currency_data(
			RawOrigin::Root.into(),
			PARA_ID,
			NATIVE,
			xcm_data_new.clone()
		));
		assert_eq!(XcmChainCurrencyData::<Test>::get(PARA_ID, NATIVE).unwrap(), xcm_data_new);
	});
}

#[test]
fn add_chain_currency_data_can_only_use_native_currency() {
	new_test_ext(None).execute_with(|| {
		assert_noop!(
			XcmpHandler::add_chain_currency_data(
				RawOrigin::Root.into(),
				PARA_ID,
				RELAY,
				XCM_DATA_NON_NATIVE
			),
			Error::<Test>::CurrencyChainComboNotSupported
		);
	});
}

// remove_chain_currency_data
#[test]
fn remove_chain_currency_data_remove_data() {
	let genesis_config = vec![(
		PARA_ID,
		NATIVE,
		XCM_DATA_NON_NATIVE.native,
		XCM_DATA_NON_NATIVE.fee_per_second,
		XCM_DATA_NON_NATIVE.instruction_weight,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		assert_ok!(XcmpHandler::remove_chain_currency_data(
			RawOrigin::Root.into(),
			PARA_ID,
			NATIVE
		));
		if XcmChainCurrencyData::<Test>::get(PARA_ID, NATIVE).is_some() {
			panic!("There should be no data set")
		};
	});
}

#[test]
fn remove_chain_currency_data_not_found() {
	new_test_ext(None).execute_with(|| {
		if XcmChainCurrencyData::<Test>::get(PARA_ID, NATIVE).is_some() {
			panic!("There should be no data set")
		};
		assert_noop!(
			XcmpHandler::remove_chain_currency_data(RawOrigin::Root.into(), PARA_ID, NATIVE,),
			Error::<Test>::CurrencyChainComboNotFound
		);
	});
}

//*****************
//Helper  functions
//*****************

// calculate_xcm_fee_and_weight
#[test]
fn calculate_xcm_fee_and_weight_works() {
	let genesis_config = vec![(
		PARA_ID,
		NATIVE,
		XCM_DATA_NON_NATIVE.native,
		XCM_DATA_NON_NATIVE.fee_per_second,
		XCM_DATA_NON_NATIVE.instruction_weight,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call_weight: u64 = 100_000_000;

		let expected_weight = transact_encoded_call_weight + XCM_DATA_NON_NATIVE.instruction_weight;
		let expected_fee = XCM_DATA_NON_NATIVE.fee_per_second * (expected_weight as u128) /
			(WEIGHT_PER_SECOND as u128);
		assert_ok!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				PARA_ID,
				NATIVE,
				transact_encoded_call_weight
			),
			(expected_fee, expected_weight),
		);
	});
}

#[test]
fn calculate_xcm_fee_and_weight_fee_overflow() {
	let gensis_config = vec![(PARA_ID, NATIVE, false, u128::MAX, 1_000)];

	new_test_ext(Some(gensis_config)).execute_with(|| {
		let transact_encoded_call_weight: u64 = 100_000_000;

		assert_noop!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				PARA_ID,
				NATIVE,
				transact_encoded_call_weight
			),
			Error::<Test>::FeeOverflow
		);
	});
}

#[test]
fn calculate_xcm_fee_and_weight_weight_overflow() {
	let gensis_config = vec![(PARA_ID, NATIVE, false, 1_000, u64::MAX)];

	new_test_ext(Some(gensis_config)).execute_with(|| {
		let transact_encoded_call_weight: u64 = u64::MAX;

		assert_noop!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				PARA_ID,
				NATIVE,
				transact_encoded_call_weight
			),
			Error::<Test>::WeightOverflow
		);
	});
}

#[test]
fn calculate_xcm_fee_and_weight_no_xcm_data() {
	new_test_ext(None).execute_with(|| {
		let transact_encoded_call_weight: u64 = 100_000_000;

		if let Some(_) = XcmChainCurrencyData::<Test>::get(PARA_ID, NATIVE) {
			panic!("There should be no data set")
		};
		assert_noop!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				PARA_ID,
				NATIVE,
				transact_encoded_call_weight
			),
			Error::<Test>::CurrencyChainComboNotFound
		);
	});
}

// get_instruction_set
#[test]
fn get_instruction_set_only_support_local_and_foreign_native_currency() {
	let genesis_config = vec![(
		PARA_ID,
		RELAY,
		XCM_DATA_NON_NATIVE.native,
		XCM_DATA_NON_NATIVE.fee_per_second,
		XCM_DATA_NON_NATIVE.instruction_weight,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;

		assert_noop!(
			XcmpHandler::get_instruction_set(
				PARA_ID,
				RELAY,
				ALICE,
				transact_encoded_call,
				transact_encoded_call_weight
			),
			Error::<Test>::CurrencyChainComboNotSupported
		);
	});
}

#[test]
fn get_instruction_set_fails_if_no_xcmp_data() {
	new_test_ext(None).execute_with(|| {
		// add foreign non-native currency
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;

		assert_noop!(
			XcmpHandler::get_instruction_set(
				PARA_ID,
				RELAY,
				ALICE,
				transact_encoded_call,
				transact_encoded_call_weight
			),
			Error::<Test>::CurrencyChainComboNotFound
		);
	});
}

#[test]
fn get_instruction_set_local_currency_instructions() {
	let genesis_config = vec![(
		PARA_ID,
		NATIVE,
		XCM_DATA_NON_NATIVE.native,
		XCM_DATA_NON_NATIVE.fee_per_second,
		XCM_DATA_NON_NATIVE.instruction_weight,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let (xcm_fee, xcm_weight) = XcmpHandler::calculate_xcm_fee_and_weight(
			PARA_ID,
			NATIVE,
			transact_encoded_call_weight,
		)
		.unwrap();
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();
		let fee_location: MultiLocation = MultiLocation::new(0, Here).into();
		let asset = MultiAsset {
			id: Concrete(fee_location.clone().into()),
			fun: Fungibility::Fungible(xcm_fee),
		};
		let (mut expected_local_instructions, mut expected_target_instructions) =
			XcmpHandler::get_base_currency_instructions(
				PARA_ID,
				asset,
				descend_location,
				transact_encoded_call.clone(),
				transact_encoded_call_weight,
				xcm_weight,
			)
			.unwrap();

		let assets: MultiAssets = vec![MultiAsset {
			id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(LOCAL_PARA_ID)) }),
			fun: Fungible(xcm_fee.into()),
		}]
		.try_into()
		.unwrap();
		expected_local_instructions.0.insert(
			1,
			DepositAsset::<<Test as Config>::Call> {
				assets: Wild(All),
				max_assets: 1,
				beneficiary: MultiLocation { parents: 1, interior: X1(Parachain(PARA_ID)) },
			},
		);
		expected_target_instructions.0.insert(0, ReserveAssetDeposited(assets));

		assert_eq!(
			XcmpHandler::get_instruction_set(
				PARA_ID,
				NATIVE,
				ALICE,
				transact_encoded_call,
				transact_encoded_call_weight
			)
			.unwrap(),
			(expected_local_instructions, expected_target_instructions)
		);
	});
}

#[test]
fn get_instruction_set_foreign_currency_instructions() {
	let genesis_config = vec![(
		PARA_ID,
		RELAY,
		XCM_DATA_NATIVE.native,
		XCM_DATA_NATIVE.fee_per_second,
		XCM_DATA_NATIVE.instruction_weight,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let (xcm_fee, xcm_weight) =
			XcmpHandler::calculate_xcm_fee_and_weight(PARA_ID, RELAY, transact_encoded_call_weight)
				.unwrap();
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();
		let fee_location: MultiLocation = MultiLocation::parent().into();
		let asset = MultiAsset {
			id: Concrete(fee_location.clone().into()),
			fun: Fungibility::Fungible(xcm_fee),
		};
		let (expected_local_instructions, mut expected_target_instructions) =
			XcmpHandler::get_base_currency_instructions(
				PARA_ID,
				asset,
				descend_location,
				transact_encoded_call.clone(),
				transact_encoded_call_weight,
				xcm_weight,
			)
			.unwrap();

		let assets: MultiAssets =
			vec![MultiAsset { id: Concrete(fee_location), fun: Fungible(xcm_fee.into()) }]
				.try_into()
				.unwrap();
		expected_target_instructions.0.insert(0, WithdrawAsset(assets));

		assert_eq!(
			XcmpHandler::get_instruction_set(
				PARA_ID,
				RELAY,
				ALICE,
				transact_encoded_call,
				transact_encoded_call_weight
			)
			.unwrap(),
			(expected_local_instructions, expected_target_instructions)
		);
	});
}

#[test]
fn transact_in_local_chain_works() {
	let genesis_config = vec![(
		PARA_ID,
		NATIVE,
		XCM_DATA_NATIVE.native,
		XCM_DATA_NATIVE.fee_per_second,
		XCM_DATA_NATIVE.instruction_weight,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let (xcm_fee, _) = XcmpHandler::calculate_xcm_fee_and_weight(
			PARA_ID,
			NATIVE,
			transact_encoded_call_weight,
		)
		.unwrap();
		let asset = MultiAsset {
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
			fun: Fungible(xcm_fee),
		};

		let (expected_local_instructions, _) = XcmpHandler::get_instruction_set(
			PARA_ID,
			NATIVE,
			ALICE,
			transact_encoded_call,
			transact_encoded_call_weight,
		)
		.unwrap();

		assert_ok!(XcmpHandler::transact_in_local_chain(expected_local_instructions));
		assert_eq!(
			transact_asset(),
			vec![
				// Withdrawing asset
				(
					asset.clone().into(),
					MultiLocation { parents: 1, interior: X1(Parachain(LOCAL_PARA_ID)) }
				),
				// Depositing asset
				(
					asset.clone().into(),
					MultiLocation { parents: 1, interior: X1(Parachain(PARA_ID)) }
				),
			]
		);
		assert_eq!(events(), [Event::XcmpHandler(crate::Event::XcmTransactedLocally)]);
	});
}

#[test]
fn transact_in_target_chain_works() {
	let genesis_config = vec![(
		PARA_ID,
		NATIVE,
		XCM_DATA_NATIVE.native,
		XCM_DATA_NATIVE.fee_per_second,
		XCM_DATA_NATIVE.instruction_weight,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let (xcm_fee, xcm_weight) = XcmpHandler::calculate_xcm_fee_and_weight(
			PARA_ID,
			NATIVE,
			transact_encoded_call_weight,
		)
		.unwrap();
		let asset = MultiAsset {
			id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(LOCAL_PARA_ID)) }),
			fun: Fungible(xcm_fee),
		};

		let (_, target_instructions) = XcmpHandler::get_instruction_set(
			PARA_ID,
			NATIVE,
			ALICE,
			transact_encoded_call.clone(),
			transact_encoded_call_weight.clone(),
		)
		.unwrap();

		assert_ok!(XcmpHandler::transact_in_target_chain(PARA_ID, target_instructions));
		assert_eq!(
			sent_xcm(),
			vec![(
				MultiLocation { parents: 1, interior: X1(Parachain(PARA_ID)) },
				Xcm([
					ReserveAssetDeposited(asset.into()),
					BuyExecution {
						fees: MultiAsset {
							id: Concrete(MultiLocation {
								parents: 1,
								interior: X1(Parachain(LOCAL_PARA_ID))
							}),
							fun: Fungible(xcm_fee),
						},
						weight_limit: Limited(xcm_weight),
					},
					DescendOrigin(X1(AccountId32 { network: Any, id: ALICE.into() }),),
					Transact {
						origin_type: OriginKind::SovereignAccount,
						require_weight_at_most: transact_encoded_call_weight,
						call: transact_encoded_call.clone().into(),
					},
					RefundSurplus,
					DepositAsset {
						assets: Wild(All),
						max_assets: 1,
						beneficiary: MultiLocation {
							parents: 1,
							interior: X1(Parachain(LOCAL_PARA_ID)),
						},
					},
				]
				.to_vec()),
			)]
		);
		assert_eq!(events(), [Event::XcmpHandler(crate::Event::XcmSent { para_id: PARA_ID })]);
	});
}

#[test]
fn pay_xcm_fee_works() {
	new_test_ext(None).execute_with(|| {
		let local_sovereign_account: AccountId =
			Sibling::from(LOCAL_PARA_ID).into_account_truncating();
		let fee = 3_500_000;
		let alice_balance = 8_000_000;

		Balances::set_balance(RawOrigin::Root.into(), ALICE, alice_balance, 0).unwrap();

		assert_ok!(XcmpHandler::pay_xcm_fee(0, ALICE, fee));
		assert_eq!(Balances::free_balance(ALICE), alice_balance - fee);
		assert_eq!(Balances::free_balance(local_sovereign_account), fee);
	});
}

fn events() -> Vec<Event> {
	let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}
