// This file is part of OAK Blockchain.

// Copyright (C) 2021 OAK Network
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

use crate::{mock::*, Error, LastTimeSlot, Task, TaskHashInput, TaskQueue, Tasks};
use frame_support::{assert_noop, assert_ok, traits::OnInitialize};
use frame_system::RawOrigin;
use sp_runtime::traits::{BlakeTwo256, Hash};

const SCHEDULED_TIME: u64 = 33198768180;

#[test]
fn schedule_invalid_time() {
	new_test_ext().execute_with(|| {
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
	new_test_ext().execute_with(|| {
		let start_block_time: u64 = (SCHEDULED_TIME + 5) * 1000;
		Timestamp::set_timestamp(start_block_time);
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
fn schedule_no_message() {
	new_test_ext().execute_with(|| {
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
	new_test_ext().execute_with(|| {
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
fn schedule_works() {
	new_test_ext().execute_with(|| {
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
fn schedule_duplicates_errors() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 60;
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			vec![50],
			scheduled_time,
			vec![2, 4, 5]
		));
		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
				vec![50],
				scheduled_time,
				vec![2, 4]
			),
			Error::<Test>::DuplicateTask,
		);
	})
}

#[test]
fn schedule_time_slot_full() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 120;
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			vec![50],
			scheduled_time,
			vec![2, 4]
		));
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			vec![60],
			scheduled_time,
			vec![2, 4, 5]
		));

		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
				vec![70],
				scheduled_time,
				vec![2]
			),
			Error::<Test>::TimeSlotFull,
		);
	})
}

#[test]
fn cancel_works_for_scheduled() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 180;
		let owner: AccountId = ALICE;
		let task_id1 = schedule_task(owner, vec![40], scheduled_time, vec![2, 4, 5]);
		let task_id2 = schedule_task(owner, vec![50], scheduled_time, vec![2, 4]);
		System::reset_events();

		assert_ok!(AutomationTime::cancel_task(Origin::signed(owner), task_id1,));
		assert_ok!(AutomationTime::cancel_task(Origin::signed(owner), task_id2,));

		if let Some(_) = AutomationTime::get_scheduled_tasks(scheduled_time) {
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
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 180;
		let owner: AccountId = ALICE;
		let task_id = add_task_to_task_queue(owner, vec![40], scheduled_time, vec![2, 4, 5]);

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
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 240;
		let task_id = schedule_task(ALICE, vec![40], scheduled_time, vec![2, 4, 5]);

		assert_noop!(
			AutomationTime::cancel_task(Origin::signed(BOB), task_id),
			Error::<Test>::NotTaskOwner,
		);
	})
}

#[test]
fn cancel_task_must_exist() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 300;
		let task = Task::<Test>::create_event_task(ALICE, vec![40], scheduled_time, vec![2, 4, 5]);
		let task_id = BlakeTwo256::hash_of(&task);

		assert_noop!(
			AutomationTime::cancel_task(Origin::signed(ALICE), task_id),
			Error::<Test>::TaskDoesNotExist,
		);
	})
}

#[test]
fn cancel_task_not_found() {
	new_test_ext().execute_with(|| {
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
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 240;
		let owner: AccountId = ALICE;
		let task_id = schedule_task(owner, vec![40], scheduled_time, vec![2, 4, 5]);
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
//10_000 + 10_000 + 20_000 per time slot added to queue
//10_000 + 10_000 + 20_000 per task run

#[test]
fn trigger_tasks_nothing_to_do() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 600;
		let start_block_time: u64 = (scheduled_time + 52) * 1000;

		LastTimeSlot::<Test>::put(scheduled_time);
		Timestamp::set_timestamp(start_block_time);
		System::reset_events();

		AutomationTime::trigger_tasks(60_000);

		assert_eq!(events(), vec![],);
	})
}

#[test]
fn trigger_tasks_completes_all_tasks() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 360;
		let start_block_time: u64 = (scheduled_time) * 1000;

		let message_one: Vec<u8> = vec![2, 4, 5];
		add_task_to_task_queue(ALICE, vec![40], scheduled_time, message_one.clone());
		let message_two: Vec<u8> = vec![2, 4];
		add_task_to_task_queue(ALICE, vec![50], scheduled_time, message_two.clone());

		LastTimeSlot::<Test>::put(scheduled_time);
		Timestamp::set_timestamp(start_block_time);
		System::reset_events();

		AutomationTime::trigger_tasks(90_000);

		assert_eq!(
			events(),
			[
				Event::AutomationTime(crate::Event::Notify { message: message_one.clone() }),
				Event::AutomationTime(crate::Event::Notify { message: message_two.clone() }),
			]
		);
		assert_eq!(0, AutomationTime::get_task_queue().len());
	})
}

#[test]
fn trigger_tasks_completes_some_tasks() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 420;
		let start_block_time: u64 = (scheduled_time) * 1000;

		let message_one: Vec<u8> = vec![2, 4, 5];
		add_task_to_task_queue(ALICE, vec![40], scheduled_time, message_one.clone());
		let message_two: Vec<u8> = vec![2, 4];
		add_task_to_task_queue(ALICE, vec![50], scheduled_time, message_two.clone());

		LastTimeSlot::<Test>::put(scheduled_time);
		Timestamp::set_timestamp(start_block_time);
		System::reset_events();

		AutomationTime::trigger_tasks(70_000);

		assert_eq!(
			events(),
			[Event::AutomationTime(crate::Event::Notify { message: message_one.clone() }),]
		);

		assert_eq!(1, AutomationTime::get_task_queue().len());
	})
}

#[test]
fn trigger_tasks_adds_more_tasks_to_task_queue() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 540;
		let start_block_time: u64 = (scheduled_time) * 1000;

		let message_one: Vec<u8> = vec![2, 4, 5];
		add_task_to_task_queue(ALICE, vec![40], scheduled_time, message_one.clone());
		let message_two: Vec<u8> = vec![2, 4];
		schedule_task(ALICE, vec![50], scheduled_time, message_two.clone());
		let message_three: Vec<u8> = vec![2, 4];
		schedule_task(ALICE, vec![60], scheduled_time, message_three.clone());

		LastTimeSlot::<Test>::put(scheduled_time - 60);
		Timestamp::set_timestamp(start_block_time);
		System::reset_events();

		AutomationTime::trigger_tasks(120_000);

		assert_eq!(
			events(),
			[
				Event::AutomationTime(crate::Event::Notify { message: message_one.clone() }),
				Event::AutomationTime(crate::Event::Notify { message: message_two.clone() }),
			]
		);
		assert_eq!(1, AutomationTime::get_task_queue().len());
	})
}

#[test]
fn trigger_tasks_only_adds_tasks_to_task_queue() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 600;
		let start_block_time: u64 = (scheduled_time + 52) * 1000;

		let message_one: Vec<u8> = vec![2, 4, 5];
		add_task_to_task_queue(ALICE, vec![40], scheduled_time, message_one.clone());

		LastTimeSlot::<Test>::put(scheduled_time - 120);
		Timestamp::set_timestamp(start_block_time);
		System::reset_events();

		AutomationTime::trigger_tasks(20_000);

		assert_eq!(events(), vec![],);
	})
}

#[test]
fn on_init_runs_tasks() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 480;
		let start_block_time: u64 = (scheduled_time) * 1000;

		let message_one: Vec<u8> = vec![2, 4, 5];
		add_task_to_task_queue(ALICE, vec![40], scheduled_time, message_one.clone());
		let message_two: Vec<u8> = vec![2, 4];
		add_task_to_task_queue(ALICE, vec![50], scheduled_time, message_two.clone());

		LastTimeSlot::<Test>::put(scheduled_time);
		Timestamp::set_timestamp(start_block_time);
		System::reset_events();

		AutomationTime::on_initialize(1);
		assert_eq!(
			events(),
			[Event::AutomationTime(crate::Event::Notify { message: message_one.clone() }),]
		);

		AutomationTime::on_initialize(2);
		assert_eq!(
			events(),
			[Event::AutomationTime(crate::Event::Notify { message: message_two.clone() }),]
		);

		assert_eq!(0, AutomationTime::get_task_queue().len());
	})
}

fn schedule_task(
	owner: AccountId,
	provided_id: Vec<u8>,
	scheduled_time: u64,
	message: Vec<u8>,
) -> sp_core::H256 {
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
	scheduled_time: u64,
	message: Vec<u8>,
) -> sp_core::H256 {
	let task_hash_input =
		TaskHashInput::<Test>::create_hash_input(owner.clone(), provided_id.clone());
	let task_id = BlakeTwo256::hash_of(&task_hash_input);
	let task = Task::<Test>::create_event_task(owner, provided_id, scheduled_time, message);
	Tasks::<Test>::insert(task_id, task);
	let mut task_queue = AutomationTime::get_task_queue();
	task_queue.push(task_id);
	TaskQueue::<Test>::put(task_queue);
	task_id
}

fn events() -> Vec<Event> {
	let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}
