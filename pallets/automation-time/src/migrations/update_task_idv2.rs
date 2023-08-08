use core::marker::PhantomData;

use crate::{
	AccountTaskId, Config, MissedTaskV2, MissedTaskV2Of, ScheduledTasksOf, TaskIdV2, UnixTime,
};
use codec::{Decode, Encode};
use frame_support::{
	dispatch::EncodeLike,
	storage::types::ValueQuery,
	traits::{Get, OnRuntimeUpgrade},
	weights::{RuntimeDbWeight, Weight},
	Twox64Concat,
};
use scale_info::{prelude::format, TypeInfo};

use sp_runtime::traits::Convert;
use sp_std::{vec, vec::Vec};
use xcm::{latest::prelude::*, VersionedMultiLocation};

use crate::migrations::utils::{
	deprecate::{generate_old_task_id, old_taskid_to_idv2},
	OldAccountTaskId, OldMissedTaskV2Of, OldScheduledTasksOf, TEST_TASKID1, TEST_TASKID2,
};

// This is old TaskQueueV2 with old Task
#[frame_support::storage_alias]
pub type TaskQueueV2<T: Config> =
	StorageValue<AutomationTime, Vec<OldAccountTaskId<T>>, ValueQuery>;

pub struct UpdateTaskIDV2ForTaskQueueV2<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for UpdateTaskIDV2ForTaskQueueV2<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "automation-time", "TaskID migration");

		let mut storage_ops: u64 = 0;

		let old_task_queue = TaskQueueV2::<T>::get();
		let migrated_tasks = old_task_queue.len();

		let new_task_queue: Vec<AccountTaskId<T>> = old_task_queue
			.iter()
			.map(|(a, b)| ((*a).clone(), old_taskid_to_idv2::<T>(b)))
			.collect::<Vec<AccountTaskId<T>>>();

		storage_ops += old_task_queue.len() as u64;
		crate::TaskQueueV2::<T>::put(new_task_queue);

		log::info!(
			target: "automation-time",
			"migration: Update TaskQueueV2 succesful! Migrated {} tasks.",
			migrated_tasks
		);

		let db_weight: RuntimeDbWeight = T::DbWeight::get();
		db_weight.reads_writes(storage_ops + 1, storage_ops + 1)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		let prev_count = crate::TaskQueueV2::<T>::get().len() as u32;
		Ok(prev_count.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(prev_count: Vec<u8>) -> Result<(), &'static str> {
		let prev_count: u32 = Decode::decode(&mut prev_count.as_slice())
			.expect("the state parameter should be something that was generated by pre_upgrade");
		let post_count = crate::TaskQueueV2::<T>::get().len() as u32;
		assert!(post_count == prev_count);

		Ok(())
	}
}

// This is old ScheduledTasksV3 with old Taskid using T::Hash
#[frame_support::storage_alias]
pub type ScheduledTasksV3<T: Config> =
	StorageMap<AutomationTime, Twox64Concat, UnixTime, OldScheduledTasksOf<T>>;
pub struct UpdateTaskIDV2ForScheduledTasksV3<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for UpdateTaskIDV2ForScheduledTasksV3<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "automation-time", "TaskID migration");

		let mut migrated_tasks = 0;
		let mut storage_ops: u64 = 0;
		ScheduledTasksV3::<T>::iter().for_each(|(time_slot, old_scheduled_tasks)| {
			storage_ops += 1;
			let migrated_scheduled_tasks: Vec<(T::AccountId, TaskIdV2)> = old_scheduled_tasks
				.tasks
				.into_iter()
				.map(|(account_id, old_task_id)| {
					migrated_tasks += 1;
					storage_ops += 1;
					(account_id.clone(), old_taskid_to_idv2::<T>(&(old_task_id.clone())))
				})
				.collect::<Vec<(T::AccountId, TaskIdV2)>>();

			storage_ops += 1;
			crate::ScheduledTasksV3::<T>::insert(
				time_slot,
				ScheduledTasksOf::<T> {
					tasks: migrated_scheduled_tasks,
					weight: old_scheduled_tasks.weight,
				},
			);
		});

		log::info!(
			target: "automation-time",
			"migration: Update ScheduledTasksV3 succesful! Migrated {} tasks.",
			migrated_tasks
		);

		let db_weight: RuntimeDbWeight = T::DbWeight::get();
		db_weight.reads_writes(storage_ops + 1, storage_ops + 1)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		let prev_count = crate::ScheduledTasksV3::<T>::iter().count() as u32;
		Ok(prev_count.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(prev_count: Vec<u8>) -> Result<(), &'static str> {
		let prev_count: u32 = Decode::decode(&mut prev_count.as_slice())
			.expect("the state parameter should be something that was generated by pre_upgrade");
		let post_count = crate::ScheduledTasksV3::<T>::iter().count() as u32;
		assert!(post_count == prev_count);

		Ok(())
	}
}

// This is old MissedQueueV2 with old Task
#[frame_support::storage_alias]
pub type MissedQueueV2<T: Config> =
	StorageValue<AutomationTime, Vec<OldMissedTaskV2Of<T>>, ValueQuery>;

pub struct UpdateTaskIDV2ForMissedQueueV2<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for UpdateTaskIDV2ForMissedQueueV2<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "automation-time", "TaskID migration");

		let old_missed_queue = MissedQueueV2::<T>::get();
		let migrated_tasks: u64 = old_missed_queue.len() as u64;

		let migrated_missed_queuee: Vec<MissedTaskV2Of<T>> = old_missed_queue
			.into_iter()
			.map(|old_missed_task| {
				let a: MissedTaskV2Of<T> = MissedTaskV2Of::<T> {
					owner_id: old_missed_task.owner_id.clone(),
					execution_time: old_missed_task.execution_time,
					task_id: old_taskid_to_idv2::<T>(&old_missed_task.task_id.clone()),
				};

				a
			})
			.collect::<Vec<MissedTaskV2Of<T>>>();

		crate::MissedQueueV2::<T>::put(migrated_missed_queuee);

		log::info!(
			target: "automation-time",
			"migration: Update MissedQueueV2 succesful! Migrated {} tasks.",
			migrated_tasks
		);

		let db_weight: RuntimeDbWeight = <T as frame_system::Config>::DbWeight::get();
		db_weight.reads_writes(migrated_tasks + 1, migrated_tasks + 1)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		let prev_count = crate::MissedQueueV2::<T>::get().len() as u32;
		Ok(prev_count.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(prev_count: Vec<u8>) -> Result<(), &'static str> {
		let prev_count: u32 = Decode::decode(&mut prev_count.as_slice())
			.expect("the state parameter should be something that was generated by pre_upgrade");
		let post_count = crate::MissedQueueV2::<T>::get().len() as u32;
		assert!(post_count == prev_count);

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::{
		generate_old_task_id, MissedTaskV2, UpdateTaskIDV2ForMissedQueueV2,
		UpdateTaskIDV2ForScheduledTasksV3, UpdateTaskIDV2ForTaskQueueV2, TEST_TASKID1,
		TEST_TASKID2,
	};
	use crate::{
		migrations::utils::{OldMissedTaskV2, OldScheduledTasksOf},
		mock::*,
	};
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_runtime::AccountId32;

	#[test]
	fn on_runtime_upgrade() {
		new_test_ext(0).execute_with(|| {
			let account_id1 = AccountId32::new(ALICE);
			let account_id2 = AccountId32::new(BOB);

			let task_id1: Vec<u8> = TEST_TASKID1.as_bytes().to_vec();
			let task_id2: Vec<u8> = TEST_TASKID2.as_bytes().to_vec();

			let old_taskqueuev2 = vec![
				(account_id1.clone(), generate_old_task_id::<Test>(account_id1.clone(), vec![1])),
				(account_id2.clone(), generate_old_task_id::<Test>(account_id2.clone(), vec![2])),
			];

			super::TaskQueueV2::<Test>::put(old_taskqueuev2);
			UpdateTaskIDV2ForTaskQueueV2::<Test>::on_runtime_upgrade();

			let queue = crate::TaskQueueV2::<Test>::get();
			let (task_owner, task_id) =
				queue.first().expect("migration task failed to copy task over");

			assert_eq!(
				*task_owner,
				account_id1.clone(),
				"migration failed to convert OldTaskId -> TaskIDV2 for TaskQueueV2"
			);
			assert_eq!(*task_id, task_id1);

			let (task_owner, task_id) =
				queue.last().expect("migration task failed to copy task over");
			assert_eq!(
				*task_owner,
				account_id2.clone(),
				"migration failed to convert OldTaskId -> TaskIDV2 for TaskQueueV2"
			);
			assert_eq!(*task_id, task_id2);
		})
	}

	#[test]
	fn on_runtime_upgrade_for_schedule_taskv3() {
		new_test_ext(0).execute_with(|| {
			let task_owner1 = AccountId32::new(ALICE);
			let task_owner2 = AccountId32::new(BOB);

			// These are H256/BlakeTwo256 hex generate from our old task id generation from hashing
			let task_id1: Vec<u8> = TEST_TASKID1.as_bytes().to_vec();
			let task_id2: Vec<u8> = TEST_TASKID2.as_bytes().to_vec();

			super::ScheduledTasksV3::<Test>::insert(
				3600,
				OldScheduledTasksOf::<Test> {
					tasks: vec![
						(
							task_owner1.clone(),
							generate_old_task_id::<Test>(task_owner1.clone(), vec![1]),
						),
						(
							task_owner2.clone(),
							generate_old_task_id::<Test>(task_owner2.clone(), vec![2]),
						),
					],
					// just a random number to compare later on
					weight: 1_789_657,
				},
			);

			super::ScheduledTasksV3::<Test>::insert(
				7200,
				OldScheduledTasksOf::<Test> {
					tasks: vec![
						(
							task_owner2.clone(),
							generate_old_task_id::<Test>(task_owner2.clone(), vec![33]),
						),
						(
							task_owner1.clone(),
							generate_old_task_id::<Test>(task_owner1.clone(), vec![32]),
						),
					],
					// just a random number to compare later on
					weight: 1_967_672,
				},
			);
			UpdateTaskIDV2ForScheduledTasksV3::<Test>::on_runtime_upgrade();

			// ensure all the slots are converted properly
			let scheduled_tasks = crate::ScheduledTasksV3::<Test>::get(3600)
				.expect("ScheduledTasksV3 failed to migrate");
			assert_eq!(
				scheduled_tasks.weight, 1_789_657,
				"migration failed to convert old ScheduledTasksV3 to new TaskID format"
			);
			assert_eq!(
				scheduled_tasks.tasks,
				vec![
					(task_owner1.clone(), task_id1.clone()),
					(task_owner2.clone(), task_id2.clone()),
				],
				"migration failed to convert old ScheduledTasksV3 to new TaskID format"
			);

			let scheduled_tasks = crate::ScheduledTasksV3::<Test>::get(7200)
				.expect("ScheduledTasksV3 failed to migrate");
			assert_eq!(
				scheduled_tasks.weight, 1_967_672,
				"migration failed to convert old ScheduledTasksV3 to new TaskID format"
			);
			assert_eq!(
				scheduled_tasks.tasks,
				vec![
					// (task owner, vec![33]) hash -> "0x7191f3d83bcbeb221f7b00f501e9a8da3ba3c2d15a672eb694ee7e09dbaddd1e"
					(
						task_owner2.clone(),
						"0x7191f3d83bcbeb221f7b00f501e9a8da3ba3c2d15a672eb694ee7e09dbaddd1e"
							.as_bytes()
							.to_vec(),
					),
					// (task owner1, vec![32]) hash -> "0xe94040ca5d09f0e1023aecf5abc3afac0b9e66bc3b1209183b3a009f4c073c2b"
					(
						task_owner1.clone(),
						"0xe94040ca5d09f0e1023aecf5abc3afac0b9e66bc3b1209183b3a009f4c073c2b"
							.as_bytes()
							.to_vec(),
					),
				],
				"migration failed to convert old ScheduledTasksV3 to new TaskID format"
			);
		})
	}

	#[test]
	fn on_runtime_upgrade_for_missed_queue_v2() {
		new_test_ext(0).execute_with(|| {
			let account_id1 = AccountId32::new(ALICE);
			let account_id2 = AccountId32::new(BOB);

			// These are H256/BlakeTwo256 hex generate from our old task id generation from hashing
			let task_id1: Vec<u8> = TEST_TASKID1.as_bytes().to_vec();
			let task_id2: Vec<u8> = TEST_TASKID2.as_bytes().to_vec();

			let old_missed_queueu_v2 = vec![
				OldMissedTaskV2 {
					owner_id: account_id1.clone(),
					task_id: generate_old_task_id::<Test>(account_id1.clone(), vec![1]),
					execution_time: 3600,
				},
				OldMissedTaskV2 {
					owner_id: account_id2.clone(),
					task_id: generate_old_task_id::<Test>(account_id2.clone(), vec![2]),
					execution_time: 7200,
				},
			];

			super::MissedQueueV2::<Test>::put(old_missed_queueu_v2);
			UpdateTaskIDV2ForMissedQueueV2::<Test>::on_runtime_upgrade();

			let queue = crate::MissedQueueV2::<Test>::get();

			assert_eq!(
				queue.first().expect("migration task failed to copy missed queuev2 over"),
				&MissedTaskV2 {
                    owner_id: account_id1.clone(),
                    task_id: task_id1,
                    execution_time: 3600,
                },
				"migration failed to convert old MissedTaskV2 -> new MissedTaskV2 for MissedQueueV2"
			);

			assert_eq!(
				queue.last().expect("migration task failed to copy missed queuev2 over"),
				&MissedTaskV2 {
                    owner_id: account_id2.clone(),
                    task_id: task_id2,
                    execution_time: 7200,
                },
				"migration failed to convert old MissedTaskV2 -> new MissedTaskV2 for MissedQueueV2"
			);
		})
	}
}
