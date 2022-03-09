use crate::{Config, Weight};
use frame_support::traits::Get;
use log::info;

pub mod v1 {
	use frame_support::{migration::get_storage_value};

  use crate::{LastTimeSlot, AutomationTimeStorageVersion};

use super::*;

	pub fn migrate<T: Config>() -> Weight {
		info!(target: "automation-time", "Migrating automation-time v1");
    let pallet_prefix: &[u8] = b"AutomationTime";
		let storage_item_prefix: &[u8] = b"LastTimeSlot";

		let stored_data = get_storage_value::<u64>(
			pallet_prefix,
			storage_item_prefix,
      &[],
		).expect("Must have last slot value");

    // TODO: (jzhou) Gate migration on not having storage version
    // TODO: (jzhou) What if stored_data is None?
    let storage_version = AutomationTimeStorageVersion::<T>::get();
    if storage_version == None || storage_version < Some(1) {
      LastTimeSlot::<T>::put((stored_data, stored_data));
      info!(target: "automation-time", "Completed automation-time migration to v1");
      AutomationTimeStorageVersion::<T>::put(1);
      T::DbWeight::get().reads_writes(1, 1)
    } else {
      info!("migration already run before");
      0
    }
	}
}
