use crate::*;
use frame_support::{pallet_prelude::Weight, traits::OnRuntimeUpgrade};
use orml_asset_registry::AssetMetadata;
use primitives::assets::CustomMetadata;

pub type AssetMetadataOf = AssetMetadata<Balance, CustomMetadata>;

pub fn dollar(decimals: u32) -> Balance {
	10_u128.pow(decimals)
}

pub fn cent(decimals: u32) -> Balance {
	dollar(decimals) / 100
}

#[frame_support::storage_alias]
type LastAssetId<T: Config> =
	StorageValue<AssetRegistry, <T as orml_asset_registry::Config>::AssetId>;

pub struct AddShidenAsset;
impl OnRuntimeUpgrade for AddShidenAsset {
	fn on_runtime_upgrade() -> Weight {
		log::info!(
			target: "asset_registry",
			"on_runtime_upgrade: add shiden asset"
		);

		let asset = AssetMetadataOf {
			decimals: 18,
			name: b"Shiden".to_vec(),
			symbol: b"SDN".to_vec(),
			additional: CustomMetadata {
				fee_per_second: Some(416_000_000_000),
				conversion_rate: None,
			},
			existential_deposit: cent(18),
			location: Some(MultiLocation::new(1, X1(Parachain(2007))).into()),
		};

		let _ = orml_asset_registry::Pallet::<Runtime>::do_register_asset(asset.clone(), None)
			.map_err(|e| {
				log::error!(
						target: "asset_registry",
						"failed to register shiden native token with err: {:?}", e
				)
			});

		log::info!(
			target: "asset_registry",
			"on_runtime_upgrade: complete"
		);

		// Storage: AssetRegistry Metadata (r:1 w:1)
		// Storage: AssetRegistry LocationToAssetId (r:1 w:1)
		// Storage: AssetRegistry LastAssetId (r:1 w:1)
		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(3u64, 3u64)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::OnRuntimeUpgradeHelpersExt;

		log::info!(
			target: "asset_registry",
			"pre_upgrade check"
		);

		let asset_metadata_count =
			orml_asset_registry::Metadata::<Runtime>::iter().collect::<Vec<_>>().len();
		Self::set_temp_storage::<u32>(
			asset_metadata_count.try_into().unwrap(),
			"pre_asset_metadata_count",
		);

		let location_to_asset_id_count = orml_asset_registry::LocationToAssetId::<Runtime>::iter()
			.collect::<Vec<_>>()
			.len();
		Self::set_temp_storage::<u32>(
			location_to_asset_id_count.try_into().unwrap(),
			"pre_location_to_asset_id_count",
		);

		let last_asset_id = LastAssetId::<Runtime>::get().unwrap();
		Self::set_temp_storage::<u32>(last_asset_id, "pre_last_asset_id");

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::OnRuntimeUpgradeHelpersExt;

		log::info!(
			target: "asset_registry",
			"post_upgrade check"
		);

		let asset_metadata_count =
			orml_asset_registry::Metadata::<Runtime>::iter().collect::<Vec<_>>().len();
		let pre_asset_metadata_count =
			Self::get_temp_storage::<u32>("pre_asset_metadata_count").unwrap();
		assert_eq!(pre_asset_metadata_count + 1, asset_metadata_count as u32);

		let location_to_asset_id_count = orml_asset_registry::LocationToAssetId::<Runtime>::iter()
			.collect::<Vec<_>>()
			.len();
		let pre_location_to_asset_id_count =
			Self::get_temp_storage::<u32>("pre_location_to_asset_id_count").unwrap();
		assert_eq!(pre_location_to_asset_id_count + 1, location_to_asset_id_count as u32);

		let last_asset_id = LastAssetId::<Runtime>::get().unwrap();
		let pre_last_asset_id = Self::get_temp_storage::<u32>("pre_last_asset_id").unwrap();
		assert_eq!(pre_last_asset_id + 1, last_asset_id);

		log::info!(target: "asset_registry", "added shiden native token to asset registry; last asset id: {:?}", last_asset_id);

		Ok(())
	}
}
