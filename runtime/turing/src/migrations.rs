use super::*;
use frame_support::traits::OnRuntimeUpgrade;

pub mod asset_registry {
	use super::*;
	use frame_support::{Twox64Concat, WeakBoundedVec};
	use orml_asset_registry::AssetMetadata;

	pub type AssetMetadataOf = AssetMetadata<Balance, CustomMetadata>;

	pub mod parachains {
		pub mod testchain {
			pub const ID: u32 = 1999;
		}

		pub mod heiko {
			pub const ID: u32 = 2085;
			pub const HKO_KEY: &[u8] = b"HKO";
			pub const SKSM_KEY: &[u8] = b"sKSM";
		}

		pub mod karura {
			pub const ID: u32 = 2000;
			pub const KAR_KEY: &[u8] = &[0, 128];
			pub const AUSD_KEY: &[u8] = &[0, 129];
			pub const LKSM_KEY: &[u8] = &[0, 131];
		}

		pub mod khala {
			pub const ID: u32 = 2004;
		}
	}

	pub fn dollar(decimals: u32) -> Balance {
		10_u128.pow(decimals)
	}

	pub fn cent(decimals: u32) -> Balance {
		dollar(decimals) / 100
	}

	pub fn millicent(decimals: u32) -> Balance {
		cent(decimals) / 1_000
	}

	/// Based on the precedent set by other projects. This will need to be changed.
	pub fn ksm_per_second() -> u128 {
		cent(12) * 16
	}

	/// Assuming ~ $0.50 TUR price.
	pub fn tur_per_second() -> u128 {
		let ksm_decimals: u32 = 12;
		let tur_decimals: u32 = 10;
		let diff = ksm_decimals - tur_decimals;
		let tur_equivalent = ksm_per_second() / 10_u128.pow(diff);

		// Assuming KSM ~ $130.00.
		tur_equivalent * 260
	}

	pub struct AssetRegistryMigration;
	impl OnRuntimeUpgrade for AssetRegistryMigration {
		fn on_runtime_upgrade() -> Weight {
			log::info!(
				target: "asset_registry",
				"on_runtime_upgrade: Attempted to apply asset_registry migration"
			);

			let assets = vec![
				(
					0,
					AssetMetadataOf {
						decimals: 10,
						name: b"Native".to_vec(),
						symbol: b"TUR".to_vec(),
						additional: Default::default(),
						existential_deposit: EXISTENTIAL_DEPOSIT,
						location: Some(MultiLocation::new(0, Here).into()),
					},
				),
				(
					1,
					AssetMetadataOf {
						decimals: 12,
						name: b"KSM".to_vec(),
						symbol: b"Kusama".to_vec(),
						additional: Default::default(),
						existential_deposit: 10 * millicent(12),
						location: Some(MultiLocation::parent().into()),
					},
				),
				(
					2,
					AssetMetadataOf {
						decimals: 12,
						name: b"AUSD".to_vec(),
						symbol: b"AUSD".to_vec(),
						additional: Default::default(),
						existential_deposit: cent(12),
						location: Some(
							MultiLocation::new(
								1,
								X2(
									Parachain(parachains::karura::ID),
									GeneralKey(WeakBoundedVec::<u8, ConstU32<32>>::force_from(
										parachains::karura::AUSD_KEY.to_vec(),
										None,
									)),
								),
							)
							.into(),
						),
					},
				),
				(
					3,
					AssetMetadataOf {
						decimals: 12,
						name: b"Karura".to_vec(),
						symbol: b"KAR".to_vec(),
						additional: Default::default(),
						existential_deposit: 10 * cent(12),
						location: Some(
							MultiLocation::new(
								1,
								X2(
									Parachain(parachains::karura::ID),
									GeneralKey(WeakBoundedVec::<u8, ConstU32<32>>::force_from(
										parachains::karura::KAR_KEY.to_vec(),
										None,
									)),
								),
							)
							.into(),
						),
					},
				),
				(
					4,
					AssetMetadataOf {
						decimals: 12,
						name: b"Liquid KSM".to_vec(),
						symbol: b"LKSM".to_vec(),
						additional: Default::default(),
						existential_deposit: 50 * millicent(12),
						location: Some(
							MultiLocation::new(
								1,
								X2(
									Parachain(parachains::karura::ID),
									GeneralKey(WeakBoundedVec::<u8, ConstU32<32>>::force_from(
										parachains::karura::LKSM_KEY.to_vec(),
										None,
									)),
								),
							)
							.into(),
						),
					},
				),
				(
					5,
					AssetMetadataOf {
						decimals: 12,
						name: b"Heiko".to_vec(),
						symbol: b"HKO".to_vec(),
						additional: Default::default(),
						existential_deposit: 50 * cent(12),
						location: Some(
							MultiLocation::new(
								1,
								X2(
									Parachain(parachains::heiko::ID),
									GeneralKey(WeakBoundedVec::<u8, ConstU32<32>>::force_from(
										parachains::heiko::HKO_KEY.to_vec(),
										None,
									)),
								),
							)
							.into(),
						),
					},
				),
				(
					6,
					AssetMetadataOf {
						decimals: 12,
						name: b"SKSM".to_vec(),
						symbol: b"SKSM".to_vec(),
						additional: Default::default(),
						existential_deposit: 50 * millicent(12),
						location: Some(
							MultiLocation::new(
								1,
								X2(
									Parachain(parachains::heiko::ID),
									GeneralKey(WeakBoundedVec::<u8, ConstU32<32>>::force_from(
										parachains::heiko::SKSM_KEY.to_vec(),
										None,
									)),
								),
							)
							.into(),
						),
					},
				),
				(
					7,
					AssetMetadataOf {
						decimals: 12,
						name: b"PHA".to_vec(),
						symbol: b"PHA".to_vec(),
						additional: Default::default(),
						existential_deposit: cent(12),
						location: Some(
							MultiLocation::new(1, X1(Parachain(parachains::khala::ID))).into(),
						),
					},
				),
				(
					8,
					AssetMetadataOf {
						decimals: 12,
						name: b"UNIT".to_vec(),
						symbol: b"UNIT".to_vec(),
						additional: CustomMetadata {
							fee_per_second: Some(tur_per_second()),
							conversion_rate: Some(1),
						},
						existential_deposit: 10 * millicent(12),
						location: Some(
							MultiLocation::new(1, X1(Parachain(parachains::testchain::ID))).into(),
						),
					},
				),
			];

			use frame_support::migration::put_storage_value;

			// Insert new data
			for (id, metadata) in assets.iter() {
				orml_asset_registry::Pallet::<Runtime>::do_register_asset_without_asset_processor(
					metadata.clone(),
					*id,
				)
				.expect("should not fail");
			}

			// Set LastAssetId - zero index
			let pallet_prefix: &[u8] = b"AssetRegistry";
			let last_asset_id_prefix: &[u8] = b"LastAssetId";
			let last_asset_id: TokenId = (assets.len() - 1).try_into().unwrap();
			put_storage_value::<TokenId>(pallet_prefix, last_asset_id_prefix, &[], last_asset_id);

			log::info!(
				target: "asset_registry",
				"on_runtime_upgrade: New data inserted"
			);

			// Each asset + each asset location + updating last asset id
			let total_rw = assets.len() as u32 * 2 + 1;
			<Runtime as frame_system::Config>::DbWeight::get()
				.reads_writes(total_rw as Weight, total_rw as Weight)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			log::info!(
				target: "asset_registry",
				"pre_upgrade check"
			);

			use frame_support::{
				migration::{get_storage_value, storage_key_iter},
				traits::OnRuntimeUpgradeHelpersExt,
			};

			let pallet_prefix: &[u8] = b"AssetRegistry";
			let metadata_prefix: &[u8] = b"Metadata";
			let location_to_asset_id_prefix: &[u8] = b"LocationToAssetId";
			let last_asset_id_prefix: &[u8] = b"LastAssetId";

			// Assert Metadata length is 0
			let metadata = storage_key_iter::<TokenId, AssetMetadataOf, Twox64Concat>(
				pallet_prefix,
				metadata_prefix,
			)
			.collect::<Vec<_>>();
			assert_eq!(metadata.len(), 0);

			// Assert LocationToAssetId length is 0
			let location_to_asset_id = storage_key_iter::<MultiLocation, TokenId, Twox64Concat>(
				pallet_prefix,
				location_to_asset_id_prefix,
			)
			.collect::<Vec<_>>();
			assert_eq!(location_to_asset_id.len(), 0);

			// Assert last asset id is 0
			let last_asset_id =
				get_storage_value::<TokenId>(pallet_prefix, last_asset_id_prefix, &[]).unwrap_or(0);
			assert_eq!(last_asset_id, 0);

			// Get tokens total issuance for comparing after migration
			let pallet_prefix: &[u8] = b"Tokens";
			let total_issuance_prefix: &[u8] = b"TotalIssuance";
			let mut pre_tokens_total_issuance: Vec<(u32, u128)> = vec![];
			storage_key_iter::<CurrencyId, u128, Twox64Concat>(
				pallet_prefix,
				total_issuance_prefix,
			)
			.for_each(|(currency_id, balance)| {
				pre_tokens_total_issuance.push((currency_id as u32, balance));
			});
			Self::set_temp_storage::<Vec<(u32, u128)>>(
				pre_tokens_total_issuance.clone(),
				"pre_tokens_total_issuance",
			);
			log::info!(
				target: "asset_registry",
				"pre_upgrade tokens total issuance {}",
				pre_tokens_total_issuance.len(),
			);

			Ok(())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			log::info!(
				target: "asset_registry",
				"post_upgrade check"
			);

			use frame_support::{
				migration::{get_storage_value, storage_key_iter},
				traits::OnRuntimeUpgradeHelpersExt,
			};

			let pallet_prefix: &[u8] = b"AssetRegistry";
			let metadata_prefix: &[u8] = b"Metadata";
			let location_to_asset_id_prefix: &[u8] = b"LocationToAssetId";
			let last_asset_id_prefix: &[u8] = b"LastAssetId";

			// Assert Metadata length
			let metadata = storage_key_iter::<TokenId, AssetMetadataOf, Twox64Concat>(
				pallet_prefix,
				metadata_prefix,
			)
			.collect::<Vec<_>>();
			assert_eq!(metadata.len(), 9);

			// Assert LocationToAssetId length
			let location_to_asset_id = storage_key_iter::<MultiLocation, TokenId, Twox64Concat>(
				pallet_prefix,
				location_to_asset_id_prefix,
			)
			.collect::<Vec<_>>();
			assert_eq!(location_to_asset_id.len(), 9);

			// Assert last asset id
			let last_asset_id =
				get_storage_value::<TokenId>(pallet_prefix, last_asset_id_prefix, &[]).unwrap_or(0);
			assert_eq!(last_asset_id, 8);

			// Compare tokens totalIssuance from before and after upgrade
			let pallet_prefix: &[u8] = b"Tokens";
			let total_issuance_prefix: &[u8] = b"TotalIssuance";
			let tokens_total_issuance = storage_key_iter::<TokenId, u128, Twox64Concat>(
				pallet_prefix,
				total_issuance_prefix,
			)
			.collect::<Vec<_>>();
			log::info!(
				target: "asset_registry",
				"post_upgrade tokens total issuance {:?}",
				tokens_total_issuance.len(),
			);
			// let pre_tokens_total_issuance =
			// 	Self::get_temp_storage::<Vec<(u32, u128)>>("pre_tokens_total_issuance")
			// 		.unwrap();
			// assert_eq!(tokens_total_issuance.len(), pre_tokens_total_issuance.len());

			Ok(())
		}
	}
}
