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
	mock::*, AccountTasks, Action, ActionOf, AssetPayment, Config, Error, InstructionSequence,
	LastTimeSlot, MissedTaskV2Of, ScheduleParam, ScheduledTasksOf, TaskOf, TaskQueueV2, WeightInfo,
};

use codec::Encode;
use frame_support::{
	assert_noop, assert_ok,
	dispatch::GetDispatchInfo,
	pallet_prelude::DispatchError,
	traits::OnInitialize,
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use frame_system::{self, RawOrigin};
use rand::Rng;
use sp_runtime::{
	traits::{BlakeTwo256, Hash},
	AccountId32,
	TokenError::FundsUnavailable,
};
use sp_std::collections::btree_map::BTreeMap;
use xcm::latest::{prelude::*, Junction::Parachain, MultiLocation};

use pallet_valve::Shutdown;

pub const START_BLOCK_TIME: u64 = 33198768000 * 1_000;
const MAX_SCHEDULE_SECONDS: u64 = <Test as Config>::MaxScheduleSeconds::get();
pub const SLOT_SIZE_SECONDS: u64 = <Test as Config>::SlotSizeSeconds::get();
pub const SCHEDULED_TIME: u64 = START_BLOCK_TIME / 1_000 + SLOT_SIZE_SECONDS * 2;
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
			encoded_call_weight: Weight::from_parts(100_000, 0),
			overall_weight: Weight::from_parts(200_000, 0),
			schedule_as: Some(delegator_account),
			instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
		}
	}
}

fn create_xcmp_action(options: XcmpActionParams) -> ActionOf<Test> {
	Action::XCMP {
		destination: options.destination,
		schedule_fee: options.schedule_fee,
		execution_fee: options.execution_fee,
		encoded_call: options.encoded_call,
		encoded_call_weight: options.encoded_call_weight,
		overall_weight: options.overall_weight,
		schedule_as: options.schedule_as,
		instruction_sequence: options.instruction_sequence,
	}
}

fn create_dynamic_dispatch_remark_action(remark: Vec<u8>) -> ActionOf<Test> {
	let call: <Test as frame_system::Config>::RuntimeCall =
		frame_system::Call::remark_with_event { remark }.into();
	Action::DynamicDispatch { encoded_call: call.encode() }
}

fn generate_random_num(min: u32, max: u32) -> u32 {
	rand::thread_rng().gen_range(min, max)
}

fn calculate_local_action_schedule_fee(weight: Weight, num_of_execution: u32) -> u128 {
	NATIVE_EXECUTION_WEIGHT_FEE * (weight.ref_time() as u128) * (num_of_execution as u128)
}

fn calculate_expected_xcmp_action_schedule_fee(
	schedule_fee_location: MultiLocation,
	num_of_execution: u32,
) -> u128 {
	let schedule_fee_location = schedule_fee_location
		.reanchored(&SelfLocation::get(), <Test as Config>::UniversalLocation::get())
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

// when schedule with a Fixed Time schedule and passing an epoch that isn't the
// beginning of hour, raise an error
// the smallest granularity unit we allow is hour
#[test]
fn schedule_invalid_time_fixed_schedule() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		// prepare data
		let call: RuntimeCall = frame_system::Call::remark { remark: vec![12] }.into();

		assert_noop!(
			AutomationTime::schedule_dynamic_dispatch_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				// Simulate epoch of 1 extra second at the beginning of this hour
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME + 1] },
				Box::new(call)
			),
			Error::<Test>::InvalidTime,
		);
	})
}

// The schedule time is beginning of the hour epoch We will arrange our tasks
// into slot of hour and don't support schedule job to granularity of a unit
// that is smaller than hour.
// Verify that we're throwing InvalidTime error when caller doing so
#[test]
fn schedule_invalid_time_recurring_schedule() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		for (next_run, frequency) in vec![
			(SCHEDULED_TIME + 10, 10_u64),
			(SCHEDULED_TIME + SLOT_SIZE_SECONDS, 100_u64),
			(SCHEDULED_TIME + 10, SLOT_SIZE_SECONDS),
		]
		.iter()
		{
			// prepare data
			let call: RuntimeCall = frame_system::Call::remark { remark: vec![12] }.into();
			assert_noop!(
				AutomationTime::schedule_dynamic_dispatch_task(
					RuntimeOrigin::signed(AccountId32::new(ALICE)),
					ScheduleParam::Recurring {
						next_execution_time: *next_run,
						frequency: *frequency
					},
					Box::new(call)
				),
				Error::<Test>::InvalidTime,
			);
		}
	})
}

// when schedule task using Fixed Time Scheduled, if any of the time is in the
// past an error is return and the tasks won't be scheduled
#[test]
fn schedule_past_time() {
	new_test_ext(START_BLOCK_TIME + 1_000 * SLOT_SIZE_SECONDS * 3).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_dynamic_dispatch_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
				Box::new(frame_system::Call::remark { remark: vec![12] }.into())
			),
			Error::<Test>::PastTime,
		);

		assert_noop!(
			AutomationTime::schedule_dynamic_dispatch_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME - SLOT_SIZE_SECONDS] },
				Box::new(frame_system::Call::remark { remark: vec![12] }.into())
			),
			Error::<Test>::PastTime,
		);
	})
}

// when schedule task using Recurring Scheduled, if starting time is in the past,
// an error is return and the tasks won't be scheduled
#[test]
fn schedule_past_time_recurring() {
	new_test_ext(START_BLOCK_TIME + 1_000 * SLOT_SIZE_SECONDS * 3).execute_with(|| {
		for (next_run, frequency) in vec![
			(SCHEDULED_TIME - SLOT_SIZE_SECONDS, SLOT_SIZE_SECONDS * 2),
			(SCHEDULED_TIME, SLOT_SIZE_SECONDS * 2),
		]
		.iter()
		{
			// prepare data
			let call: RuntimeCall = frame_system::Call::remark { remark: vec![12] }.into();
			assert_noop!(
				AutomationTime::schedule_dynamic_dispatch_task(
					RuntimeOrigin::signed(AccountId32::new(ALICE)),
					ScheduleParam::Recurring {
						next_execution_time: *next_run,
						frequency: *frequency
					},
					Box::new(call)
				),
				Error::<Test>::PastTime,
			);
		}
	})
}

// When schedule tasks using Fixed schedule, none of execution time can be too
// far in the future. all element of execution_times need to fall into
//
// When schedule tasks using recurring schedule, either:
//   - next_execution_time cannot too far in the future
//   - next_execution_time is closed, but the frequency is too high
//
#[test]
fn schedule_too_far_out() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		for task_far_schedule in vec![
			// only one time slot that is far
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME + MAX_SCHEDULE_SECONDS] },
			// the first time slot is close, but the rest are too far
			ScheduleParam::Fixed {
				execution_times: vec![SCHEDULED_TIME, SCHEDULED_TIME + MAX_SCHEDULE_SECONDS],
			},
			// the next_execution_time is too far
			ScheduleParam::Recurring {
				next_execution_time: SCHEDULED_TIME + MAX_SCHEDULE_SECONDS,
				frequency: SLOT_SIZE_SECONDS,
			},
			// the next_execution_time is closed, but frequency is too big, make it further to
			// future
			ScheduleParam::Recurring {
				next_execution_time: SCHEDULED_TIME,
				frequency: 7 * MAX_SCHEDULE_SECONDS,
			},
		]
		.iter()
		{
			assert_noop!(
				AutomationTime::schedule_dynamic_dispatch_task(
					RuntimeOrigin::signed(AccountId32::new(ALICE)),
					task_far_schedule.clone(),
					Box::new(frame_system::Call::remark { remark: vec![12] }.into())
				),
				Error::<Test>::TimeTooFarOut,
			);
		}
	})
}

#[test]
fn schedule_not_enough_for_fees() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_dynamic_dispatch_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
				Box::new(frame_system::Call::remark { remark: vec![12] }.into())
			),
			Error::<Test>::InsufficientBalance,
		);
	})
}

// test schedule transfer with dynamic dispatch.
#[test]
fn schedule_transfer_with_dynamic_dispatch() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let account_id = AccountId32::new(ALICE);
		let task_id = FIRST_TASK_ID.to_vec();

		fund_account(&account_id, 900_000_000, 2, Some(0));

		let call: <Test as frame_system::Config>::RuntimeCall =
			pallet_balances::Call::transfer { dest: AccountId32::new(BOB), value: 127 }.into();

		assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
			RuntimeOrigin::signed(account_id.clone()),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(call.clone()),
		));

		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(900_000_000, 0));
		let my_events = events();

		let recipient = AccountId32::new(BOB);
		assert_eq!(Balances::free_balance(recipient.clone()), 127);

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), SCHEDULED_TIME.to_string().into_bytes());

		assert!(contains_events(
			my_events,
			vec![
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: account_id.clone(),
					task_id: task_id.clone(),
					condition,
					encoded_call: Some(call.encode()),
				}),
				RuntimeEvent::Balances(pallet_balances::pallet::Event::Transfer {
					from: account_id.clone(),
					to: recipient,
					amount: 127,
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: account_id.clone(),
					task_id: task_id.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: account_id,
					task_id,
				}),
			]
		));
	})
}

// The TaskCompleted event is emitted only when the task is successfully completed.
#[test]
fn will_emit_task_completed_event_when_task_completed() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let account_id = AccountId32::new(ALICE);
		let _task_id = FIRST_TASK_ID.to_vec();

		fund_account(&account_id, 900_000_000, 2, Some(0));

		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: vec![0] }.into();

		// Schedule a task to be executed at SCHEDULED_TIME and SCHEDULED_TIME + frequency.
		let next_execution_time = SCHEDULED_TIME + SLOT_SIZE_SECONDS;
		assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
			RuntimeOrigin::signed(account_id),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME, next_execution_time] },
			Box::new(call),
		));

		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		// First execution
		AutomationTime::trigger_tasks(Weight::from_parts(900_000_000, 0));
		let my_events = events();

		let event = my_events.into_iter().find(|e| {
			matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskCompleted { .. }))
		});

		if event.is_some() {
			panic!("TaskCompleted event should not be emitted when task is not completed");
		}

		// Second execution
		Timestamp::set_timestamp(next_execution_time * 1_000);
		System::reset_events();
		AutomationTime::trigger_tasks(Weight::from_parts(900_000_000, 0));
		let my_events = events();

		my_events
			.into_iter()
			.find(|e| matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskCompleted { .. })))
			.expect("TaskCompleted event should be emitted when task is completed");
	})
}

// The TaskCompleted event will not be emitted when the task is canceled.
#[test]
fn will_not_emit_task_completed_event_when_task_canceled() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let account_id = AccountId32::new(ALICE);
		let task_id = FIRST_TASK_ID.to_vec();

		fund_account(&account_id, 900_000_000, 2, Some(0));

		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: vec![0] }.into();

		// Schedule a task to be executed at SCHEDULED_TIME and SCHEDULED_TIME + frequency.
		let next_execution_time = SCHEDULED_TIME + SLOT_SIZE_SECONDS;
		assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
			RuntimeOrigin::signed(account_id.clone()),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME, next_execution_time] },
			Box::new(call),
		));

		// First execution
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(900_000_000, 0));
		let my_events = events();

		let event = my_events.into_iter().find(|e| {
			matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskCompleted { .. }))
		});

		if event.is_some() {
			panic!("TaskCompleted event should not be emitted when task is not completed");
		}

		assert_ok!(AutomationTime::cancel_task(RuntimeOrigin::signed(account_id), task_id));

		// Second execution
		Timestamp::set_timestamp(next_execution_time * 1_000);
		System::reset_events();
		AutomationTime::trigger_tasks(Weight::from_parts(900_000_000, 0));
		let my_events = events();

		// The TaskCompleted event will not be emitted when the task is canceled
		let event = my_events.into_iter().find(|e| {
			matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskCompleted { .. }))
		});
		if event.is_some() {
			panic!("The TaskCompleted event will not be emitted when the task is canceled");
		}
	})
}

// The task will be remove from account tasks when the task is canceled with schedule_as.
#[test]
fn will_remove_task_from_account_tasks_when_task_canceled_with_schedule_as() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let schedule_as = AccountId32::new(DELEGATOR_ACCOUNT);
		let task_owner = AccountId32::new(PROXY_ACCOUNT);
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let task_id = FIRST_TASK_ID.to_vec();

		fund_account(&task_owner, 900_000_000, 2, Some(0));

		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: vec![0] }.into();

		// Schedule task
		assert_ok!(AutomationTime::schedule_xcmp_task_through_proxy(
			RuntimeOrigin::signed(task_owner.clone()),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(destination.into()),
			Box::new(NATIVE_LOCATION.into()),
			Box::new(AssetPayment {
				asset_location: MultiLocation::new(0, Here).into(),
				amount: 10
			}),
			call.encode(),
			Weight::from_parts(100_000, 0),
			Weight::from_parts(200_000, 0),
			schedule_as.clone(),
		));

		// Check if the task's schedule_as is correct
		let task = AccountTasks::<Test>::get(task_owner.clone(), task_id.clone());
		assert_eq!(task.is_some(), true);

		let task = task.unwrap();
		assert_eq!(
			matches!(task.clone().action, Action::XCMP { schedule_as: Some(ref s), .. } if s == &schedule_as),
			true
		);

		// Cancel task with schedule_as
		assert_ok!(AutomationTime::cancel_task_with_schedule_as(
			RuntimeOrigin::signed(schedule_as),
			task_owner.clone(),
			task_id.clone(),
		));

		// Verify that the task is no longer in the accountTasks.
		assert_eq!(AutomationTime::get_account_task(task_owner, task_id), None);
	})
}

// Calling cancel_task_with_schedule_as with a schedule_as account will cause TaskScheduleAsNotMatch error.
#[test]
fn cancel_task_with_incorrect_schedule_as_will_fail() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let schedule_as = AccountId32::new(DELEGATOR_ACCOUNT);
		let task_owner = AccountId32::new(PROXY_ACCOUNT);
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let task_id = FIRST_TASK_ID.to_vec();

		fund_account(&task_owner, 900_000_000, 2, Some(0));

		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: vec![0] }.into();

		// Schedule task
		assert_ok!(AutomationTime::schedule_xcmp_task_through_proxy(
			RuntimeOrigin::signed(task_owner.clone()),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(destination.into()),
			Box::new(NATIVE_LOCATION.into()),
			Box::new(AssetPayment {
				asset_location: MultiLocation::new(0, Here).into(),
				amount: 10
			}),
			call.encode(),
			Weight::from_parts(100_000, 0),
			Weight::from_parts(200_000, 0),
			schedule_as.clone(),
		));

		// Check if the task's schedule_as is correct
		let task = AccountTasks::<Test>::get(task_owner.clone(), task_id.clone());
		assert_eq!(task.is_some(), true);

		let task = task.unwrap();
		assert_eq!(
			matches!(task.clone().action, Action::XCMP { schedule_as: Some(ref s), .. } if s == &schedule_as),
			true
		);

		// Cancel task with incorrect schedule_as
		// It will throw TaskScheduleAsNotMatch error
		assert_noop!(
			AutomationTime::cancel_task_with_schedule_as(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				task_owner.clone(),
				task_id.clone(),
			),
			Error::<Test>::TaskScheduleAsNotMatch
		);

		// Assert that the task is still present in accountTasks.
		assert_eq!(
			matches!(task.clone().action, Action::XCMP { schedule_as: Some(ref s), .. } if s == &schedule_as),
			true
		);
	})
}

// Calling cancel_task_with_schedule_as with a non-existent taskid will cause TaskDoesNotExist error.
#[test]
fn cancel_with_schedule_as_and_non_existent_taskid_will_fail() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let schedule_as = AccountId32::new(DELEGATOR_ACCOUNT);
		let task_owner = AccountId32::new(PROXY_ACCOUNT);
		let task_id = FIRST_TASK_ID.to_vec();

		fund_account(&task_owner, 900_000_000, 2, Some(0));

		// Cancel task with non-existent taskid
		// It will throw TaskDoesNotExist error
		assert_noop!(
			AutomationTime::cancel_task_with_schedule_as(
				RuntimeOrigin::signed(schedule_as),
				task_owner.clone(),
				task_id.clone(),
			),
			Error::<Test>::TaskDoesNotExist
		);
	})
}

// Calling cancel_task_with_schedule_as with an incorrect owner will cause TaskDoesNotExist error.
#[test]
fn cancel_with_schedule_as_and_incorrect_owner_will_fail() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let schedule_as = AccountId32::new(DELEGATOR_ACCOUNT);
		let task_owner = AccountId32::new(PROXY_ACCOUNT);
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let task_id = FIRST_TASK_ID.to_vec();

		fund_account(&task_owner, 900_000_000, 2, Some(0));

		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: vec![0] }.into();

		// Schedule task
		assert_ok!(AutomationTime::schedule_xcmp_task_through_proxy(
			RuntimeOrigin::signed(task_owner.clone()),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(destination.into()),
			Box::new(NATIVE_LOCATION.into()),
			Box::new(AssetPayment {
				asset_location: MultiLocation::new(0, Here).into(),
				amount: 10
			}),
			call.encode(),
			Weight::from_parts(100_000, 0),
			Weight::from_parts(200_000, 0),
			schedule_as.clone(),
		));

		// Check if the task's schedule_as is correct
		let task = AccountTasks::<Test>::get(task_owner.clone(), task_id.clone());
		assert_eq!(task.is_some(), true);

		let task = task.unwrap();
		assert_eq!(
			matches!(task.clone().action, Action::XCMP { schedule_as: Some(ref s), .. } if s == &schedule_as),
			true
		);

		// Cancel task with incorrect owner
		// It will throw TaskDoesNotExist error
		assert_noop!(
			AutomationTime::cancel_task_with_schedule_as(
				RuntimeOrigin::signed(schedule_as.clone()),
				AccountId32::new(ALICE),
				task_id.clone(),
			),
			Error::<Test>::TaskDoesNotExist
		);

		// Assert that the task is still present in accountTasks.
		assert_eq!(
			matches!(task.clone().action, Action::XCMP { schedule_as: Some(ref s), .. } if s == &schedule_as),
			true
		);
	})
}

// When a task fails, the TaskCompleted event will still be emitted.
#[test]
fn will_emit_task_completed_event_when_task_failed() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let account_id = AccountId32::new(ALICE);
		let task_id = FIRST_TASK_ID.to_vec();

		fund_account(&account_id, 900_000_000, 2, Some(0));
		let current_funds = Balances::free_balance(account_id.clone());

		// Because the execution of the transfer task twice requires a total amount is larger than current balance, the second task will fail.
		let call: <Test as frame_system::Config>::RuntimeCall = pallet_balances::Call::transfer {
			dest: AccountId32::new(BOB),
			value: current_funds / 2 + 1,
		}
		.into();

		// Schedule a task to be executed at SCHEDULED_TIME and SCHEDULED_TIME + frequency.
		let next_execution_time = SCHEDULED_TIME + SLOT_SIZE_SECONDS;
		assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
			RuntimeOrigin::signed(account_id.clone()),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME, next_execution_time] },
			Box::new(call.clone()),
		));

		// First execution
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(900_000_000, 0));
		let my_events = events();

		let event = my_events.into_iter().find(|e| {
			matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskCompleted { .. }))
		});

		if event.is_some() {
			panic!("TaskCompleted event should not be emitted when task is not completed");
		}

		// Second execution
		Timestamp::set_timestamp(next_execution_time * 1_000);
		System::reset_events();
		AutomationTime::trigger_tasks(Weight::from_parts(900_000_000, 0));
		let my_events = events();

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition
			.insert("timestamp".as_bytes().to_vec(), next_execution_time.to_string().into_bytes());

		assert!(contains_events(
			my_events,
			vec![
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: account_id.clone(),
					task_id: task_id.clone(),
					condition,
					encoded_call: Some(call.encode()),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecutionFailed {
					who: account_id.clone(),
					task_id: task_id.clone(),
					error: DispatchError::Token(FundsUnavailable),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: account_id,
					task_id,
				}),
			]
		))
	})
}

#[test]
fn calculate_auto_compound_action_schedule_fee_amount_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);
		let delegator = AccountId32::new(ALICE);
		let collator = AccountId32::new(COLLATOR_ACCOUNT);
		let action =
			Action::AutoCompoundDelegatedStake { delegator, collator, account_minimum: 100u128 };

		let fee_amount = AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution)
			.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		let weight = <Test as Config>::WeightInfo::run_auto_compound_delegated_stake_task();
		let expected_schedule_fee_amount =
			calculate_local_action_schedule_fee(weight, num_of_execution);

		assert_eq!(fee_amount, expected_schedule_fee_amount);
	})
}

#[test]
fn calculate_dynamic_dispatch_action_schedule_fee_amount_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: vec![0] }.into();
		let action = Action::DynamicDispatch { encoded_call: call.encode() };

		let fee_amount = AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution)
			.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		let weight = <Test as Config>::WeightInfo::run_dynamic_dispatch_action()
			.saturating_add(call.get_dispatch_info().weight);
		let expected_schedule_fee_amount =
			calculate_local_action_schedule_fee(weight, num_of_execution);

		assert_eq!(fee_amount, expected_schedule_fee_amount);
	})
}

#[test]
fn calculate_xcmp_action_schedule_fee_amount_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);
		let action = create_xcmp_action(XcmpActionParams::default());
		let fee_amount = AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution)
			.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);
		let expected_schedule_fee_amount = calculate_expected_xcmp_action_schedule_fee(
			action.schedule_fee_location::<Test>(),
			num_of_execution,
		);
		assert_eq!(fee_amount, expected_schedule_fee_amount);
	})
}

#[test]
fn calculate_xcmp_action_schedule_fee_amount_with_different_schedule_fees_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		ASSET_FEE_PER_SECOND.into_iter().for_each(|fee| {
			let num_of_execution = generate_random_num(1, 20);
			let action = create_xcmp_action(XcmpActionParams {
				schedule_fee: fee.asset_location,
				..XcmpActionParams::default()
			});

			let fee_amount =
				AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution)
					.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

			let expected_schedule_fee_amount =
				calculate_expected_xcmp_action_schedule_fee(fee.asset_location, num_of_execution);

			assert_eq!(fee_amount, expected_schedule_fee_amount);
		});
	})
}

#[test]
fn calculate_xcmp_action_schedule_fee_amount_with_absolute_or_relative_native_schedule_fee_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);

		let action_absolute = create_xcmp_action(XcmpActionParams {
			schedule_fee: SelfLocation::get(),
			..XcmpActionParams::default()
		});
		let fee_amount_abosolute =
			AutomationTime::calculate_schedule_fee_amount(&action_absolute, num_of_execution)
				.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		let action_relative = create_xcmp_action(XcmpActionParams {
			schedule_fee: MultiLocation::new(0, Here),
			..XcmpActionParams::default()
		});
		let fee_amount_relative =
			AutomationTime::calculate_schedule_fee_amount(&action_relative, num_of_execution)
				.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		let expected_schedule_fee_amount = calculate_expected_xcmp_action_schedule_fee(
			MultiLocation::new(0, Here),
			num_of_execution,
		);

		assert_eq!(fee_amount_abosolute, fee_amount_relative);
		assert_eq!(fee_amount_abosolute, expected_schedule_fee_amount);
	})
}

#[test]
fn calculate_xcmp_action_schedule_fee_amount_with_different_destination_returns_same_result() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);

		let action = create_xcmp_action(XcmpActionParams {
			destination: MultiLocation::new(1, X1(Parachain(PARA_ID))),
			..XcmpActionParams::default()
		});
		let fee_amount = AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution)
			.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		let action_diffrent_dest = create_xcmp_action(XcmpActionParams {
			destination: MultiLocation::new(1, X1(Parachain(3000))),
			..XcmpActionParams::default()
		});
		let fee_amount_diffrent_dest =
			AutomationTime::calculate_schedule_fee_amount(&action_diffrent_dest, num_of_execution)
				.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		assert_eq!(fee_amount, fee_amount_diffrent_dest);
	})
}

#[test]
fn calculate_xcmp_action_schedule_fee_amount_with_different_execution_fee_returns_same_result() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);

		let action = create_xcmp_action(XcmpActionParams {
			execution_fee: AssetPayment {
				asset_location: MOONBASE_ASSET_LOCATION.into(),
				amount: 100,
			},
			..XcmpActionParams::default()
		});
		let fee_amount = AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution)
			.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		let action_diffrent_execution_fee = create_xcmp_action(XcmpActionParams {
			execution_fee: AssetPayment {
				asset_location: MultiLocation::new(1, X1(Parachain(3000))).into(),
				amount: 300,
			},
			..XcmpActionParams::default()
		});
		let fee_amount_diffrent_execution_fee = AutomationTime::calculate_schedule_fee_amount(
			&action_diffrent_execution_fee,
			num_of_execution,
		)
		.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		assert_eq!(fee_amount, fee_amount_diffrent_execution_fee);
	})
}

#[test]
fn calculate_xcmp_action_schedule_fee_amount_with_different_instruction_sequence_returns_same_result(
) {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);

		let action = create_xcmp_action(XcmpActionParams {
			instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
			..XcmpActionParams::default()
		});
		let fee_amount = AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution)
			.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		let action_diff_instruction_sequence = create_xcmp_action(XcmpActionParams {
			instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			..XcmpActionParams::default()
		});
		let fee_amount_diff_instruction_sequence = AutomationTime::calculate_schedule_fee_amount(
			&action_diff_instruction_sequence,
			num_of_execution,
		)
		.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		assert_eq!(fee_amount, fee_amount_diff_instruction_sequence);
	})
}

#[test]
fn calculate_xcmp_action_schedule_fee_amount_with_different_schedule_as_returns_same_result() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);

		let action = create_xcmp_action(XcmpActionParams {
			schedule_as: Some(ALICE.into()),
			..XcmpActionParams::default()
		});
		let fee_amount = AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution)
			.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		let action_different_schedule_as = create_xcmp_action(XcmpActionParams {
			schedule_as: Some(BOB.into()),
			..XcmpActionParams::default()
		});
		let fee_amount_different_schedule_as = AutomationTime::calculate_schedule_fee_amount(
			&action_different_schedule_as,
			num_of_execution,
		)
		.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);

		assert_eq!(fee_amount, fee_amount_different_schedule_as);
	})
}

#[test]
fn calculate_xcmp_action_schedule_fee_amount_with_unknown_schedule_fees_fails() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let num_of_execution = generate_random_num(1, 20);
		let action = create_xcmp_action(XcmpActionParams {
			schedule_fee: UNKNOWN_SCHEDULE_FEE,
			..XcmpActionParams::default()
		});
		assert_noop!(
			AutomationTime::calculate_schedule_fee_amount(&action, num_of_execution),
			sp_runtime::DispatchError::Other("CouldNotDetermineFeePerSecond"),
		);
	})
}

#[test]
fn schedule_xcmp_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let alice = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		// Funds including XCM fees
		get_xcmp_funds(alice.clone());

		assert_ok!(AutomationTime::schedule_xcmp_task(
			RuntimeOrigin::signed(alice),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(destination.into()),
			Box::new(NATIVE_LOCATION.into()),
			Box::new(AssetPayment {
				asset_location: MultiLocation::new(0, Here).into(),
				amount: 10
			}),
			call,
			Weight::from_parts(100_000, 0),
			Weight::from_parts(200_000, 0),
		));
	})
}

#[test]
fn schedule_xcmp_works_with_multi_currency() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let alice = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		// Funds including XCM fees
		get_multi_xcmp_funds(alice.clone());

		assert_ok!(AutomationTime::schedule_xcmp_task(
			RuntimeOrigin::signed(alice),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(destination.into()),
			Box::new(NATIVE_LOCATION.into()),
			Box::new(AssetPayment { asset_location: destination.into(), amount: 10 }),
			call,
			Weight::from_parts(100_000, 0),
			Weight::from_parts(200_000, 0),
		));
	})
}

#[test]
fn schedule_xcmp_works_with_unsupported_currency_will_fail() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let alice = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		// Funds including XCM fees
		get_multi_xcmp_funds(alice.clone());

		assert_noop!(
			AutomationTime::schedule_xcmp_task(
				RuntimeOrigin::signed(alice),
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
				Box::new(destination.into()),
				Box::new(NATIVE_LOCATION.into()),
				Box::new(AssetPayment {
					asset_location: MultiLocation::new(1, X1(Parachain(3000))).into(),
					amount: 10
				}),
				call,
				Weight::from_parts(100_000, 0),
				Weight::from_parts(200_000, 0),
			),
			Error::<Test>::UnsupportedFeePayment,
		);
	})
}

#[test]
fn schedule_xcmp_through_proxy_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let delegator_account = AccountId32::new(DELEGATOR_ACCOUNT);
		let proxy_account = AccountId32::new(PROXY_ACCOUNT);
		let call: Vec<u8> = vec![2, 4, 5];

		// Funds including XCM fees
		get_xcmp_funds(proxy_account.clone());

		assert_ok!(AutomationTime::schedule_xcmp_task_through_proxy(
			RuntimeOrigin::signed(proxy_account.clone()),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(destination.into()),
			Box::new(MultiLocation::default().into()),
			Box::new(AssetPayment { asset_location: destination.into(), amount: 10 }),
			call,
			Weight::from_parts(100_000, 0),
			Weight::from_parts(200_000, 0),
			delegator_account.clone(),
		));

		let tasks = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME);
		assert!(tasks.is_some());

		let tasks = tasks.unwrap();
		assert_eq!(tasks.tasks[0].0, proxy_account);

		// Find the TaskScheduled event in the event list and verify if the who within it is correct.
		events()
			.into_iter()
			.find(|e| match e {
				RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
				who,
				schedule_as,
				..
			}) if *who == proxy_account && *schedule_as == Some(delegator_account.clone()) => true,
				_ => false,
			})
			.expect("TaskScheduled event should emit with 'who' being proxy_account, and 'schedule_as' being delegator_account.");
	})
}

#[test]
fn schedule_xcmp_through_proxy_same_as_delegator_account() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let delegator_account = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));

		// Funds including XCM fees
		get_xcmp_funds(delegator_account.clone());

		assert_noop!(
			AutomationTime::schedule_xcmp_task_through_proxy(
				RuntimeOrigin::signed(delegator_account.clone()),
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
				Box::new(destination.into()),
				Box::new(MultiLocation::default().into()),
				Box::new(AssetPayment { asset_location: destination.into(), amount: 10 }),
				call,
				Weight::from_parts(100_000, 0),
				Weight::from_parts(200_000, 0),
				delegator_account,
			),
			sp_runtime::DispatchError::Other("proxy error: expected `ProxyType::Any`"),
		);
	})
}

#[test]
fn schedule_xcmp_fails_if_not_enough_funds() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let para_id: u32 = 1000;
		let alice = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		// Funds not including XCM fees
		get_minimum_funds(alice.clone(), 1);

		assert_noop!(
			AutomationTime::schedule_xcmp_task(
				RuntimeOrigin::signed(alice),
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
				Box::new(destination.into()),
				Box::new(NATIVE_LOCATION.into()),
				Box::new(AssetPayment {
					asset_location: MultiLocation::new(0, Here).into(),
					amount: 10000000000000
				}),
				call,
				Weight::from_parts(100_000, 0),
				Weight::from_parts(200_000, 0),
			),
			Error::<Test>::InsufficientBalance,
		);
	})
}

#[test]
fn schedule_auto_compound_delegated_stake() {
	let alice = AccountId32::new(ALICE);
	let bob = AccountId32::new(BOB);
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(alice.clone());
		assert_ok!(AutomationTime::schedule_auto_compound_delegated_stake_task(
			RuntimeOrigin::signed(alice.clone()),
			SCHEDULED_TIME,
			SLOT_SIZE_SECONDS,
			bob.clone(),
			1_000_000_000,
		));
		let account_task_id = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME)
			.expect("Task should be scheduled")
			.tasks[0]
			.clone();
		assert_eq!(
			AutomationTime::get_account_task(account_task_id.0.clone(), account_task_id.1),
			TaskOf::<Test>::create_auto_compound_delegated_stake_task::<Test>(
				alice.clone(),
				FIRST_TASK_ID.to_vec(),
				SCHEDULED_TIME,
				SLOT_SIZE_SECONDS,
				bob,
				1_000_000_000,
				vec![],
			)
			.ok()
		);
	})
}

// Auto compounding use Recurring schedule to perform tasks.
// Thus the next_execution_time and frequency needs to follow the rule such as
// next_execution_time needs to fall into beginning of a hour block, and
// frequency must be a multiplier of SLOT_SIZE_SECONDS
#[test]
fn schedule_auto_compound_with_bad_frequency_or_execution_time() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		for (bad_execution_time, bad_frequency) in vec![
			// execute_with is valid, frequency invalid
			(SCHEDULED_TIME, 4_000),
			(SCHEDULED_TIME, 0),
			// execute_with is invalid, frequency is  valid
			(SCHEDULED_TIME + SLOT_SIZE_SECONDS / 2, SLOT_SIZE_SECONDS),
		]
		.iter()
		{
			assert_noop!(
				AutomationTime::schedule_auto_compound_delegated_stake_task(
					RuntimeOrigin::signed(AccountId32::new(ALICE)),
					*bad_execution_time,
					*bad_frequency,
					AccountId32::new(BOB),
					100_000,
				),
				Error::<Test>::InvalidTime
			);
		}
	})
}

// when schedule auto compound task, if the schedule time falls too far in the
// future, return TimeTooFarOut error
#[test]
fn schedule_auto_compound_with_time_too_far_out() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		for (execution_time, frequency) in vec![
			(SCHEDULED_TIME, MAX_SCHEDULE_SECONDS + SLOT_SIZE_SECONDS),
			(SCHEDULED_TIME + MAX_SCHEDULE_SECONDS + SLOT_SIZE_SECONDS, SLOT_SIZE_SECONDS),
		]
		.iter()
		{
			assert_noop!(
				AutomationTime::schedule_auto_compound_delegated_stake_task(
					RuntimeOrigin::signed(AccountId32::new(ALICE)),
					*execution_time,
					*frequency,
					AccountId32::new(BOB),
					100_000,
				),
				Error::<Test>::TimeTooFarOut
			);
		}
	})
}

#[test]
fn get_auto_compound_delegated_stake_task_ids_return_only_auto_compount_task_id() {
	let owner = AccountId32::new(ALICE);
	let delegator = AccountId32::new(BOB);

	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(owner.clone());
		assert_ok!(AutomationTime::schedule_auto_compound_delegated_stake_task(
			RuntimeOrigin::signed(owner.clone()),
			SCHEDULED_TIME,
			SLOT_SIZE_SECONDS,
			delegator.clone(),
			1_000_000_000,
		));

		let account_task_id = last_task_id();

		fund_account(&owner, 900_000_000, 1, Some(0));
		let call: RuntimeCall = frame_system::Call::remark { remark: vec![2, 4, 5] }.into();
		assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
			RuntimeOrigin::signed(owner.clone()),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(call),
		));
		let non_autocompound_task_id = last_task_id();

		assert_ne!(
			account_task_id, non_autocompound_task_id,
			"autocompound task id and a normal task id should be different"
		);

		// Now when we get auto compount test, we should get only once
		assert_eq!(
			AutomationTime::get_auto_compound_delegated_stake_task_ids(owner),
			vec![account_task_id],
		);
	})
}

// test that we cannot schedule another task with the same id
// Because the ID is auto-generated now so to test this scenerio, we use the
// normal schedule call to schedule a task and call into a low level API
// to schedule a new task with that same last generated task id to observe
// the error
#[test]
fn schedule_duplicates_errors() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		get_funds(owner.clone());
		let task_id = schedule_task(ALICE, vec![SCHEDULED_TIME], vec![2, 4]);

		let task2 = TaskOf::<Test>::create_event_task::<Test>(
			owner,
			task_id,
			vec![SCHEDULED_TIME],
			vec![10, 12],
			vec![],
		)
		.unwrap();

		assert_noop!(AutomationTime::schedule_task(&task2), Error::<Test>::DuplicateTask,);
	})
}

// there is an upper limit of how many time slot in Fixed Scheduled, when
// passing a large enough array we return TooManyExecutionsTimes error
#[test]
fn schedule_max_execution_times_errors() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let call: RuntimeCall = frame_system::Call::remark { remark: vec![2, 4, 5] }.into();
		assert_ok!(fund_account_dynamic_dispatch(
			&AccountId32::new(ALICE),
			// fake schedule 4 times in the schedule param below
			4,
			call.encode()
		));
		assert_noop!(
			AutomationTime::schedule_dynamic_dispatch_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				ScheduleParam::Fixed {
					execution_times: vec![
						SCHEDULED_TIME,
						SCHEDULED_TIME + SLOT_SIZE_SECONDS,
						SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
						SCHEDULED_TIME + SLOT_SIZE_SECONDS * 3
					]
				},
				Box::new(frame_system::Call::remark { remark: vec![2, 4, 5] }.into())
			),
			Error::<Test>::TooManyExecutionsTimes,
		);
	})
}

// when user made mistake and pass duplicate time slot on Fixed Schedule, we
// attempt to correct it and store the corrected schedule on-chain
// Verified that the stored schedule is corrected without any duplication
#[test]
fn schedule_execution_times_removes_dupes() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);

		get_funds(owner.clone());
		let task_id1 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME,
				SCHEDULED_TIME,
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 3,
			],
			vec![2, 4],
		);

		match AutomationTime::get_account_task(owner, task_id1) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				let expected_task = TaskOf::<Test>::create_event_task::<Test>(
					AccountId32::new(ALICE),
					vec![49, 45, 48, 45, 52],
					vec![SCHEDULED_TIME, SCHEDULED_TIME + SLOT_SIZE_SECONDS * 3],
					vec![2, 4],
					vec![],
				)
				.unwrap();

				assert_eq!(task, expected_task);
			},
		}
	})
}

// For a given tasks slot, we don't want to have too many small, light weight
// tasks or have just a handful tasks but the total weight is over the limit
// We guard with a max tasks per slot and max weight per slot.
//
// Verify that when the slot has enough tasks, new task cannot be scheduled, and
// an error TimeSlotFull is returned.
//
// we mock the MaxTasksPerSlot=2 in mocks.rs
#[test]
fn schedule_time_slot_full() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let call1: RuntimeCall = frame_system::Call::remark { remark: vec![2, 4] }.into();
		assert_ok!(fund_account_dynamic_dispatch(&AccountId32::new(ALICE), 1, call1.encode()));

		assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(call1)
		));

		let call2: RuntimeCall = frame_system::Call::remark { remark: vec![2, 4, 5] }.into();
		assert_ok!(fund_account_dynamic_dispatch(&AccountId32::new(ALICE), 1, call2.encode()));
		assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			Box::new(call2)
		));

		let call3: RuntimeCall = frame_system::Call::remark { remark: vec![2] }.into();
		assert_ok!(fund_account_dynamic_dispatch(&AccountId32::new(ALICE), 1, call3.encode()));
		assert_noop!(
			AutomationTime::schedule_dynamic_dispatch_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
				Box::new(call3)
			),
			Error::<Test>::TimeSlotFull,
		);
	})
}

// test case when a slot in full, we will roll back its state atomically
// and won't leave the task queue in a partial state.
//
// It's similar to above test. However, we test a task that has scheduled
// with many execution_times where as only a few execution_time slots are full
// while the rest of execution_time slots aren't full.
//
// even though other time slots aren't full, we still reject as a whole, return
// TimeSlotFull error and verify that none of the tasks has been scheduled into any
// time slot, even the one that isn't full.
//
// in other word, task scheduled is atomic, all task executions need to be able
// to put into the schedule task slots, otherwise none of data should be stored
// partially
#[test]
fn schedule_time_slot_full_rolls_back() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let call1: RuntimeCall = frame_system::Call::remark { remark: vec![2, 4, 5] }.into();
		assert_ok!(fund_account_dynamic_dispatch(&AccountId32::new(ALICE), 1, call1.encode()));

		let task_id1 =
			schedule_task(ALICE, vec![SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2], vec![2, 4, 5]);

		let _call2: RuntimeCall = frame_system::Call::remark { remark: vec![2, 4] }.into();
		assert_ok!(fund_account_dynamic_dispatch(&AccountId32::new(ALICE), 1, call1.encode()));
		let task_id2 =
			schedule_task(ALICE, vec![SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2], vec![2, 4]);

		let call: RuntimeCall = frame_system::Call::remark { remark: vec![2] }.into();
		assert_ok!(fund_account_dynamic_dispatch(&AccountId32::new(ALICE), 1, call.encode()));
		assert_noop!(
			AutomationTime::schedule_dynamic_dispatch_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				ScheduleParam::Fixed {
					execution_times: vec![
						SCHEDULED_TIME,
						SCHEDULED_TIME + SLOT_SIZE_SECONDS,
						SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2
					]
				},
				Box::new(call)
			),
			Error::<Test>::TimeSlotFull,
		);

		if AutomationTime::get_scheduled_tasks(SCHEDULED_TIME).is_some() {
			panic!("Tasks scheduled for the time it should have been rolled back")
		}
		if AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + SLOT_SIZE_SECONDS).is_some() {
			panic!("Tasks scheduled for the time it should have been rolled back")
		}
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: account_task_ids, .. }) => {
				assert_eq!(account_task_ids.len(), 2);
				assert_eq!(account_task_ids[0].1, task_id1);
				assert_eq!(account_task_ids[1].1, task_id2);
			},
		}
	})
}

// verify that task scheduled in different block has the right id
#[test]
fn taskid_changed_per_block() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id1 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);

		System::set_block_number(20);
		let task_id2 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
		));

		assert_eq!(task_id1, FIRST_TASK_ID.to_vec());
		assert_eq!(task_id2, vec![50, 48, 45, 48, 45, 54]);
	})
}

// verify that task scheduled in same block with different extrinsic index has different tx id
#[test]
fn taskid_adjusted_on_extrinsicid_on_same_block() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let first_caller = AccountId32::new(ALICE);
		let task_id1 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);

		// Set to a high and more than one digit extrinsic index to test task_id also match
		System::set_extrinsic_index(234);

		let second_caller = AccountId32::new(BOB);
		let task_id2 = schedule_task(
			BOB,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
		));

		assert_eq!(task_id1, FIRST_TASK_ID.to_vec());
		assert_eq!(task_id2, vec![49, 45, 50, 51, 52, 45, 56]);

		assert_has_event(RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
			who: first_caller,
			task_id: FIRST_TASK_ID.to_vec(),
			schedule_as: None,
		}));

		assert_has_event(RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
			who: second_caller,
			task_id: vec![49, 45, 50, 51, 52, 45, 56],
			schedule_as: None,
		}));
	})
}

// verify that task scheduled in same block with different extrinsic index has different tx id
#[test]
fn taskid_adjusted_on_eventindex_on_same_block_from_same_caller() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let caller = AccountId32::new(ALICE);

		let task_id1 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);

		// Set to a high and more than one digit extrinsic index to test task_id also match
		System::set_extrinsic_index(234);

		let task_id2 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);

		// 1-0-3
		assert_eq!(task_id1, "1-0-3".as_bytes().to_vec());
		// 1-234-6
		assert_eq!(task_id2, "1-234-6".as_bytes().to_vec());

		assert_has_event(RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
			who: caller.clone(),
			task_id: "1-0-3".as_bytes().to_vec(),
			schedule_as: None,
		}));

		assert_has_event(RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
			who: caller,
			task_id: "1-234-6".as_bytes().to_vec(),
			schedule_as: None,
		}));
	})
}

// verify that task scheduled in same block with same extrinsic index has different event id
#[test]
fn taskid_on_same_extrinsid_have_unique_event_index() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let task_id1 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);

		let task_id2 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
		));

		assert_eq!(task_id1, FIRST_TASK_ID.to_vec());
		assert_eq!(task_id2, SECOND_TASK_ID.to_vec());

		assert_has_event(RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
			who: owner,
			task_id: FIRST_TASK_ID.to_vec(),
			schedule_as: None,
		}));
	})
}

// verify that the owner of a task can cancel a Fixed schedule task by its id.
// In this test we focus on confirmation of canceling the task that has a single
// execution times
#[test]
fn cancel_works_for_fixed_scheduled() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id1 = schedule_task(ALICE, vec![SCHEDULED_TIME], vec![2, 4, 5]);
		let task_id2 = schedule_task(ALICE, vec![SCHEDULED_TIME], vec![2, 4]);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
		));
		System::reset_events();

		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id1.clone(),
		));
		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id2.clone(),
		));

		if AutomationTime::get_scheduled_tasks(SCHEDULED_TIME).is_some() {
			panic!("Since there were only two tasks scheduled for the time it should have been deleted")
		}
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
					who: AccountId32::new(ALICE),
					task_id: task_id1
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
					who: AccountId32::new(ALICE),
					task_id: task_id2,
				}),
			]
		);
	})
}

// verify that the owner of a task can cancel a Fixed schedule task by its id.
// In this test we focus on confirmation of canceling the task that has many
// execution times
#[test]
fn cancel_works_for_multiple_executions_scheduled() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let task_id1 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
		));
		System::reset_events();

		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(owner.clone()),
			task_id1.clone(),
		));

		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id1.clone()), None);
		if AutomationTime::get_scheduled_tasks(SCHEDULED_TIME).is_some() {
			panic!("Tasks scheduled for the time it should have been deleted")
		}
		if AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + SLOT_SIZE_SECONDS).is_some() {
			panic!("Tasks scheduled for the time it should have been deleted")
		}
		if AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2).is_some() {
			panic!("Tasks scheduled for the time it should have been deleted")
		}
		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
				who: owner,
				task_id: task_id1,
			})]
		);
	})
}

// verify that the owner of a task can cancel a Recurring schedule task by its id
#[test]
fn cancel_works_for_recurring_scheduled() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id1 =
			schedule_recurring_task(ALICE, SCHEDULED_TIME, SLOT_SIZE_SECONDS, vec![2, 4, 5]);
		let task_id2 =
			schedule_recurring_task(ALICE, SCHEDULED_TIME, SLOT_SIZE_SECONDS, vec![2, 4]);

		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
		));
		System::reset_events();

		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id1.clone(),
		));
		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id2.clone(),
		));

		if AutomationTime::get_scheduled_tasks(SCHEDULED_TIME).is_some() {
			panic!("Since there were only two tasks scheduled for the time it should have been deleted")
		}
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
					who: AccountId32::new(ALICE),
					task_id: task_id1
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
					who: AccountId32::new(ALICE),
					task_id: task_id2
				}),
			]
		);
	})
}

// given a Fixed scheduled task that has many executions time, and already ran
// at least one, we can still cancel it to prevent the rest of exectutions in
// subseuent task triggering.
#[test]
fn cancel_works_for_an_executed_task() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let call: RuntimeCall = frame_system::Call::remark_with_event { remark: vec![50] }.into();
		let task_id1 = schedule_dynamic_dispatch_task(
			ALICE,
			vec![SCHEDULED_TIME, SCHEDULED_TIME + SLOT_SIZE_SECONDS],
			call.clone(),
		);
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS,
		));
		System::reset_events();

		match AutomationTime::get_account_task(owner.clone(), task_id1.clone()) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}

		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: task_ids, .. }) => {
				assert_eq!(task_ids.len(), 1);
				assert_eq!(task_ids[0].1, task_id1);
			},
		}
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + SLOT_SIZE_SECONDS) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: task_ids, .. }) => {
				assert_eq!(task_ids.len(), 1);
				assert_eq!(task_ids[0].1, task_id1);
			},
		}

		AutomationTime::trigger_tasks(Weight::from_parts(200_000, 0));
		let my_events = events();

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), SCHEDULED_TIME.to_string().into_bytes());
		assert_eq!(
			my_events,
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id1.clone(),
					condition,
					encoded_call: Some(call.encode()),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&[50]),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id1.clone(),
				}),
			]
		);
		match AutomationTime::get_account_task(owner.clone(), task_id1.clone()) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 1);
			},
		}

		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME), None);
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + SLOT_SIZE_SECONDS) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: task_ids, .. }) => {
				assert_eq!(task_ids.len(), 1);
				assert_eq!(task_ids[0].1, task_id1);
			},
		}

		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id1.clone()
		));

		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME), None);
		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + SLOT_SIZE_SECONDS), None);

		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id1.clone()), None);
		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
				who: owner,
				task_id: task_id1,
			})]
		);
	})
}

// verify that if a tasks is already moved from the schedule slot into the task
// queue, it can still get canceling using its id.
#[test]
fn cancel_works_for_tasks_in_queue() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(vec![2, 4, 5]),
			vec![],
		);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME, SCHEDULED_TIME));

		assert_eq!(task_id, AutomationTime::get_task_queue()[0].1);
		assert_eq!(1, AutomationTime::get_task_queue().len());

		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id.clone(),
		));

		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
				who: AccountId32::new(ALICE),
				task_id
			}),]
		);
		assert_eq!(0, AutomationTime::get_task_queue().len());
	})
}

// verify that when cancelling a non-existed tasks, an error will be return
#[test]
fn cancel_task_must_exist() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		//let task_id = BlakeTwo256::hash_of(&task);
		let task_id = vec![49, 45, 48, 45, 52];

		assert_noop!(
			AutomationTime::cancel_task(RuntimeOrigin::signed(AccountId32::new(ALICE)), task_id),
			Error::<Test>::TaskDoesNotExist,
		);
	})
}

// verify if an account has a task id in its AccountTasks storage, but the
// actual task doesn't exist in any schedule slot or task queue then the cancel
// succeed to remove the task id from AccountTasks storage, but throwing an
// extra TaskNotFound event beside the normal TaskCancelled evented
//
#[test]
fn cancel_task_not_found() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let task = TaskOf::<Test>::create_event_task::<Test>(
			owner.clone(),
			vec![40],
			vec![SCHEDULED_TIME],
			vec![2, 4, 5],
			vec![],
		)
		.unwrap();
		let task_id = vec![49, 45, 48, 45, 49];
		AccountTasks::<Test>::insert(owner.clone(), task_id.clone(), task);

		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(owner.clone()),
			task_id.clone(),
		));
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskNotFound {
					who: owner.clone(),
					task_id: task_id.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
					who: owner,
					task_id: task_id.clone()
				})
			]
		);

		// now ensure the task id is also removed from AccountTasks
		assert_noop!(
			AutomationTime::cancel_task(RuntimeOrigin::signed(AccountId32::new(ALICE)), task_id),
			Error::<Test>::TaskDoesNotExist,
		);
	})
}

// verify only the owner of the task can cancel it
#[test]
fn cancel_task_fail_non_owner() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let task_id1 = schedule_task(
			ALICE,
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS,
				SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2,
			],
			vec![2, 4, 5],
		);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
		));

		System::reset_events();

		// BOB cannot cancel because he isn't the task owner
		assert_noop!(
			AutomationTime::cancel_task(
				RuntimeOrigin::signed(AccountId32::new(BOB)),
				task_id1.clone()
			),
			Error::<Test>::TaskDoesNotExist,
		);

		// But Alice can cancel as expected
		assert_ok!(AutomationTime::cancel_task(RuntimeOrigin::signed(owner), task_id1,));
	})
}

// verifying that root/sudo can force_cancel anybody's tasks
#[test]
fn force_cancel_task_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id = schedule_task(ALICE, vec![SCHEDULED_TIME], vec![2, 4, 5]);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
		));
		System::reset_events();

		assert_ok!(AutomationTime::force_cancel_task(
			RawOrigin::Root.into(),
			AccountId32::new(ALICE),
			task_id.clone()
		));
		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
				who: AccountId32::new(ALICE),
				task_id
			}),]
		);
	})
}

mod extrinsics {
	use super::*;

	mod schedule_dynamic_dispatch_task {
		use super::*;

		#[test]
		fn works() {
			new_test_ext(START_BLOCK_TIME).execute_with(|| {
				let account_id = AccountId32::new(ALICE);
				let execution_times = vec![SCHEDULED_TIME];
				let call: RuntimeCall = frame_system::Call::remark { remark: vec![] }.into();
				assert_ok!(fund_account_dynamic_dispatch(
					&account_id,
					execution_times.len(),
					call.encode()
				));

				assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
					RuntimeOrigin::signed(account_id.clone()),
					ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
					Box::new(call)
				));
				assert_eq!(
					last_event(),
					RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
						who: account_id,
						task_id: FIRST_TASK_ID.to_vec(),
						schedule_as: None,
					})
				);
			})
		}
	}
}

mod run_dynamic_dispatch_action {
	use super::*;

	#[test]
	fn cannot_decode() {
		new_test_ext(START_BLOCK_TIME).execute_with(|| {
			let account_id = AccountId32::new(ALICE);
			let bad_encoded_call: Vec<u8> = vec![1];

			let (weight, error) =
				AutomationTime::run_dynamic_dispatch_action(account_id, bad_encoded_call);

			assert_eq!(
				weight,
				<Test as Config>::WeightInfo::run_dynamic_dispatch_action_fail_decode()
			);

			assert_eq!(error, Some(DispatchError::from(Error::<Test>::CallCannotBeDecoded)));
		})
	}

	#[test]
	fn call_errors() {
		new_test_ext(START_BLOCK_TIME).execute_with(|| {
			let account_id = AccountId32::new(ALICE);
			let call: RuntimeCall = frame_system::Call::set_code { code: vec![] }.into();
			let encoded_call = call.encode();

			let (_, error) = AutomationTime::run_dynamic_dispatch_action(account_id, encoded_call);

			assert_eq!(error, Some(DispatchError::BadOrigin));
		})
	}

	#[test]
	fn call_filtered() {
		new_test_ext(START_BLOCK_TIME).execute_with(|| {
			let account_id = AccountId32::new(ALICE);
			let call: RuntimeCall = pallet_timestamp::Call::set { now: 100 }.into();
			let encoded_call = call.encode();

			let (_, error) = AutomationTime::run_dynamic_dispatch_action(account_id, encoded_call);

			assert_eq!(error, Some(DispatchError::from(frame_system::Error::<Test>::CallFiltered)));
		})
	}

	#[test]
	fn call_works() {
		new_test_ext(START_BLOCK_TIME).execute_with(|| {
			let account_id = AccountId32::new(ALICE);
			let call: RuntimeCall = frame_system::Call::remark { remark: vec![] }.into();
			let encoded_call = call.encode();

			let (_, error) = AutomationTime::run_dynamic_dispatch_action(account_id, encoded_call);

			assert_eq!(error, None);
		})
	}
}

// Weights to use for tests below
// 10_000v: run per missed task (run_missed_tasks_many_found)
// 10_000v: run per task not found in map (run_missed_tasks_many_missing, run_tasks_many_missing)
// 50_000v: weight check for running 1 more task, current static v=1 (run_tasks_many_found)
// 10_000: update task queue function overhead (update_task_queue_overhead)
// 20_000: update task queue for scheduled tasks (update_scheduled_task_queue)
// 20_000v: for each old time slot to missed tasks (append_to_missed_tasks)
// 20_000: to move a single time slot to missed tasks (shift_missed_tasks)

// ensure the first task trigger for first block run properly without error
// and will not emit any event
#[test]
fn trigger_tasks_handles_first_run() {
	new_test_ext(0).execute_with(|| {
		AutomationTime::trigger_tasks(Weight::from_parts(30_000, 0));

		assert_eq!(events(), vec![],);
	})
}

// verify when having no tasks, the trigger run to the end without error
// and there is no emitted event
#[test]
fn trigger_tasks_nothing_to_do() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_parts(30_000, 0));

		assert_eq!(events(), vec![],);
	})
}

// when calling trigger_tasks verifyign that the tasks in schedule of
// current slot are properly moved into the task queue. MissedTask will be moved
// into missed queue
#[test]
fn trigger_tasks_updates_queues() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let missed_task_id = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME - SLOT_SIZE_SECONDS],
			create_dynamic_dispatch_remark_action(vec![40]),
			vec![],
		);
		let missed_task = MissedTaskV2Of::<Test>::new(
			AccountId32::new(ALICE),
			missed_task_id,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS,
		);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		let scheduled_task_id = schedule_task(ALICE, vec![SCHEDULED_TIME], vec![50]);
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS,
		));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(50_000, 0));

		assert_eq!(AutomationTime::get_missed_queue().len(), 1);
		assert_eq!(AutomationTime::get_missed_queue()[0], missed_task);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_task_queue()[0].1, scheduled_task_id);
		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME), None);
		assert_eq!(events(), vec![],);
	})
}

// Verified tests that were scheduled in a past slot will be moved into MissQueue
// Tasks in current time slot will be process as many as possible up to the max
// weight
// In this test, we purposely set the weight so it won't process the miss tasks,
// just make sure the missed slot's tasks are moved into missed queue
#[test]
fn trigger_tasks_handles_missed_slots() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: vec![40] }.into();

		add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::DynamicDispatch { encoded_call: call.encode() },
			vec![],
		);

		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		let missed_task_id =
			schedule_task(ALICE, vec![SCHEDULED_TIME - SLOT_SIZE_SECONDS], vec![50]);
		let missed_task = MissedTaskV2Of::<Test>::new(
			AccountId32::new(ALICE),
			missed_task_id,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS,
		);

		let remark_message = vec![50];
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: remark_message.clone() }.into();
		let task_will_be_run_id =
			schedule_dynamic_dispatch_task(ALICE, vec![SCHEDULED_TIME], call.clone());
		let scheduled_task_id = schedule_task(ALICE, vec![SCHEDULED_TIME], vec![50]);

		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 2,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 2,
		));
		System::reset_events();

		// Give this enough weight limit to run and process miss queue and generate miss event
		AutomationTime::trigger_tasks(Weight::from_parts(900_000 * 2 + 40_000, 0));

		// the first 2 tasks are missed
		assert_eq!(AutomationTime::get_missed_queue().len(), 2);
		assert_eq!(AutomationTime::get_missed_queue()[1], missed_task);

		// the  final one is in current schedule will be move into the task queue
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_task_queue()[0].1, scheduled_task_id);

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), SCHEDULED_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			vec![
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_will_be_run_id.clone(),
					condition,
					encoded_call: Some(call.encode()),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: AccountId32::new(ALICE),
					hash: BlakeTwo256::hash(&remark_message),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner,
					task_id: task_will_be_run_id.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: AccountId32::new(ALICE),
					task_id: task_will_be_run_id,
				}),
			],
		);
	})
}

// Verify logic of handling missing tasks as below:
//   - task in current slot always got process first,
//   - past time schedule is retain and will eventually be moved into MissedQueueV2
//     from there, we generate a TaskMissed event, then the task is completely
//     removed from the queue
//   - existing tasks in the queue (from previous run) will also be moved to
//     MissedQueueV2, and yield a task miss event
//   - we don't backfill or run old tasks.
//
// The execution of task missed event generation is lower priority, tasks in the
// time slot got run first, if there is enough weight left, only then we run
// the task miss event and doing house cleanup
#[test]
fn trigger_tasks_limits_missed_slots() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: vec![50] }.into();

		let missing_task_id0 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::DynamicDispatch { encoded_call: call.encode() },
			vec![],
		);

		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		Timestamp::set_timestamp((SCHEDULED_TIME - 25200) * 1_000);
		let missing_task_id1 =
			schedule_task(ALICE, vec![SCHEDULED_TIME - SLOT_SIZE_SECONDS], vec![50]);

		let missing_task_id2 =
			schedule_task(ALICE, vec![SCHEDULED_TIME - SLOT_SIZE_SECONDS * 2], vec![50]);
		let missing_task_id3 =
			schedule_task(ALICE, vec![SCHEDULED_TIME - SLOT_SIZE_SECONDS * 3], vec![50]);
		let missing_task_id4 =
			schedule_task(ALICE, vec![SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4], vec![50]);
		let missing_task_id5 =
			schedule_task(ALICE, vec![SCHEDULED_TIME - SLOT_SIZE_SECONDS * 5], vec![50]);

		let remark_message = vec![50];
		let call: <Test as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: remark_message.clone() }.into();
		let task_id = schedule_dynamic_dispatch_task(ALICE, vec![SCHEDULED_TIME], call.clone());

		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 7,
			SCHEDULED_TIME - SLOT_SIZE_SECONDS * 7,
		));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(9_769_423 + 200_000, 0));

		let my_events = events();

		let owner = AccountId32::new(ALICE);

		if let Some((updated_last_time_slot, updated_last_missed_slot)) =
			AutomationTime::get_last_slot()
		{
			assert_eq!(updated_last_time_slot, SCHEDULED_TIME);

			// This line ensures when given a total weight of 9_769_423 + 200_000, missing_task_id5, missing_task_id4, missing_task_id3 and missing_task_id2 will be discarded from the missed_queue in the current version of code.
			// TODO: we should examine the tasks in missed_queue instead of examing the timestamp of missing_task_id2
			assert_eq!(updated_last_missed_slot, SCHEDULED_TIME - SLOT_SIZE_SECONDS * 2);

			let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
			condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
			condition
				.insert("timestamp".as_bytes().to_vec(), SCHEDULED_TIME.to_string().into_bytes());

			assert_eq!(
				my_events,
				[
					// The execution of encoded call task
					RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
						who: owner.clone(),
						task_id: task_id.clone(),
						condition: condition.clone(),
						encoded_call: Some(call.encode()),
					}),
					RuntimeEvent::System(frame_system::pallet::Event::Remarked {
						sender: owner.clone(),
						hash: BlakeTwo256::hash(&remark_message),
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
						who: owner.clone(),
						task_id: task_id.clone(),
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
						who: owner.clone(),
						task_id,
					}),
					// The task 0 missed
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: owner.clone(),
						task_id: missing_task_id0.clone(),
						execution_time: SCHEDULED_TIME - SLOT_SIZE_SECONDS * 7,
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
						who: owner.clone(),
						task_id: missing_task_id0,
					}),
					// The task 5 missed
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: owner.clone(),
						task_id: missing_task_id5.clone(),
						execution_time: SCHEDULED_TIME - SLOT_SIZE_SECONDS * 5,
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
						who: owner.clone(),
						task_id: missing_task_id5,
					}),
					// The task 4 missed
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: owner.clone(),
						task_id: missing_task_id4.clone(),
						execution_time: SCHEDULED_TIME - SLOT_SIZE_SECONDS * 4,
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
						who: owner.clone(),
						task_id: missing_task_id4,
					}),
					// The task 3 missed
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: owner.clone(),
						task_id: missing_task_id3.clone(),
						execution_time: SCHEDULED_TIME - SLOT_SIZE_SECONDS * 3,
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
						who: owner.clone(),
						task_id: missing_task_id3,
					}),
					// The task 2 missed
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: owner.clone(),
						task_id: missing_task_id2.clone(),
						execution_time: SCHEDULED_TIME - SLOT_SIZE_SECONDS * 2,
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
						who: owner,
						task_id: missing_task_id2,
					}),
				]
			);
		} else {
			panic!("trigger_tasks_limits_missed_slots test did not have LastTimeSlot updated")
		}

		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME - SLOT_SIZE_SECONDS) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: account_task_ids, .. }) => {
				assert_eq!(account_task_ids.len(), 1);
				assert_eq!(account_task_ids[0].1, missing_task_id1);
			},
		}
	})
}

#[test]
fn trigger_tasks_completes_all_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let message_one: Vec<u8> = vec![2, 4, 5];
		let owner = AccountId32::new(ALICE);
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_one.clone()),
			vec![],
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_two.clone()),
			vec![],
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_parts(20_000_000, 0));

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id1.clone(),
					condition: condition.clone(),
					encoded_call: Some(vec![0, 7, 12, 2, 4, 5]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&message_one),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id1.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: task_id1.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id2.clone(),
					condition,
					encoded_call: Some(vec![0, 7, 8, 2, 4]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&message_two),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id2.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner,
					task_id: task_id2.clone(),
				}),
			]
		);
		assert_eq!(0, AutomationTime::get_task_queue().len());
		assert_eq!(AutomationTime::get_account_task(AccountId32::new(ALICE), task_id1), None);
		assert_eq!(AutomationTime::get_account_task(AccountId32::new(ALICE), task_id2), None);
	})
}

#[test]
fn trigger_tasks_handles_nonexisting_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let bad_task_id = vec![1, 2, 3];
		let mut task_queue = AutomationTime::get_task_queue();
		task_queue.push((owner.clone(), bad_task_id.clone()));
		TaskQueueV2::<Test>::put(task_queue);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_parts(90_000, 0));

		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::TaskNotFound {
				who: owner,
				task_id: bad_task_id
			}),]
		);
		assert_eq!(0, AutomationTime::get_task_queue().len());
	})
}

#[test]
fn trigger_tasks_completes_some_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let message_one: Vec<u8> = vec![2, 4, 5];
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_one.clone()),
			vec![],
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_two),
			vec![],
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_parts(80_000, 0));

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id1.clone(),
					condition,
					encoded_call: Some(vec![0, 7, 12, 2, 4, 5]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&message_one),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id1.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: task_id1.clone(),
				}),
			]
		);

		assert_eq!(1, AutomationTime::get_task_queue().len());
		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id1), None);
		assert_ne!(AutomationTime::get_account_task(owner, task_id2), None);
	})
}

#[test]
fn trigger_tasks_completes_all_missed_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id1 = add_task_to_missed_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(vec![40]),
			vec![],
		);
		let task_id2 = add_task_to_missed_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(vec![40]),
			vec![],
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_parts(130_000, 0));

		let owner = AccountId32::new(ALICE);

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: owner.clone(),
					task_id: task_id1.clone(),
					execution_time: SCHEDULED_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: task_id1.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: owner.clone(),
					task_id: task_id2.clone(),
					execution_time: SCHEDULED_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner,
					task_id: task_id2.clone(),
				}),
			]
		);

		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		assert_eq!(AutomationTime::get_account_task(AccountId32::new(ALICE), task_id1), None);
		assert_eq!(AutomationTime::get_account_task(AccountId32::new(ALICE), task_id2), None);
	})
}

#[test]
fn missed_tasks_updates_executions_left() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let task_id1 = add_task_to_missed_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME, SCHEDULED_TIME + SLOT_SIZE_SECONDS],
			create_dynamic_dispatch_remark_action(vec![40]),
			vec![],
		);
		let task_id2 = add_task_to_missed_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME, SCHEDULED_TIME + SLOT_SIZE_SECONDS],
			create_dynamic_dispatch_remark_action(vec![40]),
			vec![],
		);

		match AutomationTime::get_account_task(owner.clone(), task_id1.clone()) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}
		match AutomationTime::get_account_task(owner.clone(), task_id2.clone()) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		AutomationTime::trigger_tasks(Weight::from_parts(130_000, 0));

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: task_id1.clone(),
					execution_time: SCHEDULED_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: task_id2.clone(),
					execution_time: SCHEDULED_TIME
				}),
			]
		);

		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		match AutomationTime::get_account_task(owner.clone(), task_id1) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 1);
			},
		}
		match AutomationTime::get_account_task(owner, task_id2) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 1);
			},
		}
	})
}

#[test]
fn missed_tasks_removes_completed_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let message_one: Vec<u8> = vec![2, 5, 7];
		let owner = AccountId32::new(ALICE);
		let task_id01 = add_task_to_missed_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME, SCHEDULED_TIME - SLOT_SIZE_SECONDS],
			create_dynamic_dispatch_remark_action(message_one.clone()),
			vec![],
		);

		let mut task_queue = AutomationTime::get_task_queue();
		task_queue.push((owner.clone(), task_id01.clone()));
		TaskQueueV2::<Test>::put(task_queue);

		assert_eq!(AutomationTime::get_missed_queue().len(), 1);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		match AutomationTime::get_account_task(owner.clone(), task_id01.clone()) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();
		AutomationTime::trigger_tasks(Weight::from_parts(20_000_000, 0));

		assert_eq!(AutomationTime::get_task_queue().len(), 0);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id01.clone(),
					condition: condition.clone(),
					encoded_call: Some(vec![0, 7, 12, 2, 5, 7]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&message_one),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id01.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: owner.clone(),
					task_id: task_id01.clone(),
					execution_time: SCHEDULED_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: task_id01.clone(),
				}),
			]
		);
		assert_eq!(AutomationTime::get_account_task(owner, task_id01), None);
	})
}

#[test]
fn trigger_tasks_completes_some_xcmp_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
		let encoded_call = vec![3, 4, 5];
		let owner = AccountId32::new(ALICE);
		let task_id = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::XCMP {
				destination,
				schedule_fee: NATIVE_LOCATION,
				execution_fee: AssetPayment {
					asset_location: MultiLocation::new(0, Here).into(),
					amount: 10,
				},
				encoded_call: encoded_call.clone(),
				encoded_call_weight: Weight::from_parts(100_000, 0),
				overall_weight: Weight::from_parts(200_000, 0),
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
			},
			vec![],
		);

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id.clone(),
					condition,
					encoded_call: Some(encoded_call),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted { who: owner, task_id })
			]
		);
	})
}

#[test]
fn trigger_tasks_completes_auto_compound_delegated_stake_task() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let delegator = AccountId32::new(DELEGATOR_ACCOUNT);
		get_funds(delegator.clone());
		let before_balance = Balances::free_balance(delegator.clone());
		let account_minimum = before_balance / 2;
		let task_id = vec![1];

		let action = Action::AutoCompoundDelegatedStake {
			delegator: delegator.clone(),
			collator: AccountId32::new(COLLATOR_ACCOUNT),
			account_minimum,
		};

		add_recurring_task_to_task_queue(
			DELEGATOR_ACCOUNT,
			task_id.clone(),
			SCHEDULED_TIME,
			SLOT_SIZE_SECONDS,
			action.clone(),
			vec![],
		);

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));
		let emitted_events = events();

		let new_balance = Balances::free_balance(delegator.clone());
		assert!(new_balance < before_balance);
		assert_eq!(new_balance, account_minimum);

		let fee_amount = AutomationTime::calculate_schedule_fee_amount(&action, 1)
			.expect(EXPECT_CALCULATE_SCHEDULE_FEE_AMOUNT);
		let delegation = before_balance - fee_amount - account_minimum;

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert!(contains_events(
			emitted_events,
			vec![
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: delegator.clone(),
					task_id: task_id.clone(),
					condition,
					encoded_call: None,
				}),
				RuntimeEvent::ParachainStaking(
					pallet_parachain_staking::Event::DelegationIncreased {
						delegator: AccountId::from(DELEGATOR_ACCOUNT),
						candidate: AccountId::from(COLLATOR_ACCOUNT),
						amount: delegation,
						in_top: true,
					}
				),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: delegator.clone(),
					task_id: task_id.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskRescheduled {
					who: delegator,
					task_id,
					schedule_as: None,
				})
			]
		));
	})
}

// Scheduling an auto compound delegated stake task will succeed if the account has a sufficient balance and an delegation with the specificed collator.
// Condition:
// 1. User's wallet balance >= minimum balance + execution fee
// 2. User has a delegation with the specificed collator
// Expected result:
// 1. Current execution will run
// 2. Next execution will be scheduled
// 3. The task will re-run in the next execution time
#[test]
fn auto_compound_delegated_stake_enough_balance_has_delegation() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let delegator = AccountId32::new(DELEGATOR_ACCOUNT);
		get_funds(delegator.clone());
		let before_balance = Balances::free_balance(delegator.clone());
		// Minimum balance is half of the user's wallet balance
		let account_minimum = before_balance / 2;
		let frequency = SLOT_SIZE_SECONDS;

		let task_id = add_recurring_task_to_task_queue(
			DELEGATOR_ACCOUNT,
			vec![1],
			SCHEDULED_TIME,
			frequency,
			Action::AutoCompoundDelegatedStake {
				delegator: delegator.clone(),
				collator: AccountId32::new(COLLATOR_ACCOUNT),
				account_minimum,
			},
			vec![],
		);

		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));

		let emitted_events = events();

		// Expected result:
		// 1. Current execution will run
		emitted_events
			.clone()
			.into_iter()
			.find(|e| matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskExecuted { .. })))
			.expect("TaskExecuted event should have been emitted");

		// 2. Next execution will be scheduled
		emitted_events
			.into_iter()
			.find(|e| {
				matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskRescheduled { .. }))
			})
			.expect("TaskRescheduled event should have been emitted");

		let next_scheduled_time = SCHEDULED_TIME + frequency;
		AutomationTime::get_scheduled_tasks(next_scheduled_time)
			.expect("Task should have been rescheduled")
			.tasks
			.into_iter()
			.find(|t| *t == (delegator.clone(), task_id.clone()))
			.expect("Task should have been rescheduled");
		let task = AutomationTime::get_account_task(delegator.clone(), task_id)
			.expect("Task should not have been removed from task map");
		assert_eq!(task.schedule.known_executions_left(), 1);
		assert_eq!(task.execution_times(), vec![next_scheduled_time]);

		// 3. The task will re-run in the next execution time
		Timestamp::set_timestamp(next_scheduled_time * 1_000);
		get_funds(delegator);
		System::reset_events();
		AutomationTime::trigger_tasks(Weight::from_parts(100_000_000_000, 0));

		events()
			.into_iter()
			.find(|e| matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskTriggered { .. })))
			.expect("TaskTriggered event should have been emitted again!");
	})
}

// If the account has not enough balance and a delegation with the specificed collator,
// the next execution of the task will be re-scheduled because the InsufficientBalance error is not an abort error
// Condition:
// 1. User's wallet balance < minimum balance + execution fee
// 2. User has a delegation with the specificed collator
// Expected result:
// 1. The current execution will result in failure, triggering the emission of an TaskExecutionFailed event, error: DelegationDNE
// 2. The next execution will be scheduled because the InsufficientBalance error is not an abort error
#[test]
fn auto_compound_delegated_stake_not_enough_balance_has_delegation() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let delegator = AccountId32::new(DELEGATOR_ACCOUNT);
		get_funds(delegator.clone());
		let before_balance = Balances::free_balance(delegator.clone());
		// Minimum balance is twice of the user's wallet balance
		let account_minimum = before_balance * 2;
		let frequency = SLOT_SIZE_SECONDS;

		let task_id = add_recurring_task_to_task_queue(
			DELEGATOR_ACCOUNT,
			vec![1],
			SCHEDULED_TIME,
			frequency,
			Action::AutoCompoundDelegatedStake {
				delegator: delegator.clone(),
				collator: AccountId32::new(COLLATOR_ACCOUNT),
				account_minimum,
			},
			vec![],
		);

		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));

		let emitted_events = events();

		// Expected result:
		// 1. Emit TaskExecutionFailed event with error: InsufficientBalance
		let insufficient_balance_error: DispatchError = Error::<Test>::InsufficientBalance.into();
		emitted_events.clone().into_iter()
			.find(|e| matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskExecutionFailed { error, .. }) if *error == insufficient_balance_error )).expect("TaskExecutionFailed event should have been emitted");

		// The next execution will be scheduled because the InsufficientBalance error is not an abort error
		emitted_events
			.into_iter()
			.find(|e| matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskRescheduled { .. })))
			.expect("TaskRescheduled event should have been emitted");

		let next_scheduled_time = SCHEDULED_TIME + frequency;
		AutomationTime::get_scheduled_tasks(next_scheduled_time)
			.expect("Task should have been rescheduled")
			.tasks
			.into_iter()
			.find(|t| *t == (delegator.clone(), task_id.clone()))
			.expect("Task should have been rescheduled");
		let task = AutomationTime::get_account_task(delegator, task_id)
			.expect("Task should not have been removed from task map");
		assert_eq!(task.schedule.known_executions_left(), 1);
		assert_eq!(task.execution_times(), vec![next_scheduled_time]);
	})
}

// If the account has a sufficient balance and no delegator with the specificed collator,
// the next execution of the task will not be re-scheduled because the DelegatorDNE error is an abort error.
// Condition:
// 1. User's wallet balance >= minimum balance + execution fee
// 2. User has no delegator
// Expected result:
// 1. The current execution will result in failure, triggering the emission of an TaskExecutionFailed event, error: DelegatorDNE
// 2. The next execution will be scheduled because the DelegatorDNE error is not an abort error
#[test]
fn auto_compound_delegated_stake_enough_balance_no_delegator() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let delegator = AccountId32::new(ALICE);
		get_funds(delegator.clone());
		let before_balance = Balances::free_balance(delegator.clone());
		// Minimum balance is half of the user's wallet balance
		let account_minimum = before_balance / 2;
		let frequency = SLOT_SIZE_SECONDS;

		let task_id = add_recurring_task_to_task_queue(
			ALICE,
			vec![1],
			SCHEDULED_TIME,
			frequency,
			Action::AutoCompoundDelegatedStake {
				delegator: delegator.clone(),
				collator: AccountId32::new(BOB),
				account_minimum,
			},
			vec!["DelegatorDNE".as_bytes().to_vec(), "DelegationDNE".as_bytes().to_vec()],
		);

		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));

		let emitted_events = events();

		// Expected result:
		// 1. The current execution will result in failure, triggering the emission of an TaskExecutionFailed event, error: DelegationDNE
		let delegator_error: DispatchError =
			<pallet_parachain_staking::Error<Test>>::DelegatorDNE.into();
		emitted_events
			.clone()
			.into_iter()
			.find(|e| {
				matches!(e,
					RuntimeEvent::AutomationTime(crate::Event::TaskExecutionFailed {
						error,
						..
					}) if *error == delegator_error)
			})
			.expect("TaskExecutionFailed event should have been emitted");

		// 2. The next execution will be scheduled because the DelegatorDNE error is not an abort error
		emitted_events
			.into_iter()
			.find(|e| {
				matches!(e,
					RuntimeEvent::AutomationTime(crate::Event::TaskNotRescheduled {
						error,
						..
					}) if *error == delegator_error)
			})
			.expect("TaskNotRescheduled event should have been emitted");

		assert!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + frequency)
			.filter(|scheduled| {
				scheduled.tasks.iter().any(|t| *t == (delegator.clone(), task_id.clone()))
			})
			.is_none());
		assert!(AutomationTime::get_account_task(delegator, task_id).is_none());
	})
}

// If the account has a sufficient balance and no delegator with the specificed collator,
// the next execution of the task will not be re-scheduled because the DelegationDNE error is an abort error.
// Condition:
// 1. User's wallet balance >= minimum balance + execution fee
// 2. User has no delegation with the specificed collator
// Expected result:
// 1. The current execution will result in failure, triggering the emission of an TaskExecutionFailed event, error: DelegationDNE
// 2. The next execution will be scheduled because the DelegationDNE error is not an abort error
#[test]
fn auto_compound_delegated_stake_enough_balance_no_delegation() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let delegator = AccountId32::new(DELEGATOR_ACCOUNT);
		get_funds(delegator.clone());
		let before_balance = Balances::free_balance(delegator.clone());
		// Minimum balance is half of the user's wallet balance
		let account_minimum = before_balance / 2;
		let frequency = SLOT_SIZE_SECONDS;

		let task_id = add_recurring_task_to_task_queue(
			DELEGATOR_ACCOUNT,
			vec![1],
			SCHEDULED_TIME,
			frequency,
			Action::AutoCompoundDelegatedStake {
				delegator: delegator.clone(),
				collator: AccountId32::new(BOB),
				account_minimum,
			},
			vec!["DelegatorDNE".as_bytes().to_vec(), "DelegationDNE".as_bytes().to_vec()],
		);

		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));

		let emitted_events = events();

		// Expected result:
		// 1. The current execution will result in failure, triggering the emission of an TaskExecutionFailed event, error: DelegationDNE
		let delegation_error: DispatchError =
			<pallet_parachain_staking::Error<Test>>::DelegationDNE.into();
		emitted_events
			.clone()
			.into_iter()
			.find(|e| {
				matches!(e,
					RuntimeEvent::AutomationTime(crate::Event::TaskExecutionFailed {
						error,
						..
					}) if *error == delegation_error)
			})
			.expect("TaskExecutionFailed event should have been emitted");

		// 2. The next execution will be scheduled because the DelegationDNE error is not an abort error
		emitted_events
			.into_iter()
			.find(|e| {
				matches!(e,
					RuntimeEvent::AutomationTime(crate::Event::TaskNotRescheduled {
						error,
						..
					}) if *error == delegation_error)
			})
			.expect("TaskNotRescheduled event should have been emitted");

		assert!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + frequency)
			.filter(|scheduled| {
				scheduled.tasks.iter().any(|t| *t == (delegator.clone(), task_id.clone()))
			})
			.is_none());
		assert!(AutomationTime::get_account_task(delegator, task_id).is_none());
	})
}

// If the account has a sufficient balance and no delegator with the specificed collator,
// the next execution of the task will not be re-scheduled because the InsufficientBalance error is an abort error.
// Condition:
// 1. User's wallet balance < minimum balance + execution fee
// 2. User has no delegation with the specificed collator
// Expected result:
// 1. Emit TaskExecutionFailed event with error: InsufficientBalance
// 2. The next execution will be scheduled because the InsufficientBalance error is not an abort error
#[test]
fn auto_compound_delegated_stake_not_enough_balance_no_delegation() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let delegator = AccountId32::new(DELEGATOR_ACCOUNT);
		let collator = AccountId32::new(BOB);
		get_funds(delegator.clone());
		let before_balance = Balances::free_balance(delegator.clone());
		// Minimum balance is twice of the user's wallet balance
		let account_minimum = before_balance * 2;
		let frequency = SLOT_SIZE_SECONDS;

		let task_id = add_recurring_task_to_task_queue(
			DELEGATOR_ACCOUNT,
			vec![1],
			SCHEDULED_TIME,
			frequency,
			Action::AutoCompoundDelegatedStake {
				delegator: delegator.clone(),
				collator,
				account_minimum,
			},
			vec!["DelegatorDNE".as_bytes().to_vec(), "DelegationDNE".as_bytes().to_vec()],
		);

		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));
		let emitted_events = events();

		// Expected result:
		//  1. The current execution will result in failure, triggering the emission of an TaskExecutionFailed event, error: InsufficientBalance
		let insufficient_balance_error: DispatchError = Error::<Test>::InsufficientBalance.into();
		emitted_events
			.clone()
			.into_iter()
			.find(|e| {
				matches!(e,
					RuntimeEvent::AutomationTime(crate::Event::TaskExecutionFailed {
						error,
						..
					}) if *error == insufficient_balance_error)
			})
			.expect("TaskExecutionFailed event should have been emitted");

		// 2. The next execution will be scheduled because the InsufficientBalance error is not an abort error
		emitted_events
			.into_iter()
			.find(|e| {
				matches!(e, RuntimeEvent::AutomationTime(crate::Event::TaskRescheduled { .. }))
			})
			.expect("TaskRescheduled event should have been emitted");

		let next_scheduled_time = SCHEDULED_TIME + frequency;
		AutomationTime::get_scheduled_tasks(next_scheduled_time)
			.expect("Task should have been rescheduled")
			.tasks
			.into_iter()
			.find(|t| *t == (delegator.clone(), task_id.clone()))
			.expect("Task should have been rescheduled");
		let task = AutomationTime::get_account_task(delegator, task_id)
			.expect("Task should not have been removed from task map");
		assert_eq!(task.schedule.known_executions_left(), 1);
		assert_eq!(task.execution_times(), vec![next_scheduled_time]);
	})
}

// verify that execution left of a Fixed scheduled task will be decreased by
// one upon a succesful run.
#[test]
fn trigger_tasks_updates_executions_left() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let message_one: Vec<u8> = vec![2, 5, 7];
		let owner = AccountId32::new(ALICE);
		let task_id01 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME, SCHEDULED_TIME + SLOT_SIZE_SECONDS],
			create_dynamic_dispatch_remark_action(message_one.clone()),
			vec![],
		);

		match AutomationTime::get_account_task(owner.clone(), task_id01.clone()) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id01.clone(),
					condition,
					encoded_call: (Some(vec![0, 7, 12, 2, 5, 7])),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&message_one),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id01.clone(),
				}),
			]
		);
		match AutomationTime::get_account_task(owner, task_id01) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 1);
			},
		}
	})
}

#[test]
fn trigger_tasks_removes_completed_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let message_one: Vec<u8> = vec![2, 5, 7];
		let owner = AccountId32::new(ALICE);
		let task_id01 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_one.clone()),
			vec![],
		);

		match AutomationTime::get_account_task(owner.clone(), task_id01.clone()) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 1);
			},
		}

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_parts(120_000, 0));

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id01.clone(),
					condition,
					encoded_call: Some(vec![0, 7, 12, 2, 5, 7]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&message_one),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id01.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: task_id01.clone(),
				}),
			]
		);
		assert_eq!(AutomationTime::get_account_task(owner, task_id01), None);
	})
}

#[test]
fn on_init_runs_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let message_one: Vec<u8> = vec![2, 4, 5];
		let owner = AccountId32::new(ALICE);
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_one.clone()),
			vec![],
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_two.clone()),
			vec![],
		);
		let task_id3 = add_task_to_task_queue(
			ALICE,
			vec![60],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(vec![50]),
			vec![],
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::on_initialize(1);

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			[
				// The execution of task 1
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id1.clone(),
					condition: condition.clone(),
					encoded_call: Some(vec![0, 7, 12, 2, 4, 5]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&message_one),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id1.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: task_id1.clone(),
				}),
				// The execution of task 2
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: task_id2.clone(),
					condition,
					encoded_call: Some(vec![0, 7, 8, 2, 4]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(&message_two),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: task_id2.clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: task_id2.clone(),
				}),
			]
		);
		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id1), None);
		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id2), None);
		assert_ne!(AutomationTime::get_account_task(owner.clone(), task_id3.clone()), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		Timestamp::set_timestamp(START_BLOCK_TIME + (SLOT_SIZE_SECONDS * 1_000));
		AutomationTime::on_initialize(2);
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: task_id3.clone(),
					execution_time: LAST_BLOCK_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: task_id3.clone(),
				}),
			],
		);
		assert_eq!(AutomationTime::get_account_task(owner, task_id3), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 0);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
	})
}

// When our blockchain boot and initialize, it will start trigger and run tasks up to
// a MaxWeightPercentage of the MaxBlockWeight
//
#[test]
fn on_init_check_task_queue() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME - SLOT_SIZE_SECONDS * 2));
		let mut tasks = vec![];

		for i in 0..5 {
			let task_id = add_task_to_task_queue(
				ALICE,
				vec![i],
				vec![SCHEDULED_TIME],
				create_dynamic_dispatch_remark_action(vec![i]),
				vec![],
			);
			tasks.push(task_id.clone());
		}
		Timestamp::set_timestamp(START_BLOCK_TIME + (10 * 1000));
		AutomationTime::on_initialize(1);

		let owner = AccountId32::new(ALICE);

		let mut condition: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		condition.insert("type".as_bytes().to_vec(), "time".as_bytes().to_vec());
		condition.insert("timestamp".as_bytes().to_vec(), LAST_BLOCK_TIME.to_string().into_bytes());

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: tasks[0].clone(),
					condition: condition.clone(),
					encoded_call: Some(vec![0, 7, 4, 0]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(vec![0].as_slice()),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: tasks[0].clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: tasks[0].clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: tasks[1].clone(),
					condition: condition.clone(),
					encoded_call: Some(vec![0, 7, 4, 1]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(vec![1].as_slice()),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: tasks[1].clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: tasks[1].clone(),
				}),
			],
		);
		assert_eq!(AutomationTime::get_task_queue().len(), 3);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		Timestamp::set_timestamp(START_BLOCK_TIME + (40 * 1000));
		AutomationTime::on_initialize(2);
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: tasks[2].clone(),
					condition: condition.clone(),
					encoded_call: Some(vec![0, 7, 4, 2]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(vec![2].as_slice()),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: tasks[2].clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: tasks[2].clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskTriggered {
					who: owner.clone(),
					task_id: tasks[3].clone(),
					condition: condition.clone(),
					encoded_call: Some(vec![0, 7, 4, 3]),
				}),
				RuntimeEvent::System(frame_system::pallet::Event::Remarked {
					sender: owner.clone(),
					hash: BlakeTwo256::hash(vec![3].as_slice()),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskExecuted {
					who: owner.clone(),
					task_id: tasks[3].clone(),
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner.clone(),
					task_id: tasks[3].clone(),
				}),
			],
		);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		Timestamp::set_timestamp(START_BLOCK_TIME + (SLOT_SIZE_SECONDS * 1000));
		AutomationTime::on_initialize(3);
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: tasks[4].clone(),
					execution_time: LAST_BLOCK_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCompleted {
					who: owner,
					task_id: tasks[4].clone(),
				}),
			],
		);
		assert_eq!(AutomationTime::get_task_queue().len(), 0);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
	})
}

#[test]
fn on_init_shutdown() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		AutomationTime::shutdown();

		let message_one: Vec<u8> = vec![2, 4, 5];
		let owner = AccountId32::new(ALICE);
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_one),
			vec![],
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(message_two),
			vec![],
		);
		let task_id3 = add_task_to_task_queue(
			ALICE,
			vec![60],
			vec![SCHEDULED_TIME],
			create_dynamic_dispatch_remark_action(vec![50]),
			vec![],
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::on_initialize(1);
		assert_eq!(events(), []);
		Timestamp::set_timestamp(START_BLOCK_TIME + (SLOT_SIZE_SECONDS * 1_000));
		AutomationTime::on_initialize(2);
		assert_eq!(events(), [],);
		assert_ne!(AutomationTime::get_account_task(owner.clone(), task_id1), None);
		assert_ne!(AutomationTime::get_account_task(owner.clone(), task_id2), None);
		assert_ne!(AutomationTime::get_account_task(owner, task_id3), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 3);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
	})
}
