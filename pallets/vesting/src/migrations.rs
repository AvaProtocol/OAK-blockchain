use crate::{BalanceOf, Config, TotalUnvestedAllocation, VestingSchedule};
use frame_support::{pallet_prelude::Weight, traits::Get};
use log::info;
use sp_runtime::traits::{Saturating, Zero};

pub fn set_total_unvested_allocation<T: Config>() -> Weight {
	info!(target: "vesting", "Setting TotalUnvestedAllocation");

	let mut reads = 0;
	let unvested_funds = VestingSchedule::<T>::iter()
		.map(|(_, vests)| {
			reads = reads.saturating_add(1);
			vests
		})
		.flatten()
		.fold(Zero::zero(), |acc: BalanceOf<T>, (_, amount)| acc.saturating_add(amount));

	TotalUnvestedAllocation::<T>::set(unvested_funds);

	T::DbWeight::get().reads_writes(reads, 1)
}
