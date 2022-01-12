// use super::*;
// use mock::*;
use crate::{mock::*, Error, Task};
use frame_support::{assert_noop, assert_ok};

const SCHEDULED_TIME: u64 = 33198768180;

#[test]
fn schedule_invalid_time() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
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
			AutomationTime::schedule_notify_task(Origin::signed(ALICE), SCHEDULED_TIME, vec![12]),
			Error::<Test>::PastTime,
		);

		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
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
			AutomationTime::schedule_notify_task(Origin::signed(ALICE), SCHEDULED_TIME, vec![]),
			Error::<Test>::EmptyMessage,
		);
	})
}

#[test]
fn schedule_works() {
	new_test_ext().execute_with(|| {
		let message: Vec<u8> = vec![2, 4, 5];
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
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
					let expected_task =
						Task::<Test>::create_event_task(ALICE.clone(), SCHEDULED_TIME, message);

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
		let message: Vec<u8> = vec![2, 4, 5];
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			scheduled_time,
			message.clone()
		));
		assert_noop!(
			AutomationTime::schedule_notify_task(
				Origin::signed(ALICE),
				scheduled_time,
				message.clone()
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
			scheduled_time,
			vec![2, 4]
		));
		assert_ok!(AutomationTime::schedule_notify_task(
			Origin::signed(ALICE),
			scheduled_time,
			vec![2, 4, 5]
		));

		assert_noop!(
			AutomationTime::schedule_notify_task(Origin::signed(ALICE), scheduled_time, vec![2]),
			Error::<Test>::TimeSlotFull,
		);
	})
}

#[test]
fn cancel_works() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 180;
		let task_id1 = create_task(ALICE, scheduled_time, vec![2, 4, 5]);
		let task_id2 = create_task(ALICE, scheduled_time, vec![2, 4]);

		assert_ok!(AutomationTime::cancel_task(Origin::signed(ALICE), task_id1,));

		assert_ok!(AutomationTime::cancel_task(Origin::signed(ALICE), task_id2,));

		if let Some(_) = AutomationTime::get_scheduled_tasks(scheduled_time) {
			panic!("Since there were only two tasks scheduled for the time it should have been deleted")
		}
	})
}

#[test]
fn cancel_must_be_owner() {
	new_test_ext().execute_with(|| {
		let scheduled_time = SCHEDULED_TIME + 240;
		let task_id = create_task(ALICE, scheduled_time, vec![2, 4, 5]);

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
		let task_id = create_task(ALICE, scheduled_time, vec![2, 4, 5]);

		assert_ok!(AutomationTime::cancel_task(Origin::signed(ALICE), task_id,));
		assert_noop!(
			AutomationTime::cancel_task(Origin::signed(ALICE), task_id),
			Error::<Test>::TaskDoesNotExist,
		);
	})
}

fn create_task(owner: AccountId, scheduled_time: u64, message: Vec<u8>) -> sp_core::H256 {
	assert_ok!(AutomationTime::schedule_notify_task(
		Origin::signed(owner),
		scheduled_time,
		message,
	));
	let task_ids = AutomationTime::get_scheduled_tasks(scheduled_time).unwrap();
	task_ids[task_ids.len() - 1]
}
