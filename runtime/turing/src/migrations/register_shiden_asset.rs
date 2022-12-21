use crate::*;
use frame_support::{pallet_prelude::Weight, traits::OnRuntimeUpgrade};
use orml_asset_registry::AssetMetadata;
use primitives::assets::{ConversionRate, CustomMetadata};

pub type AssetMetadataOf = AssetMetadata<Balance, CustomMetadata>;

pub fn dollar(decimals: u32) -> Balance {
	10_u128.pow(decimals)
}

pub fn cent(decimals: u32) -> Balance {
	dollar(decimals) / 100
}
