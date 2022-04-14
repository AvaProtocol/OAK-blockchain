use crate::{Config, Weight};
use frame_support::traits::Get;
use log::info;

pub mod v1 {
	use frame_support::{migration::get_storage_value, traits::StorageVersion};

	use crate::{LastTimeSlot, Pallet};

	use super::*;

	pub fn migrate<T: Config>() -> Weight {
		info!(target: "automation-time", "Migrating automation-time v1");
		let pallet_prefix: &[u8] = b"AutomationTime";
		let storage_item_prefix: &[u8] = b"LastTimeSlot";

		let stored_data = get_storage_value::<u64>(pallet_prefix, storage_item_prefix, &[])
			.expect("Must have last slot value");

		LastTimeSlot::<T>::put((stored_data, stored_data));
		info!(target: "automation-time", "Completed automation-time migration to v1");
		StorageVersion::new(1).put::<Pallet<T>>();
		T::DbWeight::get().reads_writes(1, 1)
	}
}

pub mod v2 {
	use frame_support::{migration::{storage_iter, storage_key_iter, get_storage_value}, traits::StorageVersion, Twox64Concat, BoundedVec};

	use crate::{LastTimeSlot, Pallet, Task, Vec};

	use super::*;

	pub fn migrate<T: Config>() -> Weight {
		info!(target: "automation-time", "Migrating automation-time v2");
		let pallet_prefix: &[u8] = b"AutomationTime";

		let scheduled_tasks_prefix: &[u8] = b"ScheduledTasks";
		let _scheduled_tasks: Vec<_> = storage_key_iter::<u64, BoundedVec<T::Hash, T::MaxTasksPerSlot>, Twox64Concat>(pallet_prefix, scheduled_tasks_prefix)
			.drain()
			.collect();
		let missed_queue_prefix: &[u8] = b"MissedQueue";
		let _missed_tasks: Vec<_> = storage_iter::<T::Hash>(pallet_prefix, missed_queue_prefix)
			.drain()
			.collect();
		let task_queue_prefix: &[u8] = b"TaskQueue";
		let _running_tasks: Vec<_> = storage_iter::<T::Hash>(pallet_prefix, task_queue_prefix)
			.drain()
			.collect();
		let tasks_prefix: &[u8] = b"Tasks";
		let _tasks: Vec<_> = storage_key_iter::<T::Hash, Task<T>, Twox64Concat>(pallet_prefix, tasks_prefix)
			.drain()
			.collect();

		let pallet_prefix: &[u8] = b"AutomationTime";
		let storage_item_prefix: &[u8] = b"LastTimeSlot";
		let stored_min_slot = get_storage_value::<u64>(pallet_prefix, storage_item_prefix, &[])
			.expect("Must have last slot value");
		let diff_to_hour = stored_min_slot % 3600;
		let current_hour_slot = stored_min_slot.saturating_sub(diff_to_hour);
		LastTimeSlot::<T>::put((current_hour_slot, current_hour_slot));

		info!(target: "automation-time", "Completed automation-time migration to v2");
		StorageVersion::new(2).put::<Pallet<T>>();
		T::DbWeight::get().reads_writes(4, 4)
	}
}
