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
