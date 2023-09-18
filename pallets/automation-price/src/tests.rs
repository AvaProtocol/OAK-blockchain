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

use crate::{mock::*, AssetPayment, Config};

use pallet_xcmp_handler::InstructionSequence;

use frame_support::{
	assert_ok,
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use frame_system::{self, RawOrigin};
use sp_core::Get;
use sp_runtime::AccountId32;

use xcm::latest::{prelude::*, Junction::Parachain, MultiLocation};

use crate::weights::WeightInfo;

pub const START_BLOCK_TIME: u64 = 33198768000 * 1_000;
pub const SCHEDULED_TIME: u64 = START_BLOCK_TIME / 1_000 + 7200;
const LAST_BLOCK_TIME: u64 = START_BLOCK_TIME / 1_000;

// This is 1-0-3: {1: block idx}-{0: first extrinsic in block}-{3: the event index}
const FIRST_TASK_ID: [u8; 5] = [49, 45, 48, 45, 51];
const SECOND_TASK_ID: [u8; 5] = [49, 45, 48, 45, 54];

const EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT: &str = "Calculate schedule fee amount should work";

const DEFAULT_SCHEDULE_FEE_LOCATION: MultiLocation = MOONBASE_ASSET_LOCATION;

struct XcmpActionParams {
	destination: MultiLocation,
	schedule_fee: MultiLocation,
	execution_fee: AssetPayment,
	encoded_call: Vec<u8>,
	encoded_call_weight: Weight,
	overall_weight: Weight,
	schedule_as: Option<AccountId32>,
	instruction_sequence: InstructionSequence,
}

fn setup_asset(sender: &AccountId32) {
	AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		chain1.to_vec(),
		exchange1.to_vec(),
		asset1.to_vec(),
		asset2.to_vec(),
		10,
		vec![sender.clone()],
	);
}

impl Default for XcmpActionParams {
	fn default() -> Self {
		let delegator_account = AccountId32::new(DELEGATOR_ACCOUNT);
		XcmpActionParams {
			destination: MultiLocation::new(1, X1(Parachain(PARA_ID))),
			schedule_fee: DEFAULT_SCHEDULE_FEE_LOCATION,
			execution_fee: AssetPayment {
				asset_location: MOONBASE_ASSET_LOCATION.into(),
				amount: 100,
			},
			encoded_call: vec![3, 4, 5],
			encoded_call_weight: Weight::from_ref_time(100_000),
			overall_weight: Weight::from_ref_time(200_000),
			schedule_as: Some(delegator_account),
			instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
		}
	}
}

fn calculate_local_action_schedule_fee(weight: Weight, num_of_execution: u32) -> u128 {
	NATIVE_EXECUTION_WEIGHT_FEE * (weight.ref_time() as u128) * (num_of_execution as u128)
}

fn calculate_expected_xcmp_action_schedule_fee(
	schedule_fee_location: MultiLocation,
	num_of_execution: u32,
) -> u128 {
	let schedule_fee_location = schedule_fee_location
		.reanchored(
			&MultiLocation::new(1, X1(Parachain(<Test as Config>::SelfParaId::get().into()))),
			<Test as Config>::UniversalLocation::get(),
		)
		.expect("Location reanchor failed");
	let weight = <Test as Config>::WeightInfo::run_xcmp_task();

	if schedule_fee_location == MultiLocation::default() {
		calculate_local_action_schedule_fee(weight, num_of_execution)
	} else {
		let fee_per_second =
			get_fee_per_second(&schedule_fee_location).expect("Get fee per second should work");
		fee_per_second * (weight.ref_time() as u128) * (num_of_execution as u128) /
			(WEIGHT_REF_TIME_PER_SECOND as u128)
	}
}

// Helper function to asset event easiser
/// Assert the given `event` exists.
#[cfg(any(feature = "std", feature = "runtime-benchmarks", test))]
pub fn assert_has_event(event: RuntimeEvent) {
	let evts = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();
	assert!(evts.iter().any(|record| record == &event))
}

#[allow(dead_code)]
#[cfg(any(feature = "std", feature = "runtime-benchmarks", test))]
pub fn assert_last_event(event: RuntimeEvent) {
	assert_eq!(events().last().expect("events expected"), &event);
}

/// Check that events appear in the emitted_events list in order,
fn contains_events(emitted_events: Vec<RuntimeEvent>, events: Vec<RuntimeEvent>) -> bool {
	// If the target events list is empty, consider it satisfied as there are no specific order requirements
	if events.is_empty() {
		return true
	}

	// Convert both lists to iterators
	let mut emitted_iter = emitted_events.iter();
	let events_iter = events.iter();

	// Iterate through the target events
	for target_event in events_iter {
		// Initialize a boolean variable to track whether the target event is found
		let mut found = false;

		// Continue iterating through the emitted events until a match is found or there are no more emitted events
		for emitted_event in emitted_iter.by_ref() {
			// Compare event type and event data for a match
			if emitted_event == target_event {
				// Target event found, mark as found and advance the emitted iterator
				found = true;
				break
			}
		}

		// If the target event is not found, return false
		if !found {
			return false
		}
	}

	// If all target events are found in order, return true
	true
}

const exchange1: &[u8] = "exchange1".as_bytes();
const chain1: &[u8] = "KUSAMA".as_bytes();
const asset1: &[u8] = "TUR".as_bytes();
const asset2: &[u8] = "USDC".as_bytes();

#[test]
fn test_initialize_asset_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_ok!(AutomationPrice::initialize_asset(
			RawOrigin::Root.into(),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			10,
			vec!(AccountId32::new(ALICE))
		));
	})
}

#[test]
fn test_update_asset_prices() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let sender = AccountId32::new(ALICE);

		setup_asset(&sender);

		assert_ok!(AutomationPrice::update_asset_prices(
			RuntimeOrigin::signed(sender),
			vec!(chain1.to_vec()),
			vec!(exchange1.to_vec()),
			vec!(asset1.to_vec()),
			vec!(asset2.to_vec()),
			vec!(1005),
			vec!(START_BLOCK_TIME as u128),
			vec!(1),
		));

		let p = AutomationPrice::get_asset_price_data((
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
		))
		.expect("cannot get price");

		assert_eq!(p.round, 1);
		assert_eq!(p.amount, 1005);
	})
}

#[test]
fn test_schedule_xcmp_task_ok() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		// TODO: Setup fund once we add fund check and weight
		let para_id: u32 = 1000;
		let creator = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));

		setup_asset(&creator);

		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			1005u128,
			"gt".as_bytes().to_vec(),
			vec!(100),
			Box::new(destination.into()),
			Box::new(NATIVE_LOCATION.into()),
			Box::new(AssetPayment {
				asset_location: MultiLocation::new(0, Here).into(),
				amount: 10000000000000
			}),
			call.clone(),
			Weight::from_ref_time(100_000),
			Weight::from_ref_time(200_000)
		));

		// Upon schedule, task will be insert into 3 places
		// 1. TaskRegistry: a fast hashmap look up using task id only
		// 2. SortedTasksIndex: an ordering BTreeMap of the task, only task id and its price
		//          trigger
		// 3. AccountTasks: hashmap to look up user task id

		let task_ids = get_task_ids_from_events();
		let task_id = task_ids.first().expect("task failed to schedule");

		let task = AutomationPrice::get_task(task_id).expect("missing task in registry");
		assert_eq!(
			task.trigger_function,
			"gt".as_bytes().to_vec(),
			"created task has wrong trigger function"
		);
		assert_eq!(task.chain, chain1.to_vec(), "created task has different chain id");
		assert_eq!(task.asset_pair.0, asset1, "created task has wrong asset pair");

		// Ensure task is inserted into the right SortedIndex

		// Create  second task, and make sure both are recorded
		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			1005u128,
			"gt".as_bytes().to_vec(),
			vec!(100),
			Box::new(destination.into()),
			Box::new(NATIVE_LOCATION.into()),
			Box::new(AssetPayment {
				asset_location: MultiLocation::new(0, Here).into(),
				amount: 10000000000000
			}),
			call.clone(),
			Weight::from_ref_time(100_000),
			Weight::from_ref_time(200_000)
		));
		let task_ids2 = get_task_ids_from_events();
		let task_id2 = task_ids2.last().expect("task failed to schedule");
		assert_ne!(task_id, task_id2, "task id dup");

		let q = AutomationPrice::get_sorted_tasks_index((
			chain1.to_vec(),
			exchange1.to_vec(),
			(asset1.to_vec(), asset2.to_vec()),
			"gt".as_bytes().to_vec(),
		));
		assert_eq!(q.into_iter().count(), 2);
	})
}
