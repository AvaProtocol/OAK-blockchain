use super::*;
use crate::{Config, Weight};
use frame_support::{
	pallet_prelude::PhantomData,
	traits::{Get, OnRuntimeUpgrade},
};

pub const PALLET_PREFIX: &[u8] = b"AutomationTime";

// Migrating LastTimeSlot from a single time to a tuple.
// NOTE: The 2 UnixTime stamps represent (last_time_slot, last_missed_slot).
// `last_time_slot` represents the last time slot that the task queue was updated.
// `last_missed_slot` represents the last scheduled slot where the missed queue has checked for missed tasks.
// pub mod v1 {
// 	use frame_support::{migration::get_storage_value, traits::StorageVersion};

// 	use crate::{LastTimeSlot, Pallet};

// 	use super::*;

// 	pub fn migrate<T: Config>() -> Weight {
// 		log::info!(target: "automation-time", "Migrating automation-time v1");
// 		let pallet_prefix: &[u8] = b"AutomationTime";
// 		let storage_item_prefix: &[u8] = b"LastTimeSlot";

// 		let stored_data = get_storage_value::<u64>(pallet_prefix, storage_item_prefix, &[])
// 			.expect("Must have last slot value");

// 		LastTimeSlot::<T>::put((stored_data, stored_data));
// 		log::info!(target: "automation-time", "Completed automation-time migration to v1");
// 		StorageVersion::new(1).put::<Pallet<T>>();
// 		T::DbWeight::get().reads_writes(1, 1)
// 	}
// }

// Swapping time interval from minute to hour. We wiped all tasks for this.
// pub mod v2 {
// 	use frame_support::{
// 		migration::{get_storage_value, storage_key_iter},
// 		traits::StorageVersion,
// 		BoundedVec, Twox64Concat,
// 	};

// 	use crate::{LastTimeSlot, MissedQueue, MissedTask, Pallet, Task, TaskQueue, Vec};

// 	use super::*;

// 	pub fn migrate<T: Config>() -> Weight {
// 		log::info!(target: "automation-time", "Migrating automation-time v2");
// 		let pallet_prefix: &[u8] = b"AutomationTime";

// 		let scheduled_tasks_prefix: &[u8] = b"ScheduledTasks";
// 		let _scheduled_tasks: Vec<_> = storage_key_iter::<
// 			u64,
// 			BoundedVec<T::Hash, T::MaxTasksPerSlot>,
// 			Twox64Concat,
// 		>(pallet_prefix, scheduled_tasks_prefix)
// 		.drain()
// 		.collect();
// 		let empty_task_queue: Vec<T::Hash> = Vec::new();
// 		TaskQueue::<T>::put(empty_task_queue);
// 		let empty_missed_queue: Vec<MissedTask<T>> = Vec::new();
// 		MissedQueue::<T>::put(empty_missed_queue);
// 		let tasks_prefix: &[u8] = b"Tasks";
// 		let _tasks: Vec<_> =
// 			storage_key_iter::<T::Hash, Task<T>, Twox64Concat>(pallet_prefix, tasks_prefix)
// 				.drain()
// 				.collect();

// 		let pallet_prefix: &[u8] = b"AutomationTime";
// 		let storage_item_prefix: &[u8] = b"LastTimeSlot";
// 		let stored_min_slot = get_storage_value::<u64>(pallet_prefix, storage_item_prefix, &[])
// 			.expect("Must have last slot value");
// 		let diff_to_hour = stored_min_slot % 3600;
// 		let current_hour_slot = stored_min_slot.saturating_sub(diff_to_hour);
// 		LastTimeSlot::<T>::put((current_hour_slot, current_hour_slot));

// 		log::info!(target: "automation-time", "Completed automation-time migration to v2");
// 		StorageVersion::new(2).put::<Pallet<T>>();
// 		T::DbWeight::get().reads_writes(4, 4)
// 	}
// }

// pub mod v3 {
// 	use super::*;

// 	use frame_support::{
// 		migration::{storage_key_iter, take_storage_value},
// 		BoundedVec, Twox64Concat,
// 	};
// 	// Does not expose a hashmap
// 	use sp_std::{collections::btree_map, prelude::*};

// 	use crate::{
// 		AccountTaskId, AccountTasks, MissedQueueV2, MissedTaskV2, ScheduledTasksV2,
// 		Task, TaskId, TaskQueueV2, Vec,
// 	};

// 	// Use a double map for tasks (accountId, taskId)
// 	pub struct MigrateToV3<T>(PhantomData<T>);
// 	impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
// 		#[cfg(feature = "try-runtime")]
// 		fn pre_upgrade() -> Result<(), &'static str> {
// 			use frame_support::{migration::get_storage_value, traits::OnRuntimeUpgradeHelpersExt};

// 			let pallet_prefix: &[u8] = b"AutomationTime";

// 			// Get count of tasks
// 			let tasks_prefix: &[u8] = b"Tasks";
// 			let pre_task_count =
// 				storage_key_iter::<T::Hash, Task<T>, Twox64Concat>(pallet_prefix, tasks_prefix)
// 					.collect::<Vec<_>>()
// 					.len() as u32;
// 			Self::set_temp_storage::<u32>(pre_task_count, "pre_task_count");

// 			// Get count per scheduled time
// 			let scheduled_prefix: &[u8] = b"ScheduledTasks";
// 			let mut pre_scheduled_count: Vec<(UnixTime, u32)> = vec![];
// 			storage_key_iter::<u64, BoundedVec<T::Hash, T::MaxTasksPerSlot>, Twox64Concat>(
// 				pallet_prefix,
// 				scheduled_prefix,
// 			)
// 			.for_each(|(time, task_ids)| {
// 				pre_scheduled_count.push((time, task_ids.len() as u32));
// 			});
// 			Self::set_temp_storage::<Vec<(UnixTime, u32)>>(
// 				pre_scheduled_count,
// 				"pre_scheduled_count",
// 			);

// 			// Get count of tasks in task queue
// 			let task_queue_prefix: &[u8] = b"TaskQueue";
// 			let pre_task_queue_count =
// 				get_storage_value::<Vec<TaskId<T>>>(pallet_prefix, task_queue_prefix, &[])
// 					.unwrap_or(vec![])
// 					.len() as u32;
// 			Self::set_temp_storage::<u32>(pre_task_queue_count, "pre_task_queue_count");

// 			// Get count of tasks in missed queue
// 			let missed_queue_prefix: &[u8] = b"MissedQueue";
// 			let pre_missed_queue_count =
// 				get_storage_value::<Vec<MissedTask<T>>>(pallet_prefix, missed_queue_prefix, &[])
// 					.unwrap_or(vec![])
// 					.len() as u32;
// 			Self::set_temp_storage::<u32>(pre_missed_queue_count, "pre_missed_queue_count");

// 			log::info!(
// 				target: "automation-time",
// 				"migration: AutomationTime storage version v3 PRE migration checks succesful!"
// 			);

// 			Ok(())
// 		}

// 		fn on_runtime_upgrade() -> Weight {
// 			log::info!(target: "automation-time", "Migrating automation-time v3");
// 			let pallet_prefix: &[u8] = b"AutomationTime";

// 			// Move all tasks from Tasks to AccountTasks
// 			let old_tasks_prefix: &[u8] = b"Tasks";
// 			let old_tasks =
// 				storage_key_iter::<T::Hash, Task<T>, Twox64Concat>(pallet_prefix, old_tasks_prefix)
// 					.drain()
// 					.collect::<btree_map::BTreeMap<T::Hash, Task<T>>>();
// 			old_tasks.iter().for_each(|(task_id, task)| {
// 				AccountTasks::<T>::insert(task.owner_id.clone(), task_id, task.clone());
// 			});
// 			let task_migrated = old_tasks.len() as u64;

// 			// Move all tasks from ScheduledTasks to ScheduledTasksV2
// 			let mut times_migrated = 0u64;
// 			let old_scheduled_prefix: &[u8] = b"ScheduledTasks";
// 			storage_key_iter::<u64, BoundedVec<T::Hash, T::MaxTasksPerSlot>, Twox64Concat>(
// 				pallet_prefix,
// 				old_scheduled_prefix,
// 			)
// 			.drain()
// 			.for_each(|(time, task_ids)| {
// 				let new_task_ids = task_ids
// 					.into_iter()
// 					.filter_map(|task_id| {
// 						if let Some(task) = old_tasks.get(&task_id) {
// 							Some((task.owner_id.clone(), task_id))
// 						} else {
// 							log::debug!(target: "automation-time", "Unable to get task with id {:?}", task_id);
// 							None
// 						}
// 					})
// 					.collect::<Vec<_>>();

// 				let account_task_ids: BoundedVec<AccountTaskId<T>, T::MaxTasksPerSlot> =
// 					new_task_ids.try_into().unwrap();
// 				ScheduledTasksV2::<T>::insert(time, account_task_ids);
// 				times_migrated += 1;
// 			});

// 			// Move all tasks from TaskQueue to TaskQueueV2
// 			let old_task_queue_prefix: &[u8] = b"TaskQueue";
// 			let new_task_ids =
// 				take_storage_value::<Vec<TaskId<T>>>(pallet_prefix, old_task_queue_prefix, &[])
// 					.unwrap_or(vec![])
// 					.into_iter()
// 					.filter_map(|task_id| {
// 						if let Some(task) = old_tasks.get(&task_id) {
// 							Some((task.owner_id.clone(), task_id))
// 						} else {
// 							log::debug!(target: "automation-time", "Unable to get task with id {:?}", task_id);
// 							None
// 						}
// 					})
// 					.collect::<Vec<_>>();
// 			TaskQueueV2::<T>::put(new_task_ids);

// 			// Move all tasks from MissedQueue to MissedQueueV2, and convert from MissedTask to MissedTaskV2
// 			let old_missed_queue_prefix: &[u8] = b"MissedQueue";
// 			let new_missed_tasks = take_storage_value::<Vec<MissedTask<T>>>(
// 				pallet_prefix,
// 				old_missed_queue_prefix,
// 				&[],
// 			)
// 			.unwrap_or(vec![])
// 			.into_iter()
// 			.filter_map(|missed_task| {
// 				if let Some(task) = old_tasks.get(&missed_task.task_id) {
// 					Some(MissedTaskV2::<T>::create_missed_task(
// 						task.owner_id.clone(),
// 						missed_task.task_id,
// 						missed_task.execution_time,
// 					))
// 				} else {
// 					log::debug!(target: "automation-time", "Unable to get task with id {:?}", missed_task.task_id);
// 					None
// 				}
// 			})
// 			.collect::<Vec<_>>();
// 			MissedQueueV2::<T>::put(new_missed_tasks);

// 			// For each task there is
// 			// 1 read to get it into memory from Tasks
// 			// 1 write to remove it from Tasks
// 			// 1 write to add it to AccountTasks
// 			let weight = T::DbWeight::get().reads_writes(task_migrated + 1, task_migrated * 2);

// 			// For each time in scheduled tasks there is
// 			// 1 read to get it into memory
// 			// 1 write to remove it from the old scheduled tasks
// 			// 1 write to add it to the new scheduled tasks
// 			let weight =
// 				weight + T::DbWeight::get().reads_writes(times_migrated + 1, times_migrated * 2);

// 			// For the task queue there is
// 			// 1 read to get it into memory
// 			// 1 write to remove it from the old task queue
// 			// 1 write to add it to the new task queue
// 			let weight = weight + T::DbWeight::get().reads_writes(1, 2);

// 			// For the mised queue there is
// 			// 1 read to get it into memory
// 			// 1 write to remove it from the old task queue
// 			// 1 write to add it to the new task queue
// 			let weight = weight + T::DbWeight::get().reads_writes(1, 2);

// 			// Adding a buffer for the rest of the code
// 			weight + 100_000_000
// 		}

// 		#[cfg(feature = "try-runtime")]
// 		fn post_upgrade() -> Result<(), &'static str> {
// 			use frame_support::{migration::get_storage_value, traits::OnRuntimeUpgradeHelpersExt};

// 			let pallet_prefix: &[u8] = b"AutomationTime";

// 			// Task count should not have changed
// 			let tasks_prefix: &[u8] = b"AccountTasks";
// 			let post_task_count =
// 				storage_key_iter::<T::Hash, Task<T>, Twox64Concat>(pallet_prefix, tasks_prefix)
// 					.collect::<Vec<_>>()
// 					.len() as u32;
// 			let pre_task_count = Self::get_temp_storage::<u32>("pre_task_count").unwrap();
// 			assert_eq!(pre_task_count, post_task_count);

// 			// Tasks count per scheduled time should not have changed
// 			let scheduled_prefix: &[u8] = b"ScheduledTasksV2";
// 			let mut post_scheduled_count: Vec<(UnixTime, u32)> = vec![];
// 			storage_key_iter::<u64, BoundedVec<AccountTaskId<T>, T::MaxTasksPerSlot>, Twox64Concat>(
// 				pallet_prefix,
// 				scheduled_prefix,
// 			).for_each(|(time, task_ids)| {
// 				post_scheduled_count.push((time, task_ids.len() as u32));
// 			});
// 			let pre_scheduled_count =
// 				Self::get_temp_storage::<Vec<(UnixTime, u32)>>("pre_scheduled_count").unwrap();
// 			assert_eq!(post_scheduled_count, pre_scheduled_count);

// 			// Number of tasks in task queue should not have changed
// 			let task_queue_prefix: &[u8] = b"TaskQueueV2";
// 			let post_task_queue_count =
// 				get_storage_value::<Vec<TaskId<T>>>(pallet_prefix, task_queue_prefix, &[])
// 					.unwrap_or(vec![])
// 					.len() as u32;
// 			let pre_task_queue_count =
// 				Self::get_temp_storage::<u32>("pre_task_queue_count").unwrap();
// 			assert_eq!(post_task_queue_count, pre_task_queue_count);

// 			// Number of tasks in missed queue should not have changed
// 			let missed_queue_prefix: &[u8] = b"MissedQueueV2";
// 			let post_missed_queue_count =
// 				get_storage_value::<Vec<MissedTaskV2<T>>>(pallet_prefix, missed_queue_prefix, &[])
// 					.unwrap_or(vec![])
// 					.len() as u32;
// 			let pre_missed_queue_count =
// 				Self::get_temp_storage::<u32>("pre_missed_queue_count").unwrap();
// 			assert_eq!(post_missed_queue_count, pre_missed_queue_count);

// 			log::info!(
// 				target: "automation-time",
// 				"migration: AutomationTime storage version v3 POST migration checks succesful! Migrated {} tasks.",
// 				post_task_count
// 			);

// 			Ok(())
// 		}
// 	}
// }

pub mod v4 {
	use super::*;
	use crate::{AccountTaskId, ScheduledTasks};
	use frame_support::{migration::storage_key_iter, BoundedVec, Twox64Concat};

	pub const OLD_STORAGE_PREFIX: &[u8] = b"ScheduledTasksV2";
	pub const NEW_STORAGE_PREFIX: &[u8] = b"ScheduledTasksV3";

	// Use weight to determine when scheduled task slot is full
	pub struct MigrateToV4<T>(PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV4<T> {
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			use frame_support::traits::OnRuntimeUpgradeHelpersExt;

			// Get count per scheduled time
			let mut pre_scheduled_count: Vec<(UnixTime, u32)> = vec![];
			storage_key_iter::<u64, BoundedVec<AccountTaskId<T>, T::MaxTasksPerSlot>, Twox64Concat>(
				PALLET_PREFIX,
				OLD_STORAGE_PREFIX,
			)
			.for_each(|(time, task_ids)| {
				pre_scheduled_count.push((time, task_ids.len() as u32));
			});
			Self::set_temp_storage::<Vec<(UnixTime, u32)>>(
				pre_scheduled_count,
				"pre_scheduled_count",
			);

			log::info!(
				target: "automation-time",
				"migration: AutomationTime storage version v4 PRE migration checks succesful!"
			);

			Ok(())
		}

		fn on_runtime_upgrade() -> Weight {
			log::info!(target: "automation-time", "Migrating automation-time v4");

			// Move all tasks from ScheduledTasksV2 to ScheduledTasksV3
			let mut task_count = 0u64;
			let mut migrated_keys_count = 0u64;
			storage_key_iter::<UnixTime, BoundedVec<AccountTaskId<T>, T::MaxTasksPerSlot>, Twox64Concat>(
				PALLET_PREFIX,
				OLD_STORAGE_PREFIX,
			)
			.drain()
			.for_each(|(time, task_ids)| {
				migrated_keys_count += 1;
				let weight = task_ids.clone()
					.into_iter()
					.fold(0u128, |acc, (account_id, task_id)| {
						task_count += 1;
						if let Some(task) = Pallet::<T>::get_account_task(account_id.clone(), task_id) {
							acc + task.action.execution_weight::<T>() as u128
						} else {
							log::warn!(target: "automation-time", "Unable to get task with id {:?} for account {:?}", task_id, account_id);
							acc
						}
					});

				// Insert all tasks without checking MaxWeightPerSlot to allow existing tasks to be run
				ScheduledTasksV3::<T>::insert(time, ScheduledTasks{ tasks: task_ids.to_vec(), weight });
			});

			// 1 read to load iterator
			// task_count reads to load tasks
			// For each time (migrated_key_count) in scheduled tasks there is
			// 1 read to get it into memory
			// 1 write to remove it from the old scheduled tasks
			// 1 write to add it to the new scheduled tasks
			let weight = T::DbWeight::get()
				.reads_writes(migrated_keys_count + task_count + 1, migrated_keys_count * 2);

			// Adding a buffer for the rest of the code
			weight + 10_000_000 + (migrated_keys_count * 15_000_000)
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			use frame_support::traits::OnRuntimeUpgradeHelpersExt;

			// Tasks count per scheduled time should not have changed
			let mut post_scheduled_count: Vec<(UnixTime, u32)> = vec![];
			storage_key_iter::<UnixTime, ScheduledTasksOf<T>, Twox64Concat>(
				PALLET_PREFIX,
				NEW_STORAGE_PREFIX,
			)
			.for_each(|(time, ScheduledTasks { tasks: task_ids, weight })| {
				post_scheduled_count.push((time, task_ids.len() as u32));
				assert!(weight > 0);
				if weight > T::MaxWeightPerSlot::get() {
					log::info!(
						target: "automation-time",
						"ScheduledTasks at {} are over the weight limit.",
						time
					);
				}
			});
			let pre_scheduled_count =
				Self::get_temp_storage::<Vec<(UnixTime, u32)>>("pre_scheduled_count").unwrap();
			assert_eq!(post_scheduled_count, pre_scheduled_count);

			log::info!(
				target: "automation-time",
				"migration: AutomationTime storage version v4 POST migration checks succesful! Migrated {} tasks.",
				post_scheduled_count.iter().map(|(_, count)| count).sum::<u32>()
			);

			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;

	mod migrations_v4 {
		use super::*;
		use frame_support::{
			migration::{put_storage_value, storage_key_iter},
			Hashable,
		};
		use sp_runtime::AccountId32;

		#[test]
		fn on_runtime_upgrade() {
			new_test_ext(0).execute_with(|| {
				let account_id = AccountId32::new(ALICE);
				let task_id_1 = Pallet::<Test>::generate_task_id(account_id.clone(), vec![1]);
				let task_id_2 = Pallet::<Test>::generate_task_id(account_id.clone(), vec![2]);
				let task_1 = TaskOf::<Test>::create_event_task(
					account_id.clone(),
					vec![1],
					vec![1].try_into().unwrap(),
					vec![1],
				);
				let task_2 = TaskOf::<Test>::create_event_task(
					account_id.clone(),
					vec![2],
					vec![2].try_into().unwrap(),
					vec![2],
				);
				AccountTasks::<Test>::insert(account_id.clone(), task_id_1, task_1);
				AccountTasks::<Test>::insert(account_id.clone(), task_id_2, task_2);
				let partial_tasks: BoundedVec<_, MaxTasksPerSlot> =
					vec![(account_id.clone(), task_id_1)].try_into().unwrap();
				let all_tasks: BoundedVec<_, ConstU32<256>> =
					vec![(account_id.clone(), task_id_1), (account_id.clone(), task_id_2)]
						.try_into()
						.unwrap();
				put_storage_value(
					PALLET_PREFIX,
					v4::OLD_STORAGE_PREFIX,
					&0u64.twox_64_concat(),
					partial_tasks,
				);
				put_storage_value(
					PALLET_PREFIX,
					v4::OLD_STORAGE_PREFIX,
					&1u64.twox_64_concat(),
					all_tasks,
				);
				v4::MigrateToV4::<Test>::on_runtime_upgrade();
				assert_eq!(
					None,
					storage_key_iter::<
						UnixTime,
						BoundedVec<AccountTaskId<Test>, MaxTasksPerSlot>,
						Twox64Concat,
					>(PALLET_PREFIX, v4::OLD_STORAGE_PREFIX,)
					.next()
				);
				assert_eq!(
					Pallet::<Test>::get_scheduled_tasks(0).unwrap(),
					ScheduledTasks { tasks: vec![(account_id.clone(), task_id_1)], weight: 20_000 }
				);
				assert_eq!(
					Pallet::<Test>::get_scheduled_tasks(1).unwrap(),
					ScheduledTasks {
						tasks: vec![
							(account_id.clone(), task_id_1),
							(account_id.clone(), task_id_2)
						],
						weight: 40_000
					}
				);
			})
		}
	}
}
