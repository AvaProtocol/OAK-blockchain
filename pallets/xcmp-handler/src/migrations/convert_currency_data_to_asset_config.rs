use core::marker::PhantomData;

use codec::{Decode, Encode};
use frame_support::{
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
	Twox64Concat,
};
use scale_info::TypeInfo;
use xcm::latest::prelude::*;

use crate::{Config, XcmAssetConfig, XcmFlow};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Pre-migrations storage struct
#[derive(Clone, Copy, Debug, Encode, Decode, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OldXcmCurrencyData {
	pub native: bool,
	pub fee_per_second: u128,
	pub instruction_weight: u64,
	pub flow: XcmFlow,
}

impl From<OldXcmCurrencyData> for XcmAssetConfig {
	fn from(data: OldXcmCurrencyData) -> Self {
		XcmAssetConfig {
			fee_per_second: data.fee_per_second,
			instruction_weight: data.instruction_weight,
			flow: data.flow,
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

pub struct ConvertCurrencyDataToAssetConfig<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for ConvertCurrencyDataToAssetConfig<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "xcmp-handler", "ConvertCurrencyDataToAssetConfig migration");

		let migrated_count = XcmChainCurrencyData::<T>::iter()
			.map(|(parachain_id, _currency_id, xcm_data)| {
				let migrated_data = XcmAssetConfig::from(xcm_data);
				crate::DestinationAssetConfig::<T>::insert(
					MultiLocation::new(1, X1(Parachain(parachain_id))),
					migrated_data.clone(),
				);
				log::info!(target: "xcmp-handler", "ConvertCurrencyDataToAssetConfig migrated para_id: {}", parachain_id);
				migrated_data
			})
			.count();

		log::info!(target: "xcmp-handler", "ConvertCurrencyDataToAssetConfig successful! Migrated {} object.", migrated_count);

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

		let post_count = crate::DestinationAssetConfig::<T>::iter().count() as u32;
		let pre_count = Self::get_temp_storage::<u32>("pre_migration_xcm_data_count").unwrap();

		assert_eq!(post_count, pre_count);

		log::info!(
			target: "xcmp-handler",
			"ConvertCurrencyDataToAssetConfig try-runtime checks complete"
		);

		Ok(())
	}
}
