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
use crate::{mock::*, InstructionSequence};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::{AccountIdConversion, Convert};
use xcm::latest::{prelude::*, Weight};

//*****************
//Extrinsics
//*****************

const PARA_ID: u32 = 1000;

//*****************
//Helper  functions
//*****************

// get_instruction_set
#[test]
fn get_instruction_set_local_currency_instructions() {
	let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
	let asset_location = MultiLocation::new(1, X1(Parachain(PARA_ID)));

	new_test_ext().execute_with(|| {
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight = Weight::from_parts(100_000_000, 0);
		let overall_weight = Weight::from_parts(200_000_000, 0);
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();

		let expected_instructions = XcmpHandler::get_local_currency_instructions(
			destination,
			asset_location,
			descend_location,
			transact_encoded_call.clone(),
			transact_encoded_call_weight,
			overall_weight,
			10,
		)
		.unwrap();

		assert_eq!(
			XcmpHandler::get_instruction_set(
				destination,
				asset_location,
				10,
				ALICE,
				transact_encoded_call,
				transact_encoded_call_weight,
				overall_weight,
				InstructionSequence::PayThroughSovereignAccount,
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
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let asset_location = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight = Weight::from_parts(100_000_000, 0);
		let xcm_weight = transact_encoded_call_weight
			.checked_add(&Weight::from_parts(100_000_000, 0))
			.expect("xcm_weight overflow");
		let xcm_fee = (xcm_weight.ref_time() as u128) * 5_000_000_000;
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();

		let (local, target) = XcmpHandler::get_local_currency_instructions(
			destination,
			asset_location,
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
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let asset_location = destination;
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight = Weight::from_parts(100_000_000, 0);
		let xcm_weight = transact_encoded_call_weight
			.checked_add(&Weight::from_parts(100_000_000, 0))
			.expect("xcm_weight overflow");
		let xcm_fee = (xcm_weight.ref_time() as u128) * 5_000_000_000;
		let asset = MultiAsset { id: Concrete(asset_location), fun: Fungible(xcm_fee) };
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();

		let (local_instructions, _) = XcmpHandler::get_local_currency_instructions(
			destination,
			asset_location,
			descend_location,
			transact_encoded_call,
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
					asset.clone(),
					MultiLocation { parents: 1, interior: X1(Parachain(LOCAL_PARA_ID)) }
				),
				// Depositing asset
				(asset, MultiLocation { parents: 1, interior: X1(Parachain(PARA_ID)) }),
			]
		);
		assert_eq!(events(), [RuntimeEvent::XcmpHandler(crate::Event::XcmTransactedLocally)]);
	});
}

#[test]
fn transact_in_target_chain_works() {
	new_test_ext().execute_with(|| {
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let asset_location = MultiLocation { parents: 1, interior: X1(Parachain(LOCAL_PARA_ID)) };
		let transact_encoded_call: Vec<u8> = vec![0, 1, 2];
		let transact_encoded_call_weight = Weight::from_parts(100_000_000, 0);
		let xcm_weight = transact_encoded_call_weight
			.checked_add(&Weight::from_parts(100_000_000, 0))
			.expect("xcm_weight overflow");
		let xcm_fee = (xcm_weight.ref_time() as u128) * 5_000_000_000;
		let asset = MultiAsset { id: Concrete(asset_location), fun: Fungible(xcm_fee) };
		let descend_location: Junctions =
			AccountIdToMultiLocation::convert(ALICE).try_into().unwrap();

		let (_, target_instructions) = XcmpHandler::get_local_currency_instructions(
			destination,
			asset_location,
			descend_location,
			transact_encoded_call.clone(),
			transact_encoded_call_weight,
			xcm_weight,
			xcm_fee,
		)
		.unwrap();

		assert_ok!(XcmpHandler::transact_in_target_chain(destination, target_instructions));
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
					DescendOrigin(X1(AccountId32 { network: None, id: ALICE.into() }),),
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: transact_encoded_call_weight,
						call: transact_encoded_call.into(),
					},
					RefundSurplus,
					DepositAsset {
						assets: Wild(AllCounted(1)),
						beneficiary: MultiLocation {
							parents: 1,
							interior: X1(Parachain(LOCAL_PARA_ID)),
						},
					},
				]
				.to_vec()),
			)]
		);
		assert_eq!(events(), [RuntimeEvent::XcmpHandler(crate::Event::XcmSent { destination })]);
	});
}

#[test]
fn pay_xcm_fee_works() {
	new_test_ext().execute_with(|| {
		let local_sovereign_account: AccountId =
			Sibling::from(LOCAL_PARA_ID).into_account_truncating();
		let fee = 3_500_000;
		let alice_balance = 8_000_000;

		Balances::force_set_balance(RawOrigin::Root.into(), ALICE, alice_balance).unwrap();

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

		Balances::force_set_balance(RawOrigin::Root.into(), ALICE, alice_balance).unwrap();

		assert_ok!(XcmpHandler::pay_xcm_fee(ALICE, fee));
		assert_eq!(Balances::free_balance(ALICE), alice_balance);
		assert_eq!(Balances::free_balance(local_sovereign_account), 0);
	});
}

fn events() -> Vec<RuntimeEvent> {
	let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}
