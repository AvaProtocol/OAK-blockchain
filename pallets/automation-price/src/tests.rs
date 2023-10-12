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

use crate::{
	mock::*, AccountStats, Action, AssetPayment, Config, Error, StatType, Task, TaskIdList,
	TaskStats, Tasks,
};
use pallet_xcmp_handler::InstructionSequence;

use frame_support::{
	assert_noop, assert_ok,
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use frame_system::{self, RawOrigin};
use sp_core::Get;
use sp_runtime::{AccountId32, ArithmeticError};

use xcm::latest::{prelude::*, Junction::Parachain, MultiLocation};

use crate::weights::WeightInfo;

use sp_std::collections::btree_map::BTreeMap;

pub const START_BLOCK_TIME: u64 = 33198768000 * 1_000;
pub const START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND: u128 = 33198768000 + 3600;

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

// Helper function to asset event easiser
/// Assert the given `event` not exists.
#[cfg(any(feature = "std", feature = "runtime-benchmarks", test))]
pub fn assert_no_event(event: RuntimeEvent) {
	let evts = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();
	assert!(evts.iter().all(|record| record != &event))
}

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

#[test]
fn test_initialize_asset_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let sender = AccountId32::new(ALICE);
		assert_ok!(AutomationPrice::initialize_asset(
			RawOrigin::Root.into(),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			10,
			vec!(sender.clone())
		));

		assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::AssetCreated {
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset1: asset1.to_vec(),
			asset2: asset2.to_vec(),
			decimal: 10,
		}));
	})
}

#[test]
fn test_initialize_asset_reject_duplicate_asset() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let sender = AccountId32::new(ALICE);
		AutomationPrice::initialize_asset(
			RawOrigin::Root.into(),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			10,
			vec![sender.clone()],
		);

		assert_noop!(
			AutomationPrice::initialize_asset(
				RawOrigin::Root.into(),
				chain1.to_vec(),
				exchange1.to_vec(),
				asset1.to_vec(),
				asset2.to_vec(),
				10,
				vec!(sender.clone())
			),
			Error::<Test>::AssetAlreadyInitialized,
		);
	})
}

#[test]
fn test_update_asset_prices() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let sender = AccountId32::new(ALICE);

		setup_asset(&sender, chain1.to_vec());

		assert_ok!(AutomationPrice::update_asset_prices(
			RuntimeOrigin::signed(sender.clone()),
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
			(asset1.to_vec(), asset2.to_vec()),
		))
		.expect("cannot get price");

		assert_eq!(p.round, 1);
		assert_eq!(p.amount, 1005);

		assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::AssetUpdated {
			who: sender,
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset1: asset1.to_vec(),
			asset2: asset2.to_vec(),
			price: 1005,
		}));
	})
}
#[test]
fn test_update_asset_prices_multi() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let sender = AccountId32::new(ALICE);

		setup_asset(&sender, chain1.to_vec());
		setup_asset(&sender, chain2.to_vec());

		assert_ok!(AutomationPrice::update_asset_prices(
			RuntimeOrigin::signed(sender.clone()),
			vec!(chain1.to_vec(), chain2.to_vec()),
			vec!(exchange1.to_vec(), exchange1.to_vec()),
			vec!(asset1.to_vec(), asset1.to_vec()),
			vec!(asset2.to_vec(), asset2.to_vec()),
			vec!(1005, 1009),
			vec!(START_BLOCK_TIME as u128, START_BLOCK_TIME as u128),
			vec!(1, 2),
		));

		let p1 = AutomationPrice::get_asset_price_data((
			chain1.to_vec(),
			exchange1.to_vec(),
			(asset1.to_vec(), asset2.to_vec()),
		))
		.expect("cannot get price");

		assert_eq!(p1.round, 1);
		assert_eq!(p1.amount, 1005);

		let p2 = AutomationPrice::get_asset_price_data((
			chain2.to_vec(),
			exchange1.to_vec(),
			(asset1.to_vec(), asset2.to_vec()),
		))
		.expect("cannot get price");

		assert_eq!(p2.round, 2);
		assert_eq!(p2.amount, 1009);

		assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::AssetUpdated {
			who: sender.clone(),
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset1: asset1.to_vec(),
			asset2: asset2.to_vec(),
			price: 1005,
		}));

		assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::AssetUpdated {
			who: sender,
			chain: chain2.to_vec(),
			exchange: exchange1.to_vec(),
			asset1: asset1.to_vec(),
			asset2: asset2.to_vec(),
			price: 1009,
		}));
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

		setup_asset(&creator, chain1.to_vec());

		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
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

		let task = AutomationPrice::get_task(&creator, &task_id).expect("missing task in registry");
		assert_eq!(
			task.trigger_function,
			"gt".as_bytes().to_vec(),
			"created task has wrong trigger function"
		);
		assert_eq!(task.chain, chain1.to_vec(), "created task has different chain id");
		assert_eq!(task.asset_pair.0, asset1, "created task has wrong asset pair");

		assert_eq!(START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND, task.expired_at);

		// Ensure task is inserted into the right SortedIndex

		// Create  second task, and make sure both are recorded
		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
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

		let sorted_task_index = AutomationPrice::get_sorted_tasks_index((
			chain1.to_vec(),
			exchange1.to_vec(),
			(asset1.to_vec(), asset2.to_vec()),
			"gt".as_bytes().to_vec(),
		))
		.unwrap();
		let task_ids: Vec<TaskIdList<Test>> = sorted_task_index.into_values().collect();
		assert_eq!(
			task_ids,
			vec!(vec!(
				(creator.clone(), vec!(49, 45, 48, 45, 49)),
				(creator.clone(), vec!(49, 45, 48, 45, 50))
			))
		);

		// We had schedule 2 tasks so far, all two belong to the same account
		assert_eq!(
			2,
			AutomationPrice::get_task_stat(StatType::TotalTasksOverall).map_or(0, |v| v),
			"total task count is incorrect"
		);
		assert_eq!(
			2,
			AutomationPrice::get_account_stat(creator, StatType::TotalTasksPerAccount)
				.map_or(0, |v| v),
			"total task count is incorrect"
		);
	})
}

// Verify that upon scheduling a task, the task expiration will be inserted into
// SortedTasksByExpiration and shard by expired_at.
#[test]
fn test_schedule_put_task_to_expiration_queue() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let para_id: u32 = 1000;
		let creator = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));

		setup_asset(&creator, chain1.to_vec());

		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
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
		let task_ids = get_task_ids_from_events();
		let task_id = task_ids.last().expect("task failed to schedule");

		let task_expiration_map = AutomationPrice::get_sorted_tasks_by_expiration();
		assert_eq!(
			task_expiration_map
				.get(&(START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND))
				.expect("missing task expiration shard"),
			&(BTreeMap::from([(task_id.clone(), creator)]))
		);
	})
}

// Verify that upon scheduling a task, the task expiration will be inserted into
// SortedTasksByExpiration and shard by expired_at.
// This test is similar as above test but we create multiple task wit different expiration to
// ensure all of them got to the right spot by expired_at time.
#[test]
fn test_schedule_put_task_to_expiration_queue_multi() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let para_id: u32 = 1000;
		let creator1 = AccountId32::new(ALICE);
		let creator2 = AccountId32::new(BOB);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));

		setup_asset(&creator1, chain1.to_vec());

		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator1.clone()),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
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
		let task_ids1 = get_task_ids_from_events();
		let task_id1 = task_ids1.last().expect("task failed to schedule");

		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator2.clone()),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND + 3600,
			"lt".as_bytes().to_vec(),
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

		let task_expiration_map = AutomationPrice::get_sorted_tasks_by_expiration();
		assert_eq!(
			task_expiration_map
				.get(&START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND)
				.expect("missing task expiration shard"),
			&BTreeMap::from([(task_id1.clone(), creator1)]),
		);
		assert_eq!(
			task_expiration_map
				.get(&(START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND + 3600))
				.expect("missing task expiration shard"),
			&BTreeMap::from([(task_id2.clone(), creator2)]),
		);
	})
}

// Verify that after calling sweep, expired task will be removed from all relevant storage. Our
// stat is also decrease accordingly to the task removal
#[test]
fn test_sweep_expired_task_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let creator = AccountId32::new(ALICE);
		let other_creator = AccountId32::new(BOB);
		let para_id: u32 = 1000;

		setup_assets_and_prices(&creator, START_BLOCK_TIME as u128);

		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let schedule_fee = MultiLocation::default();
		let execution_fee = AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		};
		let encoded_call_weight = Weight::from_ref_time(100_000);
		let overall_weight = Weight::from_ref_time(200_000);

		let expired_task_gen = 10;
		let price_target1 = 2000;
		for i in 0..expired_task_gen {
			// schedule task that has expired
			let task = Task::<Test> {
				owner_id: creator.clone().into(),
				task_id: format!("123-0-{:?}", i).as_bytes().to_vec(),
				chain: chain1.to_vec(),
				exchange: exchange1.to_vec(),
				asset_pair: (asset1.to_vec(), asset2.to_vec()),
				expired_at: START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND - 1800,
				trigger_function: "gt".as_bytes().to_vec(),
				trigger_params: vec![price_target1],
				action: Action::XCMP {
					destination,
					schedule_fee,
					execution_fee: execution_fee.clone(),
					encoded_call: vec![1, 2, 3],
					encoded_call_weight,
					overall_weight,
					schedule_as: None,
					instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
				},
			};
			assert_ok!(AutomationPrice::validate_and_schedule_task(task.clone()));
		}

		// Now we set timestamp to a later point
		Timestamp::set_timestamp(START_BLOCK_TIME.saturating_add(3600_000_u64).try_into().unwrap());

		let price_target2 = 1000;
		let task = Task::<Test> {
			owner_id: other_creator.clone().into(),
			task_id: "123-1-1".as_bytes().to_vec(),
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset_pair: (asset1.to_vec(), asset2.to_vec()),
			expired_at: START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND + 3600,
			trigger_function: "lt".as_bytes().to_vec(),
			trigger_params: vec![price_target2],
			action: Action::XCMP {
				destination,
				schedule_fee,
				execution_fee,
				encoded_call: vec![1, 2, 3],
				encoded_call_weight,
				overall_weight,
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			},
		};
		assert_ok!(AutomationPrice::validate_and_schedule_task(task.clone()));

		assert_eq!(u128::try_from(Tasks::<Test>::iter().count()).unwrap(), expired_task_gen + 1);

		assert_eq!(
			// 10 task by creator, 1 task by other_creator
			11,
			AutomationPrice::get_task_stat(StatType::TotalTasksOverall).map_or(0, |v| v),
			"total task count is incorrect"
		);
		assert_eq!(
			10,
			AutomationPrice::get_account_stat(creator.clone(), StatType::TotalTasksPerAccount)
				.map_or(0, |v| v),
			"total task count is incorrect"
		);
		assert_eq!(
			1,
			AutomationPrice::get_account_stat(
				other_creator.clone(),
				StatType::TotalTasksPerAccount
			)
			.map_or(0, |v| v),
			"total task count is incorrect"
		);

		assert_eq!(
			10,
			AutomationPrice::get_sorted_tasks_index((
				chain1.to_vec(),
				exchange1.to_vec(),
				(asset1.to_vec(), asset2.to_vec()),
				"gt".as_bytes().to_vec(),
			))
			.map_or(0, |v| v.get(&price_target1).unwrap().iter().len())
		);

		assert_eq!(
			1,
			AutomationPrice::get_sorted_tasks_index((
				chain1.to_vec(),
				exchange1.to_vec(),
				(asset1.to_vec(), asset2.to_vec()),
				"lt".as_bytes().to_vec(),
			))
			.map_or(0, |v| v.get(&price_target2).unwrap().iter().len())
		);

		assert_eq!(
			10,
			AutomationPrice::get_sorted_tasks_by_expiration()
				.get(&(START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND - 1800))
				.expect("missing task expiration shard")
				.len(),
		);

		// now we will sweep, passing a weight limit. In actualy code, this will be the
		// remaining_weight in on_idle block
		let remain_weight = 100_000_000_000;
		AutomationPrice::sweep_expired_task(Weight::from_ref_time(remain_weight));

		assert_eq!(
			AutomationPrice::get_sorted_tasks_by_expiration()
				.get(&(START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND - 1800)),
			None
		);

		for i in 0..expired_task_gen {
			assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::TaskSweep {
				who: creator.clone(),
				task_id: format!("123-0-{:?}", i).as_bytes().to_vec(),
				condition: crate::TaskCondition::AlreadyExpired {
					expired_at: START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND - 1800,
					now: START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
				},
			}));
		}

		// After sweep there should only one task remain in queue
		assert_eq!(Tasks::<Test>::iter().count(), 1);

		// The task should be removed from the SortedTasksIndex
		assert_eq!(
			0,
			AutomationPrice::get_sorted_tasks_index((
				chain1.to_vec(),
				exchange1.to_vec(),
				(asset1.to_vec(), asset2.to_vec()),
				"gt".as_bytes().to_vec(),
			))
			.expect("missing tasks sorted by price data")
			.get(&price_target1)
			.map_or(0, |v| v.iter().len())
		);
		// The task should be removed from the SortedTasksIndex
		assert_eq!(
			1,
			AutomationPrice::get_sorted_tasks_index((
				chain1.to_vec(),
				exchange1.to_vec(),
				(asset1.to_vec(), asset2.to_vec()),
				"lt".as_bytes().to_vec(),
			))
			.expect("missing tasks sorted by price data")
			.get(&price_target2)
			.map_or(0, |v| v.iter().len())
		);

		// The task stat should be changed
		assert_eq!(
			1,
			AutomationPrice::get_task_stat(StatType::TotalTasksOverall).map_or(0, |v| v),
			"total task count is incorrect"
		);
		assert_eq!(
			1,
			AutomationPrice::get_account_stat(other_creator, StatType::TotalTasksPerAccount)
				.map_or(0, |v| v),
			"total task count is incorrect"
		);
	})
}

// Test swap partially data, and leave the rest of sorted index remain intact
#[test]
fn test_sweep_expired_task_partially() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let creator = AccountId32::new(ALICE);
		let other_creator = AccountId32::new(BOB);
		let para_id: u32 = 1000;

		setup_assets_and_prices(&creator, START_BLOCK_TIME as u128);

		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let schedule_fee = MultiLocation::default();
		let execution_fee = AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		};
		let encoded_call_weight = Weight::from_ref_time(100_000);
		let overall_weight = Weight::from_ref_time(200_000);

		let expired_task_gen = 11;
		let price_target1 = 2000;
		for i in 1..expired_task_gen {
			// schedule task that has expired
			let task = Task::<Test> {
				owner_id: creator.clone().into(),
				task_id: format!("123-0-{:?}", i).as_bytes().to_vec(),
				chain: chain1.to_vec(),
				exchange: exchange1.to_vec(),
				asset_pair: (asset1.to_vec(), asset2.to_vec()),
				expired_at: START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND + 10 + i * 10,
				trigger_function: "gt".as_bytes().to_vec(),
				trigger_params: vec![price_target1],
				action: Action::XCMP {
					destination,
					schedule_fee,
					execution_fee: execution_fee.clone(),
					encoded_call: vec![1, 2, 3],
					encoded_call_weight,
					overall_weight,
					schedule_as: None,
					instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
				},
			};
			assert_ok!(AutomationPrice::validate_and_schedule_task(task.clone()));
		}

		// Now we set timestamp to a later point
		Timestamp::set_timestamp(
			START_BLOCK_TIME.saturating_add((3600 + 6 * 10) * 1000).try_into().unwrap(),
		);

		assert_eq!(
			10,
			AutomationPrice::get_sorted_tasks_index((
				chain1.to_vec(),
				exchange1.to_vec(),
				(asset1.to_vec(), asset2.to_vec()),
				"gt".as_bytes().to_vec(),
			))
			.map_or(0, |v| v.get(&price_target1).unwrap().iter().len())
		);

		assert_eq!(10, AutomationPrice::get_sorted_tasks_by_expiration().len());

		// remaining_weight in on_idle block
		let remain_weight = 100_000_000_000;
		AutomationPrice::sweep_expired_task(Weight::from_ref_time(remain_weight));

		// The task should be removed from the SortedTasksIndex
		assert_eq!(
			5,
			AutomationPrice::get_sorted_tasks_index((
				chain1.to_vec(),
				exchange1.to_vec(),
				(asset1.to_vec(), asset2.to_vec()),
				"gt".as_bytes().to_vec(),
			))
			.expect("missing tasks sorted by price data")
			.get(&price_target1)
			.map_or(0, |v| v.iter().len())
		);

		// these task all get sweeo
		for i in 1..5 {
			assert_eq!(
				0,
				AutomationPrice::get_sorted_tasks_by_expiration()
					.get(&(START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND + 10 + i * 10))
					.map_or(0, |v| v.len()),
			);
		}

		// these slot remian untouch
		for i in 6..10 {
			assert_eq!(
				1,
				AutomationPrice::get_sorted_tasks_by_expiration()
					.get(&(START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND + 10 + i * 10))
					.map_or(0, |v| v.len()),
			);
		}
	})
}

#[test]
fn test_schedule_return_error_when_reaching_max_tasks_overall_limit() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let para_id: u32 = 1000;
		let creator = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));

		setup_asset(&creator, chain1.to_vec());

		TaskStats::<Test>::insert(StatType::TotalTasksOverall, 1_000_000_000);

		assert_noop!(
			AutomationPrice::schedule_xcmp_task(
				RuntimeOrigin::signed(creator.clone()),
				chain1.to_vec(),
				exchange1.to_vec(),
				asset1.to_vec(),
				asset2.to_vec(),
				START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND + 3600,
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
			),
			Error::<Test>::MaxTasksReached,
		);
	})
}

#[test]
fn test_schedule_return_error_when_reaching_max_account_tasks_limit() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let para_id: u32 = 1000;
		let creator = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));

		setup_asset(&creator, chain1.to_vec());

		AccountStats::<Test>::insert(creator.clone(), StatType::TotalTasksPerAccount, 1_000);

		assert_noop!(
			AutomationPrice::schedule_xcmp_task(
				RuntimeOrigin::signed(creator),
				chain1.to_vec(),
				exchange1.to_vec(),
				asset1.to_vec(),
				asset2.to_vec(),
				START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
				"gt".as_bytes().to_vec(),
				vec!(100),
				Box::new(destination.into()),
				Box::new(NATIVE_LOCATION.into()),
				Box::new(AssetPayment {
					asset_location: MultiLocation::new(0, Here).into(),
					amount: 10000000000000
				}),
				call,
				Weight::from_ref_time(100_000),
				Weight::from_ref_time(200_000)
			),
			Error::<Test>::MaxTasksPerAccountReached,
		);
	})
}

// Test when price moves, the TaskQueue will be populated with the right task id
//
// In this test we will first setup 3 tasks for 3 pairs
//  task1 for pair1
//  task2 for pair2
//  task3 for pair3
//
//  when we will adjust the price of the pair, we purposely let task2 price not match
//  so we will test that task1, task3 will be moved into the queue accordingly.
//
//  then finally schedule a new task4, for pair3, but its price will match. we will test
//  that task4 are moved to TaskQueue. task2 won't be moved
//
//  The purpose of this test is to simulae a few tasks being trigger accordingly to their
//  price movement. We also verify that task  won't have its price match, will never get
//  trigger
#[test]
fn test_shift_tasks_movement_through_price_changes() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		// TODO: Setup fund once we add fund check and weight
		let para_id: u32 = 1000;
		let creator = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));

		setup_assets_and_prices(&creator, START_BLOCK_TIME as u128);

		let mut base_price = 10_000_u128;

		// Lets setup 3 tasks
		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
			"gt".as_bytes().to_vec(),
			vec!(base_price + 1000),
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

		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain2.to_vec(),
			exchange1.to_vec(),
			asset2.to_vec(),
			asset3.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
			"gt".as_bytes().to_vec(),
			vec!(base_price + 900),
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

		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain2.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset3.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND + 6000,
			"gt".as_bytes().to_vec(),
			vec!(base_price + 1000),
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

		let task_ids = get_task_ids_from_events();
		let task_id1 = task_ids.get(task_ids.len().wrapping_sub(3)).unwrap();
		// let _task_id2 = task_ids.get(task_ids.len().wrapping_sub(2)).unwrap();
		let task_id3 = task_ids.get(task_ids.len().wrapping_sub(1)).unwrap();

		// at this moment our task queue is empty
		// There is schedule tasks, but no tasks in the queue at this moment, because shift_tasks
		// has not run yet
		assert_eq!(AutomationPrice::get_task_queue().is_empty(), true);

		// shift_tasks move task from registry to the queue
		// At this moment, The price doesn't match the target so there is no change in our tasks
		AutomationPrice::shift_tasks(Weight::from_ref_time(1_000_000_000));
		assert_eq!(AutomationPrice::get_task_queue().is_empty(), true);
		let sorted_task_index = AutomationPrice::get_sorted_tasks_index((
			chain1.to_vec(),
			exchange1.to_vec(),
			(asset1.to_vec(), asset2.to_vec()),
			"gt".as_bytes().to_vec(),
		));
		assert_eq!(sorted_task_index.map_or_else(|| 0, |x| x.len()), 1);

		// now we change price of pair1 to higher than its target price, while keeping pair2/pair3 low enough,
		// only task_id1 will be moved to the queue.
		// The target price for those respectively tasks are 10100, 10900, 102000 in their pair
		// Therefore after running this price update, first task task_id1 are moved into TaskQueue
		let mut new_pair_1_price: u128 = base_price + 2000;
		let mut new_pair_2_price: u128 = 10_u128;
		let mut new_pair_3_price: u128 = 300_u128;
		assert_ok!(AutomationPrice::update_asset_prices(
			RuntimeOrigin::signed(creator.clone()),
			vec!(chain1.to_vec(), chain2.to_vec(), chain2.to_vec()),
			vec!(exchange1.to_vec(), exchange1.to_vec(), exchange1.to_vec()),
			vec!(asset1.to_vec(), asset2.to_vec(), asset1.to_vec()),
			vec!(asset2.to_vec(), asset3.to_vec(), asset3.to_vec()),
			vec!(new_pair_1_price, new_pair_2_price, new_pair_3_price),
			vec!(START_BLOCK_TIME as u128, START_BLOCK_TIME as u128, START_BLOCK_TIME as u128),
			vec!(1, 2, 3),
		));
		AutomationPrice::shift_tasks(Weight::from_ref_time(1_000_000_000));
		assert_eq!(AutomationPrice::get_task_queue(), vec![(creator.clone(), task_id1.clone())]);
		// The task are removed from SortedTasksIndex into the TaskQueue, therefore their length
		// decrease to 0
		assert_eq!(
			AutomationPrice::get_sorted_tasks_index((
				chain1.to_vec(),
				exchange1.to_vec(),
				(asset1.to_vec(), asset2.to_vec()),
				"gt".as_bytes().to_vec(),
			))
			.map_or_else(|| 0, |x| x.len()),
			0
		);

		// now we move target price of pair3 to higher than its target, and will observe that its
		// task will be moved to TaskQueue too.
		new_pair_3_price = base_price + 2000;
		AutomationPrice::update_asset_prices(
			RuntimeOrigin::signed(creator.clone()),
			vec![chain2.to_vec()],
			vec![exchange1.to_vec()],
			vec![asset1.to_vec()],
			vec![asset3.to_vec()],
			vec![new_pair_3_price],
			vec![START_BLOCK_TIME as u128],
			vec![4],
		);
		AutomationPrice::shift_tasks(Weight::from_ref_time(1_000_000_000));
		assert_eq!(
			AutomationPrice::get_task_queue(),
			vec![(creator.clone(), task_id1.clone()), (creator.clone(), task_id3.clone())]
		);
		// The task are removed from SortedTasksIndex into the TaskQueue, therefore their length
		// decrease to 0
		assert_eq!(
			AutomationPrice::get_sorted_tasks_index((
				chain2.to_vec(),
				exchange1.to_vec(),
				(asset1.to_vec(), asset3.to_vec()),
				"gt".as_bytes().to_vec(),
			))
			.map_or_else(|| 0, |x| x.len()),
			0
		);

		// Now, if a new task come up, and its price target matches the existing price, they will
		// be trigger too.
		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain2.to_vec(),
			exchange1.to_vec(),
			asset2.to_vec(),
			asset3.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
			"lt".as_bytes().to_vec(),
			// price for this asset is 10 in our last update
			vec!(20),
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
		// The task is now on the SortedTasksIndex
		assert_eq!(
			AutomationPrice::get_sorted_tasks_index((
				chain2.to_vec(),
				exchange1.to_vec(),
				(asset2.to_vec(), asset3.to_vec()),
				"lt".as_bytes().to_vec(),
			))
			.map_or_else(|| 0, |x| x.len()),
			1
		);

		AutomationPrice::shift_tasks(Weight::from_ref_time(1_000_000_000));
		let task_id4 = {
			let task_ids = get_task_ids_from_events();
			task_ids.last().unwrap().clone()
		};

		// Now the task is again, moved into the queue and be removed from SortedTasksIndex
		assert_eq!(
			AutomationPrice::get_task_queue(),
			vec![
				(creator.clone(), task_id1.clone()),
				(creator.clone(), task_id3.clone()),
				(creator.clone(), task_id4.clone())
			]
		);
		assert_eq!(
			AutomationPrice::get_sorted_tasks_index((
				chain2.to_vec(),
				exchange1.to_vec(),
				(asset2.to_vec(), asset3.to_vec()),
				"lt".as_bytes().to_vec(),
			))
			.map_or_else(|| 0, |x| x.len()),
			0
		);
	})
}

// the logic around > or < using include/exclude range to include bound or not, it can be subtle
// and error prone to human mistake so this test exist to make sure we catch that edge case.
#[test]
fn test_gt_task_not_run_when_asset_price_equal_target_price() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		// TODO: Setup fund once we add fund check and weight
		let para_id: u32 = 1000;
		let creator = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));

		setup_assets_and_prices(&creator, START_BLOCK_TIME as u128);

		let mut base_price = 1_000_u128;

		assert_ok!(AutomationPrice::schedule_xcmp_task(
			RuntimeOrigin::signed(creator.clone()),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
			START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
			"gt".as_bytes().to_vec(),
			vec!(base_price),
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

		AutomationPrice::shift_tasks(Weight::from_ref_time(1_000_000_000));
		// Task shouldn't be move to task queue to trigger, and the task queue should be empty
		assert_eq!(AutomationPrice::get_task_queue().is_empty(), true);

		let sorted_task_index = AutomationPrice::get_sorted_tasks_index((
			chain1.to_vec(),
			exchange1.to_vec(),
			(asset1.to_vec(), asset2.to_vec()),
			"gt".as_bytes().to_vec(),
		));
		assert_eq!(1, sorted_task_index.map_or_else(|| 0, |x| x.len()));
	})
}

#[test]
fn test_emit_event_when_execute_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let creator = AccountId32::new(ALICE);
		let para_id: u32 = 1000;

		setup_assets_and_prices(&creator, START_BLOCK_TIME as u128);

		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let schedule_fee = MultiLocation::default();
		let execution_fee = AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		};
		let encoded_call_weight = Weight::from_ref_time(100_000);
		let overall_weight = Weight::from_ref_time(200_000);

		let task = Task::<Test> {
			owner_id: creator.clone().into(),
			task_id: "123-0-1".as_bytes().to_vec(),
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset_pair: (asset1.to_vec(), asset2.to_vec()),
			expired_at: (START_BLOCK_TIME + 10000) as u128,
			trigger_function: "gt".as_bytes().to_vec(),
			trigger_params: vec![123],
			action: Action::XCMP {
				destination,
				schedule_fee,
				execution_fee,
				encoded_call: vec![1, 2, 3],
				encoded_call_weight,
				overall_weight,
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			},
		};

		AutomationPrice::validate_and_schedule_task(task.clone());

		AutomationPrice::run_tasks(
			vec![(task.owner_id.clone(), task.task_id.clone())],
			100_000_000_000.into(),
		);

		assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::TaskTriggered {
			who: task.owner_id.clone(),
			task_id: task.task_id.clone(),
		}));

		assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::TaskExecuted {
			who: task.owner_id.clone(),
			task_id: task.task_id,
		}));
	})
}

#[test]
fn test_decrease_task_count_when_execute_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let creator1 = AccountId32::new(ALICE);
		let creator2 = AccountId32::new(BOB);
		let para_id: u32 = 1000;

		setup_assets_and_prices(&creator1, START_BLOCK_TIME as u128);

		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let schedule_fee = MultiLocation::default();
		let execution_fee = AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		};
		let encoded_call_weight = Weight::from_ref_time(100_000);
		let overall_weight = Weight::from_ref_time(200_000);

		let task1 = Task::<Test> {
			owner_id: creator1.clone().into(),
			task_id: "123-0-1".as_bytes().to_vec(),
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset_pair: (asset1.to_vec(), asset2.to_vec()),
			expired_at: (START_BLOCK_TIME + 10000) as u128,
			trigger_function: "gt".as_bytes().to_vec(),
			trigger_params: vec![123],
			action: Action::XCMP {
				destination,
				schedule_fee,
				execution_fee: execution_fee.clone(),
				encoded_call: vec![1, 2, 3],
				encoded_call_weight,
				overall_weight,
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			},
		};

		let task2 = Task::<Test> {
			owner_id: creator2.clone().into(),
			task_id: "123-1-1".as_bytes().to_vec(),
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset_pair: (asset1.to_vec(), asset2.to_vec()),
			expired_at: (START_BLOCK_TIME + 10000) as u128,
			trigger_function: "gt".as_bytes().to_vec(),
			trigger_params: vec![123],
			action: Action::XCMP {
				destination,
				schedule_fee,
				execution_fee,
				encoded_call: vec![1, 2, 3],
				encoded_call_weight,
				overall_weight,
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			},
		};

		AutomationPrice::validate_and_schedule_task(task1.clone());
		AutomationPrice::validate_and_schedule_task(task2.clone());

		assert_eq!(
			2,
			AutomationPrice::get_task_stat(StatType::TotalTasksOverall).map_or(0, |v| v),
			"total task count is wrong"
		);
		assert_eq!(
			1,
			AutomationPrice::get_account_stat(creator1.clone(), StatType::TotalTasksPerAccount)
				.map_or(0, |v| v),
			"total task count is wrong"
		);
		assert_eq!(
			1,
			AutomationPrice::get_account_stat(creator2.clone(), StatType::TotalTasksPerAccount)
				.map_or(0, |v| v),
			"total task count is wrong"
		);

		AutomationPrice::run_tasks(
			vec![(task1.owner_id.clone(), task1.task_id.clone())],
			100_000_000_000.into(),
		);

		assert_eq!(
			1,
			AutomationPrice::get_task_stat(StatType::TotalTasksOverall).map_or(0, |v| v),
			"total task count is wrong"
		);
		assert_eq!(
			0,
			AutomationPrice::get_account_stat(creator1, StatType::TotalTasksPerAccount)
				.map_or(0, |v| v),
			"total task count of creator1 is wrong"
		);
	})
}

// when running a task, if the task is already expired, the execution engine won't run the task,
// instead an even TaskExpired is emiited
#[test]
fn test_expired_task_not_run() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let creator = AccountId32::new(ALICE);
		let para_id: u32 = 1000;

		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let schedule_fee = MultiLocation::default();
		let execution_fee = AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		};
		let encoded_call_weight = Weight::from_ref_time(100_000);
		let overall_weight = Weight::from_ref_time(200_000);

		let task = Task::<Test> {
			owner_id: creator.into(),
			task_id: "123-0-1".as_bytes().to_vec(),
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset_pair: (asset1.to_vec(), asset2.to_vec()),
			expired_at: START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
			trigger_function: "gt".as_bytes().to_vec(),
			trigger_params: vec![123],
			action: Action::XCMP {
				destination,
				schedule_fee,
				execution_fee,
				encoded_call: vec![1, 2, 3],
				encoded_call_weight,
				overall_weight,
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			},
		};

		AutomationPrice::validate_and_schedule_task(task.clone());

		// Moving the clock to simulate the task expiration
		Timestamp::set_timestamp(START_BLOCK_TIME.saturating_add(7200_000_u64).try_into().unwrap());
		AutomationPrice::run_tasks(
			vec![(task.owner_id.clone(), task.task_id.clone())],
			100_000_000_000.into(),
		);

		assert_no_event(RuntimeEvent::AutomationPrice(crate::Event::TaskTriggered {
			who: task.owner_id.clone(),
			task_id: task.task_id.clone(),
		}));

		assert_no_event(RuntimeEvent::AutomationPrice(crate::Event::TaskExecuted {
			who: task.owner_id.clone(),
			task_id: task.task_id.clone(),
		}));

		assert_last_event(RuntimeEvent::AutomationPrice(crate::Event::TaskExpired {
			who: task.owner_id.clone(),
			task_id: task.task_id.clone(),
			condition: crate::TaskCondition::AlreadyExpired {
				expired_at: task.expired_at,
				now: START_BLOCK_TIME
					.saturating_add(7200_000_u64)
					.checked_div(1000)
					.ok_or(ArithmeticError::Overflow)
					.expect("blocktime is out of range") as u128,
			},
		}));
	})
}

// when running a task, if the price has been moved against the target price, rendering the target
// price condition not match anymore. we will skip run
#[test]
fn test_price_move_against_target_price_skip_run() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let creator = AccountId32::new(ALICE);
		let para_id: u32 = 1000;

		setup_assets_and_prices(&creator, START_BLOCK_TIME as u128);

		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let schedule_fee = MultiLocation::default();
		let execution_fee = AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		};
		let encoded_call_weight = Weight::from_ref_time(100_000);
		let overall_weight = Weight::from_ref_time(200_000);

		let task = Task::<Test> {
			owner_id: creator.into(),
			task_id: "123-0-1".as_bytes().to_vec(),
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset_pair: (asset1.to_vec(), asset2.to_vec()),
			expired_at: START_BLOCK_TIME
				.checked_div(1000)
				.map_or(10000000_u128, |v| v.into())
				.saturating_add(100),
			trigger_function: "gt".as_bytes().to_vec(),
			// This asset price is set to 1000
			// The task is config to run when price > 2000, and we invoked it directly
			// so and we will observe that task won't run due to price doesn't match
			// the condition
			trigger_params: vec![2000],
			action: Action::XCMP {
				destination,
				schedule_fee,
				execution_fee,
				encoded_call: vec![1, 2, 3],
				encoded_call_weight,
				overall_weight,
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			},
		};

		AutomationPrice::validate_and_schedule_task(task.clone());

		AutomationPrice::run_tasks(
			vec![(task.owner_id.clone(), task.task_id.clone())],
			100_000_000_000.into(),
		);

		assert_no_event(RuntimeEvent::AutomationPrice(crate::Event::TaskTriggered {
			who: task.owner_id.clone(),
			task_id: task.task_id.clone(),
		}));

		assert_no_event(RuntimeEvent::AutomationPrice(crate::Event::TaskExecuted {
			who: task.owner_id.clone(),
			task_id: task.task_id.clone(),
		}));

		assert_last_event(RuntimeEvent::AutomationPrice(crate::Event::PriceAlreadyMoved {
			who: task.owner_id.clone(),
			task_id: task.task_id.clone(),
			condition: crate::TaskCondition::PriceAlreadyMoved {
				chain: chain1.to_vec(),
				exchange: exchange1.to_vec(),
				asset_pair: (asset1.to_vec(), asset2.to_vec()),
				price: 1000_u128,
				target_price: 2000_u128,
			},
		}));
	})
}

// When canceling, task is removed from 3 places:
#[test]
fn test_cancel_task_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let creator = AccountId32::new(ALICE);
		let para_id: u32 = 1000;

		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let schedule_fee = MultiLocation::default();
		let execution_fee = AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		};
		let encoded_call_weight = Weight::from_ref_time(100_000);
		let overall_weight = Weight::from_ref_time(200_000);

		let task = Task::<Test> {
			owner_id: creator.into(),
			task_id: "123-0-1".as_bytes().to_vec(),
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset_pair: (asset1.to_vec(), asset2.to_vec()),
			expired_at: START_BLOCK_TIME_1HOUR_AFTER_IN_SECOND,
			trigger_function: "gt".as_bytes().to_vec(),
			trigger_params: vec![123],
			action: Action::XCMP {
				destination,
				schedule_fee,
				execution_fee,
				encoded_call: vec![1, 2, 3],
				encoded_call_weight,
				overall_weight,
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			},
		};
		AutomationPrice::validate_and_schedule_task(task.clone());

		AutomationPrice::cancel_task(
			RuntimeOrigin::signed(task.owner_id.clone()),
			task.task_id.clone(),
		);

		assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::TaskCancelled {
			who: task.owner_id.clone(),
			task_id: task.task_id,
		}));
	})
}

#[test]
fn test_delete_asset_ok() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let sender = AccountId32::new(ALICE);
		let key = (chain1.to_vec(), exchange1.to_vec(), (asset1.to_vec(), asset2.to_vec()));

		setup_asset(&sender, chain1.to_vec());
		AutomationPrice::update_asset_prices(
			RuntimeOrigin::signed(sender.clone()),
			vec![chain1.to_vec()],
			vec![exchange1.to_vec()],
			vec![asset1.to_vec()],
			vec![asset2.to_vec()],
			vec![6789_u128],
			vec![START_BLOCK_TIME as u128],
			vec![4],
		);

		assert!(AutomationPrice::get_asset_registry_info(&key).is_some());
		assert!(AutomationPrice::get_asset_price_data(&key).is_some());

		// Now we delete asset, all the relevant asset metadata and price should be deleted
		AutomationPrice::delete_asset(
			RawOrigin::Root.into(),
			chain1.to_vec(),
			exchange1.to_vec(),
			asset1.to_vec(),
			asset2.to_vec(),
		);

		assert!(AutomationPrice::get_asset_registry_info(&key).is_none());
		assert!(AutomationPrice::get_asset_price_data(&key).is_none());

		assert_has_event(RuntimeEvent::AutomationPrice(crate::Event::AssetDeleted {
			chain: chain1.to_vec(),
			exchange: exchange1.to_vec(),
			asset1: asset1.to_vec(),
			asset2: asset2.to_vec(),
		}));
	})
}
