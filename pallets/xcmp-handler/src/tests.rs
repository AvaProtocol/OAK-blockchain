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
use frame_support::{assert_noop, assert_ok, weights::constants::WEIGHT_PER_SECOND};
use frame_system::RawOrigin;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::{AccountIdConversion, Convert};
use xcm::latest::prelude::*;

//*****************
//Extrinsics
//*****************

// add_chain_currency_data
#[test]
fn add_chain_currency_data_new_data() {
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
fn add_chain_currency_data_update_data() {
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
fn add_chain_currency_data_can_only_use_native_currency() {
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
fn remove_chain_currency_data_remove_data() {
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
fn remove_chain_currency_data_not_found() {
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

//*****************
//Helper  functions
//*****************

// calculate_xcm_fee_and_weight
#[test]
fn calculate_xcm_fee_and_weight_works() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;
		let xcm_data = XcmCurrencyData {
			native: false,
			fee_per_second: 50_000_000_000,
			instruction_weight: 1_000,
		};
		XcmChainCurrencyData::<Test>::insert(para_id, currency_id, xcm_data.clone());
		let transact_encoded_call_weight: u64 = 100_000_000;

		let expected_weight = transact_encoded_call_weight + xcm_data.instruction_weight;
		let expected_fee =
			xcm_data.fee_per_second * (expected_weight as u128) / (WEIGHT_PER_SECOND as u128);
		assert_ok!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				para_id,
				currency_id,
				transact_encoded_call_weight
			),
			(expected_fee, expected_weight),
		);
	});
}

#[test]
fn calculate_xcm_fee_and_weight_fee_overflow() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;
		let xcm_data =
			XcmCurrencyData { native: false, fee_per_second: u128::MAX, instruction_weight: 1_000 };
		XcmChainCurrencyData::<Test>::insert(para_id, currency_id, xcm_data.clone());
		let transact_encoded_call_weight: u64 = 100_000_000;

		assert_noop!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				para_id,
				currency_id,
				transact_encoded_call_weight
			),
			Error::<Test>::FeeOverflow
		);
	});
}

#[test]
fn calculate_xcm_fee_and_weight_weight_overflow() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;
		let xcm_data =
			XcmCurrencyData { native: false, fee_per_second: 1_000, instruction_weight: u64::MAX };
		XcmChainCurrencyData::<Test>::insert(para_id, currency_id, xcm_data.clone());
		let transact_encoded_call_weight: u64 = u64::MAX;

		assert_noop!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				para_id,
				currency_id,
				transact_encoded_call_weight
			),
			Error::<Test>::WeightOverflow
		);
	});
}

#[test]
fn calculate_xcm_fee_and_weight_no_xcm_data() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;
		let transact_encoded_call_weight: u64 = 100_000_000;

		if let Some(_) = XcmChainCurrencyData::<Test>::get(para_id, currency_id) {
			panic!("There should be no data set")
		};
		assert_noop!(
			XcmpHandler::calculate_xcm_fee_and_weight(
				para_id,
				currency_id,
				transact_encoded_call_weight
			),
			Error::<Test>::CurrencyChainComboNotFound
		);
	});
}

// get_instruction_set
#[test]
fn get_instruction_set_only_support_local_currency() {
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::ROC;
		let para_id: u32 = 1000;
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;

		assert_noop!(
			XcmpHandler::get_instruction_set(
				para_id,
				currency_id,
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
	new_test_ext().execute_with(|| {
		let currency_id = CurrencyId::Native;
		let para_id: u32 = 1000;
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let xcm_data = XcmCurrencyData {
			native: false,
			fee_per_second: 100_000,
			instruction_weight: 100_000_000,
		};
		XcmChainCurrencyData::<Test>::insert(para_id, currency_id, xcm_data.clone());
		let (xcm_fee, xcm_weight) = XcmpHandler::calculate_xcm_fee_and_weight(
			para_id,
			currency_id,
			transact_encoded_call_weight,
		)
		.unwrap();
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();
		let expected_instructions = XcmpHandler::get_local_currency_instructions(
			para_id,
			descend_location,
			transact_encoded_call.clone(),
			transact_encoded_call_weight,
			xcm_weight,
			xcm_fee,
		)
		.unwrap();

		assert_eq!(
			XcmpHandler::get_instruction_set(
				para_id,
				currency_id,
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
	new_test_ext().execute_with(|| {
		let para_id: u32 = 1000;
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight: u64 = 100_000_000;
		let xcm_weight = 100_000_000 + transact_encoded_call_weight;
		let xcm_fee = xcm_weight as u128 * 5_000_000_000u128;
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();

		let (local, target) = XcmpHandler::get_local_currency_instructions(
			para_id,
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
	new_test_ext().execute_with(|| {
		let para_id: u32 = 1000;
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
			para_id,
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
					MultiLocation { parents: 1, interior: X1(Parachain(para_id)) }
				),
			]
		);
		assert_eq!(events(), [Event::XcmpHandler(crate::Event::XcmTransactedLocally)]);
	});
}

#[test]
fn transact_in_target_chain_works() {
	new_test_ext().execute_with(|| {
		let para_id: u32 = 1000;
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
			para_id,
			descend_location,
			transact_encoded_call.clone(),
			transact_encoded_call_weight,
			xcm_weight,
			xcm_fee,
		)
		.unwrap();

		assert_ok!(XcmpHandler::transact_in_target_chain(para_id, target_instructions));
		assert_eq!(
			sent_xcm(),
			vec![(
				MultiLocation { parents: 1, interior: X1(Parachain(para_id)) },
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
		assert_eq!(events(), [Event::XcmpHandler(crate::Event::XcmSent { para_id: 1000 })]);
	});
}

#[test]
fn pay_xcm_fee_works() {
	new_test_ext().execute_with(|| {
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
	new_test_ext().execute_with(|| {
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
