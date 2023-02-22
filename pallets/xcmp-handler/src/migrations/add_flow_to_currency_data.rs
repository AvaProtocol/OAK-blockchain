use core::marker::PhantomData;

use codec::{Decode, Encode};
use frame_support::{
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
	Twox64Concat,
};
use scale_info::TypeInfo;

use crate::{Config, XcmCurrencyData, XcmFlow};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Pre-migrations storage struct
#[derive(Clone, Copy, Debug, Encode, Decode, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OldXcmCurrencyData {
	pub native: bool,
	pub fee_per_second: u128,
	pub instruction_weight: u64,
}

impl From<OldXcmCurrencyData> for XcmCurrencyData {
	fn from(data: OldXcmCurrencyData) -> Self {
		XcmCurrencyData {
			native: data.native,
			fee_per_second: data.fee_per_second,
			instruction_weight: data.instruction_weight,
			flow: XcmFlow::Normal,
		}
	}
}

#[frame_support::storage_alias]
pub type XcmChainCurrencyData<T: Config> = StorageDoubleMap<
	XcmpHandler,
	Twox64Concat,
	u32,
	Twox64Concat,
	<T as Config>::CurrencyId,
	OldXcmCurrencyData,
>;

pub struct AddFlowToCurrencyData<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for AddFlowToCurrencyData<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "xcmp-handler", "AddFlowToCurrencyData migration");

		let migrated_count = XcmChainCurrencyData::<T>::iter()
			.map(|(parachain_id, currency_id, xcm_data)| {
				let migrated_data = XcmCurrencyData::from(xcm_data);
				crate::XcmChainCurrencyData::<T>::insert(parachain_id, currency_id, migrated_data);
				log::info!(target: "xcmp-handler", "AddFlowToCurrencyData migrated para_id: {}", parachain_id);
				migrated_data
			})
			.count();

		log::info!(target: "xcmp-handler", "AddFlowToCurrencyData successful! Migrated {} object.", migrated_count);

		T::DbWeight::get().reads_writes(migrated_count as u64, migrated_count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::OnRuntimeUpgradeHelpersExt;

		let count = XcmChainCurrencyData::<T>::iter().count();
		Self::set_temp_storage::<u32>(count as u32, "pre_migration_xcm_data_count");

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::OnRuntimeUpgradeHelpersExt;

		let post_count = crate::XcmChainCurrencyData::<T>::iter().count() as u32;
		let pre_count = Self::get_temp_storage::<u32>("pre_migration_xcm_data_count").unwrap();

		assert_eq!(post_count, pre_count);

		log::info!(
			target: "xcmp-handler",
			"AddFlowToCurrencyData try-runtime checks complete"
		);

		Ok(())
	}
}
