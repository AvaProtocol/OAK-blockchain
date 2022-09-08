use super::*;
use frame_support::traits::OnRuntimeUpgrade;

pub mod asset_registry {
	use super::*;
	use frame_support::{Blake2_128Concat, BoundedVec, Twox64Concat, WeakBoundedVec};
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

	#[frame_support::storage_alias]
	type TotalIssuance = StorageMap<Tokens, Twox64Concat, CurrencyId, Balance>;

	#[frame_support::storage_alias]
	type Locks<T: Config> = StorageDoubleMap<
		Tokens,
		Blake2_128Concat,
		AccountId,
		Twox64Concat,
		CurrencyId,
		BoundedVec<orml_tokens::BalanceLock<Balance>, <T as orml_tokens::Config>::MaxLocks>,
	>;

	#[frame_support::storage_alias]
	type Accounts = StorageDoubleMap<
		Tokens,
		Blake2_128Concat,
		AccountId,
		Twox64Concat,
		CurrencyId,
		orml_tokens::AccountData<Balance>,
	>;

	#[frame_support::storage_alias]
	type Reserves<T: Config> = StorageDoubleMap<
		Tokens,
		Blake2_128Concat,
		AccountId,
		Twox64Concat,
		CurrencyId,
		BoundedVec<
			orml_tokens::ReserveData<<T as orml_tokens::Config>::ReserveIdentifier, Balance>,
			<T as orml_tokens::Config>::MaxReserves,
		>,
	>;

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

			// Set LastAssetId (zero index)
			let pallet_prefix: &[u8] = b"AssetRegistry";
			let last_asset_id_prefix: &[u8] = b"LastAssetId";
			let last_asset_id: TokenId = (assets.len() - 1).try_into().unwrap();
			put_storage_value::<TokenId>(pallet_prefix, last_asset_id_prefix, &[], last_asset_id);

			log::info!(
				target: "asset_registry",
				"on_runtime_upgrade: New data inserted"
			);

			// Migrate Tokens Accounts CurrencyId from Enum to u32
			let mut tokens_accounts: Vec<(AccountId, TokenId, orml_tokens::AccountData<Balance>)> =
				vec![];
			Accounts::drain().for_each(|(account_id, currency_id, account_data)| {
				tokens_accounts.push((account_id, currency_id as u32, account_data));
			});
			tokens_accounts.iter().for_each(|(account_id, currency_id, account_data)| {
				orml_tokens::Accounts::<Runtime>::insert(account_id, currency_id, account_data);
			});
			log::info!(
				target: "orml_tokens",
				"on_runtime_upgrade: Accounts data updated {:?}",
				orml_tokens::Accounts::<Runtime>::iter().count(),
			);

			// Migrate Tokens TotalIssuance CurrencyId from Enum to u32
			let mut total_issuance: Vec<(TokenId, Balance)> = vec![];
			TotalIssuance::drain().for_each(|(currency_id, balance)| {
				total_issuance.push((currency_id as u32, balance));
			});
			total_issuance.iter().for_each(|(currency_id, balance)| {
				orml_tokens::TotalIssuance::<Runtime>::insert(currency_id, balance);
			});
			log::info!(
				target: "orml_tokens",
				"on_runtime_upgrade: TotalIssuance data updated",
			);

			let mut total_rw = 0;
			// Each asset + each asset location + updating last asset id
			total_rw += assets.len() as u32 * 2 + 1;
			// Each tokens account/currency_id
			total_rw += tokens_accounts.len() as u32;
			// Each tokens currency total issuance
			total_rw += total_issuance.len() as u32;
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

			// Assert asset Metadata length is 0
			let metadata = storage_key_iter::<TokenId, AssetMetadataOf, Twox64Concat>(
				pallet_prefix,
				metadata_prefix,
			)
			.collect::<Vec<_>>();
			assert_eq!(metadata.len(), 0);

			// Assert asset LocationToAssetId length is 0
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

			// Store old tokens accounts for comparing after migrations
			let old_tokens_accounts = Accounts::iter().collect::<Vec<_>>();
			Self::set_temp_storage::<Vec<(AccountId, CurrencyId, orml_tokens::AccountData<Balance>)>>(
				old_tokens_accounts.clone(),
				"old_tokens_accounts",
			);
			log::info!(
				target: "asset_registry",
				"pre_upgrade tokens account {:?}",
				old_tokens_accounts.len(),
			);

			// Store old tokens total issuance for comparing after migrations
			let old_total_issuance = TotalIssuance::iter().collect::<Vec<_>>();
			Self::set_temp_storage::<Vec<(CurrencyId, Balance)>>(
				old_total_issuance.clone(),
				"old_total_issuance",
			);
			log::info!(
				target: "asset_registry",
				"pre_upgrade tokens total issuance {:?}",
				old_total_issuance.len(),
			);

			// Reserves and Locks are not currently used. Ensure collections are empty and don't need migrating.
			let locks = Locks::<Runtime>::iter().collect::<Vec<_>>().len();
			assert_eq!(locks, 0);
			let reserves = Reserves::<Runtime>::iter().collect::<Vec<_>>().len();
			assert_eq!(reserves, 0);

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

			// Compare tokens accounts from before and after upgrade
			let new_tokens_accounts = orml_tokens::Accounts::<Runtime>::iter().collect::<Vec<_>>();
			let old_tokens_accounts = Self::get_temp_storage::<
				Vec<(AccountId, CurrencyId, orml_tokens::AccountData<Balance>)>,
			>("old_tokens_accounts")
			.unwrap();
			assert_eq!(old_tokens_accounts.len(), new_tokens_accounts.len());
			old_tokens_accounts
				.iter()
				.for_each(|(account_id, currency_id_old, account_data)| {
					let current_account_data = orml_tokens::Accounts::<Runtime>::get(
						account_id.clone(),
						currency_id_old.clone() as u32,
					);
					assert_eq!(current_account_data.free, account_data.free);
					assert_eq!(current_account_data.reserved, account_data.reserved);
					assert_eq!(current_account_data.frozen, account_data.reserved);
				});
			log::info!(
				target: "orml_tokens",
				"post_upgrade check Accounts complete",
			);

			// Compare tokens TotalIssuance from before and after upgrade
			let new_total_issuance =
				orml_tokens::TotalIssuance::<Runtime>::iter().collect::<Vec<_>>();
			let old_total_issuance =
				Self::get_temp_storage::<Vec<(CurrencyId, Balance)>>("old_total_issuance").unwrap();
			assert_eq!(new_total_issuance.len(), old_total_issuance.len());
			old_total_issuance.iter().for_each(|(currency_id_old, balance)| {
				let current_total_issuance =
					orml_tokens::TotalIssuance::<Runtime>::get(currency_id_old.clone() as u32);
				assert_eq!(current_total_issuance, *balance);
			});
			log::info!(
				target: "orml_tokens",
				"post_upgrade check TotalIssuance complete",
			);

			// Check Reserves and Locks are still empty.
			let locks = orml_tokens::Locks::<Runtime>::iter().collect::<Vec<_>>().len();
			assert_eq!(locks, 0);
			let reserves = orml_tokens::Reserves::<Runtime>::iter().collect::<Vec<_>>().len();
			assert_eq!(reserves, 0);
			log::info!(
				target: "orml_tokens",
				"post_upgrade check Locks and Reserves complete",
			);

			Ok(())
		}
	}
}
