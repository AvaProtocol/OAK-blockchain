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
use crate::{mock::*, Error, XcmChainCurrencyData, XcmCurrencyData, XcmFlow};
use frame_support::{assert_noop, assert_ok, weights::constants::WEIGHT_PER_SECOND};
use frame_system::RawOrigin;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::{AccountIdConversion, Convert};
use xcm::latest::prelude::*;

//*****************
//Extrinsics
//*****************

const PARA_ID: u32 = 1000;
const XCM_DATA: XcmCurrencyData = XcmCurrencyData {
	native: false,
	fee_per_second: 50_000_000_000,
	instruction_weight: 100_000_000,
	flow: XcmFlow::Normal,
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
			XCM_DATA
		));
		assert_eq!(XcmChainCurrencyData::<Test>::get(PARA_ID, currency_id).unwrap(), XCM_DATA);
	});
}

#[test]
fn add_chain_currency_data_update_data() {
	let genesis_config = vec![(
		PARA_ID,
		NATIVE,
		XCM_DATA.native,
		XCM_DATA.fee_per_second,
		XCM_DATA.instruction_weight,
		XCM_DATA.flow,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		assert_eq!(XcmChainCurrencyData::<Test>::get(PARA_ID, NATIVE).unwrap(), XCM_DATA);

		let xcm_data_new = XcmCurrencyData {
			native: false,
			fee_per_second: 200,
			instruction_weight: 3_000,
			flow: XcmFlow::Normal,
		};

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
			XcmpHandler::add_chain_currency_data(RawOrigin::Root.into(), PARA_ID, RELAY, XCM_DATA),
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
		XCM_DATA.native,
		XCM_DATA.fee_per_second,
		XCM_DATA.instruction_weight,
		XCM_DATA.flow,
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
		XCM_DATA.native,
		XCM_DATA.fee_per_second,
		XCM_DATA.instruction_weight,
		XCM_DATA.flow,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call_weight: u64 = 100_000_000;

		let expected_weight = transact_encoded_call_weight + XCM_DATA.instruction_weight * 6;
		let expected_fee = XCM_DATA.fee_per_second * (expected_weight as u128) /
			(WEIGHT_PER_SECOND.ref_time() as u128);
		assert_ok!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				PARA_ID,
				NATIVE,
				transact_encoded_call_weight
			),
			(expected_fee, expected_weight, XCM_DATA),
		);
	});
}

#[test]
fn calculate_xcm_fee_and_weight_fee_overflow() {
	let gensis_config = vec![(PARA_ID, NATIVE, false, u128::MAX, 1_000, XcmFlow::Normal)];

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
	let gensis_config = vec![(PARA_ID, NATIVE, false, 1_000, u64::MAX, XcmFlow::Normal)];

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

#[test]
fn calculate_xcm_fee_handles_alternate_flow() {
	let para_id: u32 = 9999;
	let genesis_config = vec![(
		para_id,
		NATIVE,
		XCM_DATA.native,
		XCM_DATA.fee_per_second,
		XCM_DATA.instruction_weight,
		XcmFlow::Alternate,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call_weight: u64 = 100_000_000;

		let expected_weight = transact_encoded_call_weight + XCM_DATA.instruction_weight * 6;
		assert_ok!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				para_id,
				NATIVE,
				transact_encoded_call_weight
			),
			(35000000, expected_weight, XcmCurrencyData { flow: XcmFlow::Alternate, ..XCM_DATA }),
		);
	});
}

// get_instruction_set
#[test]
fn get_instruction_set_only_support_local_currency() {
	new_test_ext(None).execute_with(|| {
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
fn get_instruction_set_local_currency_instructions() {
	let genesis_config = vec![(
		PARA_ID,
		NATIVE,
		XCM_DATA.native,
		XCM_DATA.fee_per_second,
		XCM_DATA.instruction_weight,
		XCM_DATA.flow,
	)];

	new_test_ext(Some(genesis_config)).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let (xcm_fee, xcm_weight, _) = XcmpHandler::calculate_xcm_fee_and_weight(
			PARA_ID,
			NATIVE,
			transact_encoded_call_weight,
		)
		.unwrap();
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();
		let expected_instructions = XcmpHandler::get_local_currency_instructions(
			PARA_ID,
			descend_location,
			transact_encoded_call.clone(),
			transact_encoded_call_weight,
			xcm_weight,
			xcm_fee,
		)
		.unwrap();

		assert_eq!(
			XcmpHandler::get_instruction_set(
				PARA_ID,
				NATIVE,
				ALICE,
				transact_encoded_call,
				transact_encoded_call_weight
			)
			.unwrap(),
			expected_instructions
		);
	});
}

// get_local_currency_instructions
// TODO: use xcm_simulator to test these instructions.
#[test]
fn get_local_currency_instructions_works() {
	new_test_ext(None).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let xcm_weight = 100_000_000 + transact_encoded_call_weight;
		let xcm_fee = xcm_weight as u128 * 5_000_000_000u128;
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();

		let (local, target) = XcmpHandler::get_local_currency_instructions(
			PARA_ID,
			descend_location,
			transact_encoded_call,
			transact_encoded_call_weight,
			xcm_weight,
			xcm_fee,
		)
		.unwrap();
		assert_eq!(local.0.len(), 2);
		assert_eq!(target.0.len(), 6);
	});
}

#[test]
fn transact_in_local_chain_works() {
	new_test_ext(None).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let xcm_weight = 100_000_000 + transact_encoded_call_weight;
		let xcm_fee = xcm_weight as u128 * 5_000_000_000u128;
		let asset = MultiAsset {
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
			fun: Fungible(xcm_fee),
		};
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();

		let (local_instructions, _) = XcmpHandler::get_local_currency_instructions(
			PARA_ID,
			descend_location,
			transact_encoded_call.clone(),
			transact_encoded_call_weight,
			xcm_weight,
			xcm_fee,
		)
		.unwrap();

		assert_ok!(XcmpHandler::transact_in_local_chain(local_instructions));
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
	new_test_ext(None).execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let xcm_weight = 100_000_000 + transact_encoded_call_weight;
		let xcm_fee = xcm_weight as u128 * 5_000_000_000u128;
		let asset = MultiAsset {
			id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(LOCAL_PARA_ID)) }),
			fun: Fungible(xcm_fee),
		};
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();

		let (_, target_instructions) = XcmpHandler::get_local_currency_instructions(
			PARA_ID,
			descend_location,
			transact_encoded_call.clone(),
			transact_encoded_call_weight,
			xcm_weight,
			xcm_fee,
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

		assert_ok!(XcmpHandler::pay_xcm_fee(ALICE, fee));
		assert_eq!(Balances::free_balance(ALICE), alice_balance - fee);
		assert_eq!(Balances::free_balance(local_sovereign_account), fee);
	});
}

#[test]
fn pay_xcm_fee_keeps_wallet_alive() {
	new_test_ext(None).execute_with(|| {
		let local_sovereign_account: AccountId =
			Sibling::from(LOCAL_PARA_ID).into_account_truncating();
		let fee = 3_500_000;
		let alice_balance = fee;

		Balances::set_balance(RawOrigin::Root.into(), ALICE, alice_balance, 0).unwrap();

		assert_ok!(XcmpHandler::pay_xcm_fee(ALICE, fee));
		assert_eq!(Balances::free_balance(ALICE), alice_balance);
		assert_eq!(Balances::free_balance(local_sovereign_account), 0);
	});
}

fn events() -> Vec<Event> {
	let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}
