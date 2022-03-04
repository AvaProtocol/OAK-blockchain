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
	mock::*, Action, Error, LastTimeSlot, MissedQueue, Shutdown, Task, TaskHashInput, TaskQueue,
	Tasks,
};
use frame_support::{assert_noop, assert_ok, traits::OnInitialize};
use frame_system::RawOrigin;
use sp_runtime::traits::{BlakeTwo256, Hash};

const START_BLOCK_TIME: u64 = 33198768180 * 1_000;
const SCHEDULED_TIME: u64 = START_BLOCK_TIME / 1_000 + 120;
const LAST_BLOCK_TIME: u64 = START_BLOCK_TIME / 1_000;

#[test]
fn schedule_invalid_time() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
				vec![50],
				SCHEDULED_TIME + 1,
				vec![12]
			),
			Error::<Test>::InvalidTime,
		);
	})
}

#[test]
fn schedule_past_time() {
	new_test_ext(START_BLOCK_TIME + 1_000_000).execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
				vec![50],
				SCHEDULED_TIME,
				vec![12]
			),
			Error::<Test>::PastTime,
		);

		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
				vec![50],
				SCHEDULED_TIME - 60,
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
				Origin::signed(ALICE),
				vec![50],
				SCHEDULED_TIME + 1 * 24 * 60 * 60,
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
				Origin::signed(ALICE),
				vec![50],
				SCHEDULED_TIME,
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
				Origin::signed(ALICE),
				vec![],
				SCHEDULED_TIME,
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
				Origin::signed(ALICE),
				vec![60],
				SCHEDULED_TIME,
				vec![12]
			),
			Error::<Test>::InsufficientBalance,
		);
	})
}

#[test]
fn schedule_notify_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		Balances::set_balance(RawOrigin::Root.into(), ALICE, 100_000, 5).unwrap();
		let message: Vec<u8> = vec![2, 4, 5];
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			vec![50],
			SCHEDULED_TIME,
			message.clone()
		));
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(task_ids) => match AutomationTime::get_task(task_ids[0]) {
				None => {
					panic!("A task should exist if it was scheduled")
				},
				Some(task) => {
					let expected_task = Task::<Test>::create_event_task(
						ALICE.clone(),
						vec![50],
						SCHEDULED_TIME,
						message,
					);

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
				Origin::signed(ALICE),
				vec![50],
				SCHEDULED_TIME,
				BOB,
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
				Origin::signed(ALICE),
				vec![50],
				SCHEDULED_TIME,
				ALICE,
				1,
			),
			Error::<Test>::TransferToSelf,
		);
	})
}

#[test]
fn schedule_native_transfer_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		Balances::set_balance(RawOrigin::Root.into(), ALICE, 100_000, 5).unwrap();
		assert_ok!(AutomationTime::schedule_native_transfer_task(
			Origin::signed(ALICE),
			vec![50],
			SCHEDULED_TIME,
			BOB,
			1,
		));
		match AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
			None => {
				panic!("A task should be scheduled")
			},
			Some(task_ids) => match AutomationTime::get_task(task_ids[0]) {
				None => {
					panic!("A task should exist if it was scheduled")
				},
				Some(task) => {
					let expected_task = Task::<Test>::create_native_transfer_task(
						ALICE.clone(),
						vec![50],
						SCHEDULED_TIME,
						BOB,
						1,
					);

					assert_eq!(task, expected_task);
				},
			},
		}
	})
}

#[test]
fn schedule_duplicates_errors() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		Balances::set_balance(RawOrigin::Root.into(), ALICE, 100_000, 5).unwrap();
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			vec![50],
			SCHEDULED_TIME,
			vec![2, 4, 5]
		));
		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
				vec![50],
				SCHEDULED_TIME,
				vec![2, 4]
			),
			Error::<Test>::DuplicateTask,
		);
	})
}

#[test]
fn schedule_time_slot_full() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		Balances::set_balance(RawOrigin::Root.into(), ALICE, 100_000, 5).unwrap();
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			vec![50],
			SCHEDULED_TIME,
			vec![2, 4]
		));
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			vec![60],
			SCHEDULED_TIME,
			vec![2, 4, 5]
		));

		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
				vec![70],
				SCHEDULED_TIME,
				vec![2]
			),
			Error::<Test>::TimeSlotFull,
		);
	})
}

#[test]
fn cancel_works_for_scheduled() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner: AccountId = ALICE;
		let task_id1 = schedule_task(owner, vec![40], SCHEDULED_TIME, vec![2, 4, 5]);
		let task_id2 = schedule_task(owner, vec![50], SCHEDULED_TIME, vec![2, 4]);
		System::reset_events();

		assert_ok!(AutomationTime::cancel_task(Origin::signed(owner), task_id1,));
		assert_ok!(AutomationTime::cancel_task(Origin::signed(owner), task_id2,));

		if let Some(_) = AutomationTime::get_scheduled_tasks(SCHEDULED_TIME) {
			panic!("Since there were only two tasks scheduled for the time it should have been deleted")
		}
		assert_eq!(
			events(),
			[
				Event::AutomationTime(crate::Event::TaskCancelled {
					who: owner,
					task_id: task_id1
				}),
				Event::AutomationTime(crate::Event::TaskCancelled {
					who: owner,
					task_id: task_id2
				}),
			]
		);
	})
}

#[test]
fn cancel_works_for_tasks_in_queue() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner: AccountId = ALICE;
		let task_id =
			add_task_to_task_queue(owner, vec![40], Action::Notify { message: vec![2, 4, 5] });

		assert_eq!(task_id, AutomationTime::get_task_queue()[0]);
		assert_eq!(1, AutomationTime::get_task_queue().len());

		assert_ok!(AutomationTime::cancel_task(Origin::signed(owner), task_id,));

		assert_eq!(
			events(),
			[Event::AutomationTime(crate::Event::TaskCancelled { who: owner, task_id }),]
		);
		assert_eq!(0, AutomationTime::get_task_queue().len());
	})
}

#[test]
fn cancel_must_be_owner() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id = schedule_task(ALICE, vec![40], SCHEDULED_TIME, vec![2, 4, 5]);

		assert_noop!(
			AutomationTime::cancel_task(Origin::signed(BOB), task_id),
			Error::<Test>::NotTaskOwner,
		);
	})
}

#[test]
fn cancel_task_must_exist() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task = Task::<Test>::create_event_task(ALICE, vec![40], SCHEDULED_TIME, vec![2, 4, 5]);
		let task_id = BlakeTwo256::hash_of(&task);

		assert_noop!(
			AutomationTime::cancel_task(Origin::signed(ALICE), task_id),
			Error::<Test>::TaskDoesNotExist,
		);
	})
}

#[test]
fn cancel_task_not_found() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner: AccountId = ALICE;
		let task = Task::<Test>::create_event_task(owner, vec![40], SCHEDULED_TIME, vec![2, 4, 5]);
		let task_id = BlakeTwo256::hash_of(&task);
		<Tasks<Test>>::insert(task_id, task);

		assert_ok!(AutomationTime::cancel_task(Origin::signed(owner), task_id,));
		assert_eq!(
			events(),
			[
				Event::AutomationTime(crate::Event::TaskNotFound { task_id }),
				Event::AutomationTime(crate::Event::TaskCancelled { who: owner, task_id })
			]
		);
	})
}

#[test]
fn force_cancel_task_works() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let owner: AccountId = ALICE;
		let task_id = schedule_task(owner, vec![40], SCHEDULED_TIME, vec![2, 4, 5]);
		System::reset_events();

		assert_ok!(AutomationTime::force_cancel_task(RawOrigin::Root.into(), task_id));
		assert_eq!(
			events(),
			[Event::AutomationTime(crate::Event::TaskCancelled { who: owner, task_id }),]
		);
	})
}

// Weights to use for tests below
//20_000 for nothing
//10_000 for no updates
//20_000 for a new time slot
//20_000 per time slot missed
//10_000 to check if we have tasks to run
//10_000 + 10_000 + 30_000 per task run OR 20_000 per unfound task
//10_000 to check if we have missed tasks to run
//10_000 + 10_000 + 30_000 per missed task run

#[test]
fn trigger_tasks_handles_first_run() {
	new_test_ext(0).execute_with(|| {
		AutomationTime::trigger_tasks(30_000);

		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_nothing_to_do() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		LastTimeSlot::<Test>::put(LAST_BLOCK_TIME);

		AutomationTime::trigger_tasks(30_000);

		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_updates_queues() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let missed_task_id =
			add_task_to_task_queue(ALICE, vec![40], Action::Notify { message: vec![40] });
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		let scheduled_task_id = schedule_task(ALICE, vec![50], SCHEDULED_TIME, vec![50]);
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put(SCHEDULED_TIME - 60);
		System::reset_events();

		AutomationTime::trigger_tasks(50_000);

		assert_eq!(AutomationTime::get_missed_queue().len(), 1);
		assert_eq!(AutomationTime::get_missed_queue()[0], missed_task_id);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_task_queue()[0], scheduled_task_id);
		assert_eq!(AutomationTime::get_scheduled_tasks(SCHEDULED_TIME), None);
		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_handles_missed_slots() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		add_task_to_task_queue(ALICE, vec![40], Action::Notify { message: vec![40] });
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		let missed_task_id = schedule_task(ALICE, vec![50], SCHEDULED_TIME - 60, vec![50]);
		let scheduled_task_id = schedule_task(ALICE, vec![60], SCHEDULED_TIME, vec![50]);
		Timestamp::set_timestamp(SCHEDULED_TIME * 1_000);
		LastTimeSlot::<Test>::put(SCHEDULED_TIME - 120);
		System::reset_events();

		AutomationTime::trigger_tasks(70_000);

		assert_eq!(AutomationTime::get_missed_queue().len(), 2);
		assert_eq!(AutomationTime::get_missed_queue()[1], missed_task_id);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_task_queue()[0], scheduled_task_id);
		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_completes_all_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let message_one: Vec<u8> = vec![2, 4, 5];
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			Action::Notify { message: message_one.clone() },
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			Action::Notify { message: message_two.clone() },
		);
		LastTimeSlot::<Test>::put(LAST_BLOCK_TIME);

		AutomationTime::trigger_tasks(120_000);

		assert_eq!(
			events(),
			[
				Event::AutomationTime(crate::Event::Notify { message: message_one.clone() }),
				Event::AutomationTime(crate::Event::Notify { message: message_two.clone() }),
			]
		);
		assert_eq!(0, AutomationTime::get_task_queue().len());
		assert_eq!(AutomationTime::get_task(task_id1), None);
		assert_eq!(AutomationTime::get_task(task_id2), None);
	})
}

#[test]
fn trigger_tasks_handles_nonexisting_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_hash_input = TaskHashInput::<Test>::create_hash_input(ALICE, vec![20]);
		let bad_task_id = BlakeTwo256::hash_of(&task_hash_input);
		let mut task_queue = AutomationTime::get_task_queue();
		task_queue.push(bad_task_id);
		TaskQueue::<Test>::put(task_queue);
		LastTimeSlot::<Test>::put(LAST_BLOCK_TIME);

		AutomationTime::trigger_tasks(90_000);

		assert_eq!(
			events(),
			[Event::AutomationTime(crate::Event::TaskNotFound { task_id: bad_task_id }),]
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
			Action::Notify { message: message_one.clone() },
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			Action::Notify { message: message_two.clone() },
		);
		LastTimeSlot::<Test>::put(LAST_BLOCK_TIME);

		AutomationTime::trigger_tasks(90_000);

		assert_eq!(
			events(),
			[Event::AutomationTime(crate::Event::Notify { message: message_one.clone() }),]
		);

		assert_eq!(1, AutomationTime::get_task_queue().len());
		assert_eq!(AutomationTime::get_task(task_id1), None);
		assert_ne!(AutomationTime::get_task(task_id2), None);
	})
}

#[test]
fn trigger_tasks_completes_all_missed_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let task_id1 =
			add_task_to_missed_queue(ALICE, vec![40], Action::Notify { message: vec![40] });
		let task_id2 =
			add_task_to_missed_queue(ALICE, vec![50], Action::Notify { message: vec![40] });
		LastTimeSlot::<Test>::put(LAST_BLOCK_TIME);

		AutomationTime::trigger_tasks(130_000);

		assert_eq!(
			events(),
			[
				Event::AutomationTime(crate::Event::TaskMissed { who: ALICE, task_id: task_id1 }),
				Event::AutomationTime(crate::Event::TaskMissed { who: ALICE, task_id: task_id2 }),
			]
		);

		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
		assert_eq!(AutomationTime::get_task(task_id1), None);
		assert_eq!(AutomationTime::get_task(task_id2), None);
	})
}

#[test]
fn trigger_tasks_completes_some_native_transfer_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		Balances::set_balance(RawOrigin::Root.into(), ALICE, 1000, 5).unwrap();
		add_task_to_task_queue(
			ALICE,
			vec![40],
			Action::NativeTransfer { sender: ALICE, recipient: BOB, amount: 1 },
		);
		add_task_to_task_queue(
			ALICE,
			vec![50],
			Action::NativeTransfer { sender: ALICE, recipient: BOB, amount: 1 },
		);

		LastTimeSlot::<Test>::put(LAST_BLOCK_TIME);
		System::reset_events();

		AutomationTime::trigger_tasks(120_000);

		assert_eq!(Balances::free_balance(ALICE), 998);
		assert_eq!(Balances::free_balance(BOB), 2);
	})
}

#[test]
fn on_init_runs_tasks() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		let message_one: Vec<u8> = vec![2, 4, 5];
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			Action::Notify { message: message_one.clone() },
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			Action::Notify { message: message_two.clone() },
		);
		let task_id3 =
			add_task_to_task_queue(ALICE, vec![60], Action::Notify { message: vec![50] });
		LastTimeSlot::<Test>::put(LAST_BLOCK_TIME);

		AutomationTime::on_initialize(1);

		assert_eq!(
			events(),
			[
				Event::AutomationTime(crate::Event::Notify { message: message_one.clone() }),
				Event::AutomationTime(crate::Event::Notify { message: message_two.clone() }),
			]
		);
		assert_eq!(AutomationTime::get_task(task_id1), None);
		assert_eq!(AutomationTime::get_task(task_id2), None);
		assert_ne!(AutomationTime::get_task(task_id3), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 1);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);

		Timestamp::set_timestamp(START_BLOCK_TIME + (60 * 1_000));
		AutomationTime::on_initialize(2);
		assert_eq!(
			events(),
			[Event::AutomationTime(crate::Event::TaskMissed { who: ALICE, task_id: task_id3 })],
		);
		assert_eq!(AutomationTime::get_task(task_id3), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 0);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
	})
}

#[test]
fn on_init_shutdown() {
	new_test_ext(START_BLOCK_TIME).execute_with(|| {
		Shutdown::<Test>::put(true);

		let message_one: Vec<u8> = vec![2, 4, 5];
		let task_id1 = add_task_to_task_queue(
			ALICE,
			vec![40],
			Action::Notify { message: message_one.clone() },
		);
		let message_two: Vec<u8> = vec![2, 4];
		let task_id2 = add_task_to_task_queue(
			ALICE,
			vec![50],
			Action::Notify { message: message_two.clone() },
		);
		let task_id3 =
			add_task_to_task_queue(ALICE, vec![60], Action::Notify { message: vec![50] });
		LastTimeSlot::<Test>::put(LAST_BLOCK_TIME);

		AutomationTime::on_initialize(1);
		assert_eq!(events(), []);
		Timestamp::set_timestamp(START_BLOCK_TIME + (60 * 1_000));
		AutomationTime::on_initialize(2);
		assert_eq!(events(), [],);
		assert_ne!(AutomationTime::get_task(task_id1), None);
		assert_ne!(AutomationTime::get_task(task_id2), None);
		assert_ne!(AutomationTime::get_task(task_id3), None);
		assert_eq!(AutomationTime::get_task_queue().len(), 3);
		assert_eq!(AutomationTime::get_missed_queue().len(), 0);
	})
}

fn schedule_task(
	owner: AccountId,
	provided_id: Vec<u8>,
	scheduled_time: u64,
	message: Vec<u8>,
) -> sp_core::H256 {
	Balances::set_balance(RawOrigin::Root.into(), owner, 100_000, 5).unwrap();
	let task_hash_input =
		TaskHashInput::<Test>::create_hash_input(owner.clone(), provided_id.clone());
	assert_ok!(AutomationTime::schedule_notify_task(
		Origin::signed(owner),
		provided_id,
		scheduled_time,
		message,
	));
	BlakeTwo256::hash_of(&task_hash_input)
}

fn add_task_to_task_queue(
	owner: AccountId,
	provided_id: Vec<u8>,
	action: Action<Test>,
) -> sp_core::H256 {
	let task_id = create_task(owner, provided_id, 0, action);
	let mut task_queue = AutomationTime::get_task_queue();
	task_queue.push(task_id);
	TaskQueue::<Test>::put(task_queue);
	task_id
}

fn add_task_to_missed_queue(
	owner: AccountId,
	provided_id: Vec<u8>,
	action: Action<Test>,
) -> sp_core::H256 {
	let task_id = create_task(owner, provided_id, 0, action);
	let mut missed_queue = AutomationTime::get_missed_queue();
	missed_queue.push(task_id);
	MissedQueue::<Test>::put(missed_queue);
	task_id
}

fn create_task(
	owner: AccountId,
	provided_id: Vec<u8>,
	scheduled_time: u64,
	action: Action<Test>,
) -> sp_core::H256 {
	let task_hash_input =
		TaskHashInput::<Test>::create_hash_input(owner.clone(), provided_id.clone());
	let task_id = BlakeTwo256::hash_of(&task_hash_input);
	let task = match action {
		Action::Notify { message } =>
			Task::<Test>::create_event_task(owner, provided_id, scheduled_time, message),
		Action::NativeTransfer { sender: _, recipient, amount } =>
			Task::<Test>::create_native_transfer_task(
				owner,
				provided_id,
				scheduled_time,
				recipient,
				amount,
			),
	};
	Tasks::<Test>::insert(task_id, task);
	task_id
}

fn events() -> Vec<Event> {
	let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}
