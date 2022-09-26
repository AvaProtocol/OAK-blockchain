pub const PALLET_PREFIX: &[u8] = b"AutomationTime";

use crate::{
	weights::WeightInfo, AccountTaskId, Config, Pallet, ScheduledTasks, ScheduledTasksOf,
	ScheduledTasksV3, UnixTime, Weight,
};
use frame_support::{
	migration::storage_key_iter,
	pallet_prelude::PhantomData,
	traits::{Get, OnRuntimeUpgrade},
	BoundedVec, Twox64Concat,
};

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
		Self::set_temp_storage::<Vec<(UnixTime, u32)>>(pre_scheduled_count, "pre_scheduled_count");

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
							acc + task.action.execution_weight::<T>().unwrap_or(<T as Config>::WeightInfo::run_dynamic_dispatch_action_fail_decode()) as u128
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;

	use crate::{AccountTasks, TaskOf};
	use frame_support::{
		migration::{put_storage_value, storage_key_iter},
		Hashable,
	};
	use sp_runtime::{traits::ConstU32, AccountId32};

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
				OLD_STORAGE_PREFIX,
				&0u64.twox_64_concat(),
				partial_tasks,
			);
			put_storage_value(PALLET_PREFIX, OLD_STORAGE_PREFIX, &1u64.twox_64_concat(), all_tasks);
			MigrateToV4::<Test>::on_runtime_upgrade();
			assert_eq!(
				None,
				storage_key_iter::<
					UnixTime,
					BoundedVec<AccountTaskId<Test>, MaxTasksPerSlot>,
					Twox64Concat,
				>(PALLET_PREFIX, OLD_STORAGE_PREFIX,)
				.next()
			);
			assert_eq!(
				Pallet::<Test>::get_scheduled_tasks(0).unwrap(),
				ScheduledTasks { tasks: vec![(account_id.clone(), task_id_1)], weight: 20_000 }
			);
			assert_eq!(
				Pallet::<Test>::get_scheduled_tasks(1).unwrap(),
				ScheduledTasks {
					tasks: vec![(account_id.clone(), task_id_1), (account_id.clone(), task_id_2)],
					weight: 40_000
				}
			);
		})
	}
}
