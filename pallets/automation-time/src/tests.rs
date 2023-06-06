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
	mock::*, AccountTasks, Action, Config, Error, LastTimeSlot, MissedTaskV2Of, ScheduleParam,
	ScheduledTasksOf, TaskHashInput, TaskOf, TaskQueueV2, WeightInfo,
};
use codec::Encode;
use core::convert::TryInto;
use frame_support::{assert_noop, assert_ok, traits::OnInitialize, weights::Weight};
use frame_system::RawOrigin;
use pallet_valve::Shutdown;
use sp_runtime::{
	traits::{BlakeTwo256, Hash},
	AccountId32,
};

use xcm::latest::{prelude::X1, Junction::Parachain, MultiLocation};

pub const START_BLOCK_TIME: u64 = 33198768000 * 1_000;
pub const SCHEDULED_TIME: u64 = START_BLOCK_TIME / 1_000 + 7200;
const LAST_BLOCK_TIME: u64 = START_BLOCK_TIME / 1_000;

#[test]
fn schedule_invalid_time() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![SCHEDULED_TIME + 1],
				vec![12]
			),
			Error::<Test>::InvalidTime,
		);
	})
}

#[test]
fn schedule_past_time() {
	new_test_ext(START_BLOCK_TIME + 1_000 * 10800).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![SCHEDULED_TIME],
				vec![12]
			),
			Error::<Test>::PastTime,
		);

		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![SCHEDULED_TIME - 3600],
				vec![12]
			),
			Error::<Test>::PastTime,
		);
	})
}

#[test]
fn schedule_too_far_out() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![SCHEDULED_TIME + 1 * 24 * 60 * 60],
				vec![12]
			),
			Error::<Test>::TimeTooFarOut,
		);
	})
}

#[test]
fn schedule_no_message() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![SCHEDULED_TIME],
				vec![]
			),
			Error::<Test>::EmptyMessage,
		);
	})
}

#[test]
fn schedule_no_provided_id() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![],
				vec![SCHEDULED_TIME],
				vec![12]
			),
			Error::<Test>::EmptyProvidedId,
		);
	})
}

#[test]
fn schedule_not_enough_for_fees() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![60],
				vec![SCHEDULED_TIME],
				vec![12]
			),
			Error::<Test>::InsufficientBalance,
		);
	})
}

#[test]
fn schedule_notify_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		let message: Vec<u8> = vec![2, 4, 5];
		assert_ok!(AutomationTime::schedule_notify_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			vec![50],
			vec![SCHEDULED_TIME],
			message.clone()
		));
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: account_task_ids, .. }) =>
				match AutomationTime::get_account_task(
					account_task_ids[0].0.clone(),
					account_task_ids[0].1,
				) {
					None => {
						panic!("A task should exist if it was scheduled")
					},
					Some(task) => {
						let expected_task = TaskOf::<Test>::create_event_task::<Test>(
							AccountId32::new(ALICE),
							vec![50],
							vec![SCHEDULED_TIME],
							message,
						)
						.unwrap();

						assert_eq!(task, expected_task);
					},
				},
		}
	})
}

#[test]
fn schedule_native_transfer_invalid_amount() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_native_transfer_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![SCHEDULED_TIME],
				AccountId32::new(BOB),
				0,
			),
			Error::<Test>::InvalidAmount,
		);
	})
}

#[test]
fn schedule_native_transfer_cannot_transfer_to_self() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_native_transfer_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![SCHEDULED_TIME],
				AccountId32::new(ALICE),
				1,
			),
			Error::<Test>::TransferToSelf,
		);
	})
}

#[test]
fn schedule_native_transfer_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		assert_ok!(AutomationTime::schedule_native_transfer_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			vec![50],
			vec![SCHEDULED_TIME],
			AccountId32::new(BOB),
			1,
		));
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: account_task_ids, .. }) =>
				match AutomationTime::get_account_task(
					account_task_ids[0].0.clone(),
					account_task_ids[0].1,
				) {
					None => {
						panic!("A task should exist if it was scheduled")
					},
					Some(task) => {
						let expected_task = TaskOf::<Test>::create_native_transfer_task::<Test>(
							AccountId32::new(ALICE),
							vec![50],
							vec![SCHEDULED_TIME],
							AccountId32::new(BOB),
							1,
						)
						.unwrap();

						assert_eq!(task, expected_task);
					},
				},
		}
	})
}

#[test]
fn schedule_xcmp_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let alice = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		// Funds including XCM fees
		get_xcmp_funds(alice.clone());

		assert_ok!(AutomationTime::schedule_xcmp_task(
			RuntimeOrigin::signed(alice.clone()),
			vec![50],
			ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
			PARA_ID.try_into().unwrap(),
			NATIVE,
			MultiLocation::new(1, X1(Parachain(PARA_ID.into()))).into(),
			call.clone(),
			Weight::from_ref_time(100_000),
		));
	})
}

#[test]
fn schedule_xcmp_fails_if_not_enough_funds() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let alice = AccountId32::new(ALICE);
		let call: Vec<u8> = vec![2, 4, 5];
		// Funds not including XCM fees
		get_minimum_funds(alice.clone(), 1);

		assert_noop!(
			AutomationTime::schedule_xcmp_task(
				RuntimeOrigin::signed(alice.clone()),
				vec![50],
				ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
				PARA_ID.try_into().unwrap(),
				NATIVE,
				MultiLocation::new(1, X1(Parachain(PARA_ID.into()))).into(),
				call.clone(),
				Weight::from_ref_time(100_000),
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
			3_600,
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
				AutomationTime::generate_auto_compound_delegated_stake_provided_id(&alice, &bob),
				SCHEDULED_TIME,
				3_600,
				bob,
				1_000_000_000,
			)
			.ok()
		);
	})
}

#[test]
fn schedule_auto_compound_with_bad_frequency() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_auto_compound_delegated_stake_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				SCHEDULED_TIME,
				4_000,
				AccountId32::new(BOB),
				100_000,
			),
			Error::<Test>::InvalidTime,
		);
		assert_noop!(
			AutomationTime::schedule_auto_compound_delegated_stake_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				SCHEDULED_TIME,
				0,
				AccountId32::new(BOB),
				100_000,
			),
			Error::<Test>::InvalidTime,
		);
	})
}

#[test]
fn schedule_auto_compound_with_high_frequency() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_auto_compound_delegated_stake_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				SCHEDULED_TIME,
				<Test as Config>::MaxScheduleSeconds::get() + 3_600,
				AccountId32::new(BOB),
				100_000,
			),
			Error::<Test>::TimeTooFarOut,
		);
	})
}

#[test]
fn schedule_duplicates_errors() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		assert_ok!(AutomationTime::schedule_notify_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			vec![50],
			vec![SCHEDULED_TIME],
			vec![2, 4, 5]
		),);
		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![SCHEDULED_TIME],
				vec![2, 4]
			),
			Error::<Test>::DuplicateTask,
		);
	})
}

#[test]
fn schedule_max_execution_times_errors() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![50],
				vec![
					SCHEDULED_TIME,
					SCHEDULED_TIME + 3600,
					SCHEDULED_TIME + 7200,
					SCHEDULED_TIME + 10800
				],
				vec![2, 4]
			),
			Error::<Test>::TooManyExecutionsTimes,
		);
	})
}

#[test]
fn schedule_execution_times_removes_dupes() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		get_funds(owner.clone());
		let task_id1 = schedule_task(
			ALICE,
			vec![50],
			vec![
				SCHEDULED_TIME,
				SCHEDULED_TIME,
				SCHEDULED_TIME,
				SCHEDULED_TIME,
				SCHEDULED_TIME + 10800,
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
					vec![50],
					vec![SCHEDULED_TIME, SCHEDULED_TIME + 10800],
					vec![2, 4],
				)
				.unwrap();

				assert_eq!(task, expected_task);
			},
		}
	})
}

#[test]
fn schedule_time_slot_full() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		assert_ok!(AutomationTime::schedule_notify_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			vec![50],
			vec![SCHEDULED_TIME],
			vec![2, 4]
		));
		assert_ok!(AutomationTime::schedule_notify_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			vec![60],
			vec![SCHEDULED_TIME],
			vec![2, 4, 5]
		));

		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![70],
				vec![SCHEDULED_TIME],
				vec![2]
			),
			Error::<Test>::TimeSlotFull,
		);
	})
}

#[test]
fn schedule_time_slot_full_rolls_back() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		let task_id1 = schedule_task(ALICE, vec![40], vec![SCHEDULED_TIME + 7200], vec![2, 4, 5]);
		let task_id2 = schedule_task(ALICE, vec![50], vec![SCHEDULED_TIME + 7200], vec![2, 4]);

		assert_noop!(
			AutomationTime::schedule_notify_task(
				RuntimeOrigin::signed(AccountId32::new(ALICE)),
				vec![70],
				vec![SCHEDULED_TIME, SCHEDULED_TIME + 3600, SCHEDULED_TIME + 7200],
				vec![2]
			),
			Error::<Test>::TimeSlotFull,
		);

		if let Some(_) = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
			panic!("Tasks scheduled for the time it should have been rolled back")
		}
		if let Some(_) = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + 3600) {
			panic!("Tasks scheduled for the time it should have been rolled back")
		}
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + 7200) {
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

#[test]
fn cancel_works_for_scheduled() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id1 = schedule_task(ALICE, vec![40], vec![SCHEDULED_TIME], vec![2, 4, 5]);
		let task_id2 = schedule_task(ALICE, vec![50], vec![SCHEDULED_TIME], vec![2, 4]);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME - 14400, SCHEDULED_TIME - 14400));
		System::reset_events();

		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id1,
		));
		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id2,
		));

		if let Some(_) = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
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

#[test]
fn cancel_works_for_multiple_executions_scheduled() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let task_id1 = schedule_task(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME, SCHEDULED_TIME + 3600, SCHEDULED_TIME + 7200],
			vec![2, 4, 5],
		);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME - 14400, SCHEDULED_TIME - 14400));
		System::reset_events();

		assert_ok!(AutomationTime::cancel_task(RuntimeOrigin::signed(owner.clone()), task_id1,));

		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id1), None);
		if let Some(_) = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
			panic!("Tasks scheduled for the time it should have been deleted")
		}
		if let Some(_) = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + 3600) {
			panic!("Tasks scheduled for the time it should have been deleted")
		}
		if let Some(_) = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + 7200) {
			panic!("Tasks scheduled for the time it should have been deleted")
		}
		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
				who: owner,
				task_id: task_id1
			})]
		);
	})
}

#[test]
fn cancel_works_for_an_executed_task() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let task_id1 =
			schedule_task(ALICE, vec![50], vec![SCHEDULED_TIME, SCHEDULED_TIME + 3600], vec![50]);
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME - 3600, SCHEDULED_TIME - 3600));
		System::reset_events();

		match AutomationTime::get_account_task(owner.clone(), task_id1) {
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
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + 3600) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: task_ids, .. }) => {
				assert_eq!(task_ids.len(), 1);
				assert_eq!(task_ids[0].1, task_id1);
			},
		}

		AutomationTime::trigger_tasks(Weight::from_ref_time(200_000));
		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::Notify { message: vec![50] }),]
		);
		match AutomationTime::get_account_task(owner.clone(), task_id1) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 1);
			},
		}

		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME), None);
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + 3600) {
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
			task_id1
		));

		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME), None);
		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + 3600), None);

		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id1), None);
		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::TaskCancelled {
				who: owner,
				task_id: task_id1,
			})]
		);
	})
}

#[test]
fn cancel_works_for_tasks_in_queue() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::Notify { message: vec![2, 4, 5] },
		);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME, SCHEDULED_TIME));

		assert_eq!(task_id, AutomationTime::get_task_queue()[0].1);
		assert_eq!(1, AutomationTime::get_task_queue().len());

		assert_ok!(AutomationTime::cancel_task(
			RuntimeOrigin::signed(AccountId32::new(ALICE)),
			task_id,
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

#[test]
fn cancel_task_must_exist() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task = TaskOf::<Test>::create_event_task::<Test>(
			AccountId32::new(ALICE),
			vec![40],
			vec![SCHEDULED_TIME],
			vec![2, 4, 5],
		)
		.unwrap();
		let task_id = BlakeTwo256::hash_of(&task);

		assert_noop!(
			AutomationTime::cancel_task(RuntimeOrigin::signed(AccountId32::new(ALICE)), task_id),
			Error::<Test>::TaskDoesNotExist,
		);
	})
}

#[test]
fn cancel_task_not_found() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner = AccountId32::new(ALICE);
		let task = TaskOf::<Test>::create_event_task::<Test>(
			owner.clone(),
			vec![40],
			vec![SCHEDULED_TIME],
			vec![2, 4, 5],
		)
		.unwrap();
		let task_id = BlakeTwo256::hash_of(&task);
		AccountTasks::<Test>::insert(owner.clone(), task_id, task);

		assert_ok!(AutomationTime::cancel_task(RuntimeOrigin::signed(owner.clone()), task_id,));
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskNotFound {
					who: owner.clone(),
					task_id
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskCancelled { who: owner, task_id })
			]
		);
	})
}

#[test]
fn force_cancel_task_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id = schedule_task(ALICE, vec![40], vec![SCHEDULED_TIME], vec![2, 4, 5]);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME - 14400, SCHEDULED_TIME - 14400));
		System::reset_events();

		assert_ok!(AutomationTime::force_cancel_task(
			RawOrigin::Root.into(),
			AccountId32::new(ALICE),
			task_id
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
				let provided_id = vec![0];
				let task_id =
					AutomationTime::generate_task_id(account_id.clone(), provided_id.clone());
				let call: RuntimeCall = frame_system::Call::remark { remark: vec![] }.into();
				assert_ok!(fund_account_dynamic_dispatch(
					&account_id,
					execution_times.len(),
					call.encode()
				));

				assert_ok!(AutomationTime::schedule_dynamic_dispatch_task(
					RuntimeOrigin::signed(account_id.clone()),
					provided_id,
					ScheduleParam::Fixed { execution_times: vec![SCHEDULED_TIME] },
					Box::new(call)
				));
				assert_eq!(
					last_event(),
					RuntimeEvent::AutomationTime(crate::Event::TaskScheduled {
						who: account_id,
						task_id,
						schedule_as: None,
					})
				);
			})
		}
	}
}

mod run_dynamic_dispatch_action {
	use super::*;
	use sp_runtime::DispatchError;

	#[test]
	fn cannot_decode() {
		new_test_ext(START_BLOCK_TIME).execute_with(|| {
			let account_id = AccountId32::new(ALICE);
			let task_id = AutomationTime::generate_task_id(account_id.clone(), vec![1]);
			let bad_encoded_call: Vec<u8> = vec![1];

			let (weight, _) = AutomationTime::run_dynamic_dispatch_action(
				account_id.clone(),
				bad_encoded_call,
				task_id,
			);

			assert_eq!(
				weight,
				<Test as Config>::WeightInfo::run_dynamic_dispatch_action_fail_decode()
			);
			assert_eq!(
				events(),
				[RuntimeEvent::AutomationTime(crate::Event::CallCannotBeDecoded {
					who: account_id,
					task_id,
				}),]
			);
		})
	}

	#[test]
	fn call_errors() {
		new_test_ext(START_BLOCK_TIME).execute_with(|| {
			let account_id = AccountId32::new(ALICE);
			let task_id = AutomationTime::generate_task_id(account_id.clone(), vec![1]);
			let call: RuntimeCall = frame_system::Call::set_code { code: vec![] }.into();
			let encoded_call = call.encode();

			AutomationTime::run_dynamic_dispatch_action(account_id.clone(), encoded_call, task_id);

			assert_eq!(
				events(),
				[RuntimeEvent::AutomationTime(crate::Event::DynamicDispatchResult {
					who: account_id,
					task_id,
					result: Err(DispatchError::BadOrigin),
				}),]
			);
		})
	}

	#[test]
	fn call_filtered() {
		new_test_ext(START_BLOCK_TIME).execute_with(|| {
			let account_id = AccountId32::new(ALICE);
			let task_id = AutomationTime::generate_task_id(account_id.clone(), vec![1]);
			let call: RuntimeCall = pallet_timestamp::Call::set { now: 100 }.into();
			let encoded_call = call.encode();

			AutomationTime::run_dynamic_dispatch_action(account_id.clone(), encoded_call, task_id);

			assert_eq!(
				events(),
				[RuntimeEvent::AutomationTime(crate::Event::DynamicDispatchResult {
					who: account_id,
					task_id,
					result: Err(DispatchError::from(frame_system::Error::<Test>::CallFiltered)),
				}),]
			);
		})
	}

	#[test]
	fn call_works() {
		new_test_ext(START_BLOCK_TIME).execute_with(|| {
			let account_id = AccountId32::new(ALICE);
			let task_id = AutomationTime::generate_task_id(account_id.clone(), vec![1]);
			let call: RuntimeCall = frame_system::Call::remark { remark: vec![] }.into();
			let encoded_call = call.encode();

			AutomationTime::run_dynamic_dispatch_action(account_id.clone(), encoded_call, task_id);

			assert_eq!(
				events(),
				[RuntimeEvent::AutomationTime(crate::Event::DynamicDispatchResult {
					who: account_id,
					task_id,
					result: Ok(()),
				}),]
			);
		})
	}
}

// Weights to use for tests below
// 20_000: run scheduled task (run_notify_task, run_native_transfer_task)
// 10_000v: run per missed task (run_missed_tasks_many_found)
// 10_000v: run per task not found in map (run_missed_tasks_many_missing, run_tasks_many_missing)
// 50_000v: weight check for running 1 more task, current static v=1 (run_tasks_many_found)
// 10_000: update task queue function overhead (update_task_queue_overhead)
// 20_000: update task queue for scheduled tasks (update_scheduled_task_queue)
// 20_000v: for each old time slot to missed tasks (append_to_missed_tasks)
// 20_000: to move a single time slot to missed tasks (shift_missed_tasks)

#[test]
fn trigger_tasks_handles_first_run() {
	new_test_ext(0).execute_with(|| {
		AutomationTime::trigger_tasks(Weight::from_ref_time(30_000));

		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_nothing_to_do() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_ref_time(30_000));

		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_updates_queues() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let missed_task_id = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME - 3600],
			Action::Notify { message: vec![40] },
		);
		let missed_task = MissedTaskV2Of::<Test>::new(
			AccountId32::new(ALICE),
			missed_task_id,
			SCHEDULED_TIME - 3600,
		);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		let scheduled_task_id = schedule_task(ALICE, vec![50], vec![SCHEDULED_TIME], vec![50]);
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME - 3600, SCHEDULED_TIME - 3600));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(50_000));

		assert_eq!(AutomationTime::get_missed_queue().len(), 1);
		assert_eq!(AutomationTime::get_missed_queue()[0], missed_task);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_task_queue()[0].1, scheduled_task_id);
		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME), None);
		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_handles_missed_slots() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::Notify { message: vec![40] },
		);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		let missed_task_id = schedule_task(ALICE, vec![50], vec![SCHEDULED_TIME - 3600], vec![50]);
		let missed_task = MissedTaskV2Of::<Test>::new(
			AccountId32::new(ALICE),
			missed_task_id,
			SCHEDULED_TIME - 3600,
		);
		let scheduled_task_id = schedule_task(ALICE, vec![60], vec![SCHEDULED_TIME], vec![50]);
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME - 7200, SCHEDULED_TIME - 7200));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(90_000));

		assert_eq!(AutomationTime::get_missed_queue().len(), 2);
		assert_eq!(AutomationTime::get_missed_queue()[1], missed_task);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_task_queue()[0].1, scheduled_task_id);
		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_limits_missed_slots() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let missing_task_id0 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::Notify { message: vec![40] },
		);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		Timestamp::set_timestamp((SCHEDULED_TIME - 25200) * 1_000);
		let missing_task_id1 =
			schedule_task(ALICE, vec![50], vec![SCHEDULED_TIME - 3600], vec![50]);
		let missing_task_id2 =
			schedule_task(ALICE, vec![60], vec![SCHEDULED_TIME - 7200], vec![50]);
		let missing_task_id3 =
			schedule_task(ALICE, vec![70], vec![SCHEDULED_TIME - 10800], vec![50]);
		let missing_task_id4 =
			schedule_task(ALICE, vec![80], vec![SCHEDULED_TIME - 14400], vec![50]);
		let missing_task_id5 =
			schedule_task(ALICE, vec![90], vec![SCHEDULED_TIME - 18000], vec![50]);
		schedule_task(ALICE, vec![100], vec![SCHEDULED_TIME], vec![50]);
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put((SCHEDULED_TIME - 25200, SCHEDULED_TIME - 25200));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(200_000));

		if let Some((updated_last_time_slot, updated_last_missed_slot)) =
			AutomationTime::get_last_slot()
		{
			assert_eq!(updated_last_time_slot, SCHEDULED_TIME);
			assert_eq!(updated_last_missed_slot, SCHEDULED_TIME - 10800);
			assert_eq!(
				events(),
				[
					RuntimeEvent::AutomationTime(crate::Event::Notify { message: vec![50] }),
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: AccountId32::new(ALICE),
						task_id: missing_task_id0,
						execution_time: SCHEDULED_TIME - 25200,
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: AccountId32::new(ALICE),
						task_id: missing_task_id5,
						execution_time: SCHEDULED_TIME - 18000,
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: AccountId32::new(ALICE),
						task_id: missing_task_id4,
						execution_time: SCHEDULED_TIME - 14400,
					}),
					RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
						who: AccountId32::new(ALICE),
						task_id: missing_task_id3,
						execution_time: SCHEDULED_TIME - 10800,
					}),
				]
			);
		} else {
			panic!("trigger_tasks_limits_missed_slots test did not have LastTimeSlot updated")
		}
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME - 7200) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(ScheduledTasksOf::<Test> { tasks: account_task_ids, .. }) => {
				assert_eq!(account_task_ids.len(), 1);
				assert_eq!(account_task_ids[0].1, missing_task_id2);
			},
		}
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME - 3600) {
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
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::Notify { message: message_one.clone() },
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			Action::Notify { message: message_two.clone() },
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::Notify { message: message_one.clone() }),
				RuntimeEvent::AutomationTime(crate::Event::Notify { message: message_two.clone() }),
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
		let task_hash_input = TaskHashInput::new(owner.clone(), vec![20]);
		let bad_task_id = BlakeTwo256::hash_of(&task_hash_input);
		let mut task_queue = AutomationTime::get_task_queue();
		task_queue.push((owner.clone(), bad_task_id));
		TaskQueueV2::<Test>::put(task_queue);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_ref_time(90_000));

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
		let message_one: Vec<u8> = vec![2, 4, 5];
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::Notify { message: message_one.clone() },
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			Action::Notify { message: message_two.clone() },
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_ref_time(80_000));

		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::Notify { message: message_one.clone() }),]
		);

		assert_eq!(1, AutomationTime::get_task_queue().len());
		assert_eq!(AutomationTime::get_account_task(AccountId32::new(ALICE), task_id1), None);
		assert_ne!(AutomationTime::get_account_task(AccountId32::new(ALICE), task_id2), None);
	})
}

#[test]
fn trigger_tasks_completes_all_missed_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id1 = add_task_to_missed_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::Notify { message: vec![40] },
		);
		let task_id2 = add_task_to_missed_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			Action::Notify { message: vec![40] },
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::trigger_tasks(Weight::from_ref_time(130_000));

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: task_id1,
					execution_time: SCHEDULED_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: task_id2,
					execution_time: SCHEDULED_TIME
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
			vec![SCHEDULED_TIME, SCHEDULED_TIME + 3600],
			Action::Notify { message: vec![40] },
		);
		let task_id2 = add_task_to_missed_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME, SCHEDULED_TIME + 3600],
			Action::Notify { message: vec![40] },
		);

		match AutomationTime::get_account_task(owner.clone(), task_id1) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}
		match AutomationTime::get_account_task(owner.clone(), task_id2) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		AutomationTime::trigger_tasks(Weight::from_ref_time(130_000));

		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: task_id1,
					execution_time: SCHEDULED_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: task_id2,
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
		match AutomationTime::get_account_task(owner.clone(), task_id2) {
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
			vec![SCHEDULED_TIME, SCHEDULED_TIME - 3600],
			Action::Notify { message: message_one.clone() },
		);

		let mut task_queue = AutomationTime::get_task_queue();
		task_queue.push((owner.clone(), task_id01));
		TaskQueueV2::<Test>::put(task_queue);

		assert_eq!(AutomationTime::get_missed_queue().len(), 1);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		match AutomationTime::get_account_task(owner.clone(), task_id01) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();
		AutomationTime::trigger_tasks(Weight::from_ref_time(130_000));

		assert_eq!(AutomationTime::get_task_queue().len(), 0);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::Notify { message: message_one }),
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: task_id01,
					execution_time: SCHEDULED_TIME
				}),
			]
		);
		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id01), None);
	})
}

#[test]
fn trigger_tasks_completes_some_native_transfer_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		let current_funds = Balances::free_balance(AccountId32::new(ALICE));
		let transfer_amount = 1;

		add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::NativeTransfer {
				sender: AccountId32::new(ALICE),
				recipient: AccountId32::new(BOB),
				amount: transfer_amount,
			},
		);
		add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			Action::NativeTransfer {
				sender: AccountId32::new(ALICE),
				recipient: AccountId32::new(BOB),
				amount: transfer_amount,
			},
		);

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		assert_eq!(
			Balances::free_balance(AccountId32::new(ALICE)),
			current_funds - (transfer_amount * 2)
		);
		assert_eq!(Balances::free_balance(AccountId32::new(BOB)), transfer_amount * 2);
	})
}

#[test]
fn trigger_tasks_completes_some_xcmp_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let para_id = PARA_ID.try_into().unwrap();
		let task_id = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME],
			Action::XCMP {
				para_id,
				currency_id: NATIVE,
				xcm_asset_location: MultiLocation::new(1, X1(Parachain(para_id.into()))).into(),
				encoded_call: vec![3, 4, 5],
				encoded_call_weight: Weight::from_ref_time(100_000),
				schedule_as: None,
			},
		);

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::XcmpTaskSucceeded {
				para_id: PARA_ID.try_into().unwrap(),
				task_id,
			})]
		);
	})
}

#[test]
fn trigger_tasks_completes_auto_compound_delegated_stake_task() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		let before_balance = Balances::free_balance(AccountId32::new(ALICE));
		let account_minimum = before_balance / 2;

		let task_id = add_recurring_task_to_task_queue(
			ALICE,
			vec![1],
			SCHEDULED_TIME,
			3600,
			Action::AutoCompoundDelegatedStake {
				delegator: AccountId32::new(ALICE),
				collator: AccountId32::new(BOB),
				account_minimum,
			},
		);

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		let new_balance = Balances::free_balance(AccountId32::new(ALICE));
		assert!(new_balance < before_balance);
		assert_eq!(new_balance, account_minimum);
		let delegation_event = events()
			.into_iter()
			.find(|e| match e {
				RuntimeEvent::AutomationTime(
					crate::Event::SuccesfullyAutoCompoundedDelegatorStake { .. },
				) => true,
				_ => false,
			})
			.expect("AutoCompound success event should have been emitted");
		let execution_weight = MockWeight::<Test>::run_auto_compound_delegated_stake_task();
		let fee = ExecutionWeightFee::get().saturating_mul(execution_weight.ref_time().into());
		assert_eq!(
			delegation_event,
			RuntimeEvent::AutomationTime(crate::Event::SuccesfullyAutoCompoundedDelegatorStake {
				task_id,
				amount: before_balance
					.checked_sub(account_minimum.saturating_add(fee))
					.expect("Event should not exist if value is neg"),
			})
		);
	})
}

#[test]
fn auto_compound_delegated_stake_reschedules_and_reruns() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		let before_balance = Balances::free_balance(AccountId32::new(ALICE));
		let account_minimum = before_balance / 2;
		let frequency = 3_600;

		let task_id = add_recurring_task_to_task_queue(
			ALICE,
			vec![1],
			SCHEDULED_TIME,
			frequency,
			Action::AutoCompoundDelegatedStake {
				delegator: AccountId32::new(ALICE),
				collator: AccountId32::new(BOB),
				account_minimum,
			},
		);

		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		events()
			.into_iter()
			.find(|e| match e {
				RuntimeEvent::AutomationTime(crate::Event::TaskScheduled { .. }) => true,
				_ => false,
			})
			.expect("TaskScheduled event should have been emitted");
		let next_scheduled_time = SCHEDULED_TIME + frequency;
		AutomationTime::get_scheduled_tasks(next_scheduled_time)
			.expect("Task should have been rescheduled")
			.tasks
			.into_iter()
			.find(|t| *t == (AccountId32::new(ALICE), task_id))
			.expect("Task should have been rescheduled");
		let task = AutomationTime::get_account_task(AccountId32::new(ALICE), task_id)
			.expect("Task should not have been removed from task map");
		assert_eq!(task.schedule.known_executions_left(), 1);
		assert_eq!(task.execution_times(), vec![next_scheduled_time]);

		Timestamp::set_timestamp(next_scheduled_time * 1_000);
		get_funds(AccountId32::new(ALICE));
		System::reset_events();
		AutomationTime::trigger_tasks(Weight::from_ref_time(100_000_000_000));

		events()
			.into_iter()
			.find(|e| match e {
				RuntimeEvent::AutomationTime(
					crate::Event::SuccesfullyAutoCompoundedDelegatorStake { .. },
				) => true,
				_ => false,
			})
			.expect("AutoCompound success event should have been emitted");
	})
}

#[test]
fn auto_compound_delegated_stake_without_minimum_balance() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		let balance = Balances::free_balance(AccountId32::new(ALICE));
		let account_minimum = balance * 2;

		add_task_to_task_queue(
			ALICE,
			vec![1],
			vec![SCHEDULED_TIME],
			Action::AutoCompoundDelegatedStake {
				delegator: AccountId32::new(ALICE),
				collator: AccountId32::new(BOB),
				account_minimum,
			},
		);

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		let new_balance = Balances::free_balance(AccountId32::new(ALICE));
		assert_eq!(new_balance, balance);
		events()
			.into_iter()
			.find(|e| match e {
				RuntimeEvent::AutomationTime(crate::Event::AutoCompoundDelegatorStakeFailed {
					..
				}) => true,
				_ => false,
			})
			.expect("AutoCompound failure event should have been emitted");
	})
}

#[test]
fn auto_compound_delegated_stake_does_not_reschedule_on_failure() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		get_funds(AccountId32::new(ALICE));
		let before_balance = Balances::free_balance(AccountId32::new(ALICE));
		let account_minimum = before_balance * 2;
		let frequency = 3_600;

		let task_id = add_recurring_task_to_task_queue(
			ALICE,
			vec![1],
			SCHEDULED_TIME,
			frequency,
			Action::AutoCompoundDelegatedStake {
				delegator: AccountId32::new(ALICE),
				collator: AccountId32::new(BOB),
				account_minimum,
			},
		);

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		events()
			.into_iter()
			.find(|e| match e {
				RuntimeEvent::AutomationTime(crate::Event::AutoCompoundDelegatorStakeFailed {
					..
				}) => true,
				_ => false,
			})
			.expect("AutoCompound failure event should have been emitted");
		assert!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME + frequency)
			.filter(|scheduled| {
				scheduled.tasks.iter().any(|t| *t == (AccountId32::new(ALICE), task_id))
			})
			.is_none());
		assert!(AutomationTime::get_account_task(AccountId32::new(ALICE), task_id).is_none());
	})
}

#[test]
fn trigger_tasks_updates_executions_left() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let message_one: Vec<u8> = vec![2, 5, 7];
		let owner = AccountId32::new(ALICE);
		let task_id01 = add_task_to_task_queue(
			ALICE,
			vec![40],
			vec![SCHEDULED_TIME, SCHEDULED_TIME + 3600],
			Action::Notify { message: message_one.clone() },
		);

		match AutomationTime::get_account_task(owner.clone(), task_id01) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 2);
			},
		}

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::Notify { message: message_one.clone() }),]
		);
		match AutomationTime::get_account_task(owner.clone(), task_id01) {
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
			Action::Notify { message: message_one.clone() },
		);

		match AutomationTime::get_account_task(owner.clone(), task_id01) {
			None => {
				panic!("A task should exist if it was scheduled")
			},
			Some(task) => {
				assert_eq!(task.schedule.known_executions_left(), 1);
			},
		}

		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));
		System::reset_events();

		AutomationTime::trigger_tasks(Weight::from_ref_time(120_000));

		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::Notify { message: message_one.clone() }),]
		);
		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id01), None);
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
			Action::Notify { message: message_one.clone() },
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			Action::Notify { message: message_two.clone() },
		);
		let task_id3 = add_task_to_task_queue(
			ALICE,
			vec![60],
			vec![SCHEDULED_TIME],
			Action::Notify { message: vec![50] },
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::on_initialize(1);
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::Notify { message: message_one.clone() }),
				RuntimeEvent::AutomationTime(crate::Event::Notify { message: message_two.clone() }),
			]
		);
		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id1), None);
		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id2), None);
		assert_ne!(AutomationTime::get_account_task(owner.clone(), task_id3), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		Timestamp::set_timestamp(START_BLOCK_TIME + (3600 * 1_000));
		AutomationTime::on_initialize(2);
		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
				who: AccountId32::new(ALICE),
				task_id: task_id3,
				execution_time: LAST_BLOCK_TIME
			})],
		);
		assert_eq!(AutomationTime::get_account_task(owner.clone(), task_id3), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 0);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
	})
}

#[test]
fn on_init_check_task_queue() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME - 7200));
		let mut tasks = vec![];

		for i in 0..5 {
			let task_id = add_task_to_task_queue(
				ALICE,
				vec![i],
				vec![SCHEDULED_TIME],
				Action::Notify { message: vec![i] },
			);
			tasks.push(task_id);
		}
		Timestamp::set_timestamp(START_BLOCK_TIME + (10 * 1000));
		AutomationTime::on_initialize(1);
		assert_eq!(
			events(),
			[RuntimeEvent::AutomationTime(crate::Event::Notify { message: vec![0] }),],
		);
		assert_eq!(AutomationTime::get_task_queue().len(), 4);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		Timestamp::set_timestamp(START_BLOCK_TIME + (40 * 1000));
		AutomationTime::on_initialize(2);
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::Notify { message: vec![1] }),
				RuntimeEvent::AutomationTime(crate::Event::Notify { message: vec![2] }),
			],
		);
		assert_eq!(AutomationTime::get_task_queue().len(), 2);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		Timestamp::set_timestamp(START_BLOCK_TIME + (3600 * 1000));
		AutomationTime::on_initialize(3);
		assert_eq!(
			events(),
			[
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: tasks[3],
					execution_time: LAST_BLOCK_TIME
				}),
				RuntimeEvent::AutomationTime(crate::Event::TaskMissed {
					who: AccountId32::new(ALICE),
					task_id: tasks[4],
					execution_time: LAST_BLOCK_TIME
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
			Action::Notify { message: message_one.clone() },
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			vec![SCHEDULED_TIME],
			Action::Notify { message: message_two.clone() },
		);
		let task_id3 = add_task_to_task_queue(
			ALICE,
			vec![60],
			vec![SCHEDULED_TIME],
			Action::Notify { message: vec![50] },
		);
		LastTimeSlot::<Test>::put((LAST_BLOCK_TIME, LAST_BLOCK_TIME));

		AutomationTime::on_initialize(1);
		assert_eq!(events(), []);
		Timestamp::set_timestamp(START_BLOCK_TIME + (3600 * 1_000));
		AutomationTime::on_initialize(2);
		assert_eq!(events(), [],);
		assert_ne!(AutomationTime::get_account_task(owner.clone(), task_id1), None);
		assert_ne!(AutomationTime::get_account_task(owner.clone(), task_id2), None);
		assert_ne!(AutomationTime::get_account_task(owner.clone(), task_id3), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 3);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
	})
}
