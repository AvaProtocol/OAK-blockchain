use super::*;
use frame_support::traits::OnRuntimeUpgrade;

pub mod assets {
	use super::*;
	use frame_support::{Blake2_128Concat, BoundedVec, Twox64Concat, WeakBoundedVec};
	use orml_asset_registry::AssetMetadata;

	pub type AssetMetadataOf = AssetMetadata<Balance, CustomMetadata>;

	#[derive(
		Debug, Encode, Decode, Eq, PartialEq, Copy, Clone, PartialOrd, Ord, TypeInfo, MaxEncodedLen,
	)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum CurrencyId {
		Native,
		KSM,
		AUSD,
		KAR,
		LKSM,
		HKO,
		SKSM,
		PHA,
	}

	impl From<u32> for CurrencyId {
		fn from(a: u32) -> Self {
			match a {
				0 => CurrencyId::Native,
				1 => CurrencyId::KSM,
				2 => CurrencyId::AUSD,
				3 => CurrencyId::KAR,
				4 => CurrencyId::LKSM,
				5 => CurrencyId::HKO,
				6 => CurrencyId::SKSM,
				7 => CurrencyId::PHA,
				_ => CurrencyId::Native,
			}
		}
	}

	pub mod parachains {
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

	#[frame_support::storage_alias]
	type LastAssetId<T: Config> =
		StorageValue<AssetRegistry, <T as orml_asset_registry::Config>::AssetId>;

	pub struct MigrateAssetRegistry;
	impl OnRuntimeUpgrade for MigrateAssetRegistry {
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
						additional: CustomMetadata {
							fee_per_second: Some(tur_per_second()),
							conversion_rate: None,
						},
						existential_deposit: EXISTENTIAL_DEPOSIT,
						location: Some(MultiLocation::new(0, Here).into()),
					},
				),
				(
					1,
					AssetMetadataOf {
						decimals: 12,
						name: b"Kusama".to_vec(),
						symbol: b"KSM".to_vec(),
						additional: CustomMetadata {
							fee_per_second: Some(ksm_per_second()),
							conversion_rate: None,
						},
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
						additional: CustomMetadata {
							fee_per_second: Some(ksm_per_second() * 400),
							conversion_rate: None,
						},
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
						additional: CustomMetadata {
							fee_per_second: Some(ksm_per_second() * 50),
							conversion_rate: None,
						},
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
						additional: CustomMetadata {
							fee_per_second: Some(ksm_per_second() * 10),
							conversion_rate: None,
						},
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
						additional: CustomMetadata {
							fee_per_second: Some(ksm_per_second() * 30),
							conversion_rate: None,
						},
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
						additional: CustomMetadata {
							fee_per_second: Some(ksm_per_second()),
							conversion_rate: None,
						},
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
						name: b"Phala".to_vec(),
						symbol: b"PHA".to_vec(),
						additional: CustomMetadata {
							fee_per_second: Some(ksm_per_second() * 400),
							conversion_rate: None,
						},
						existential_deposit: cent(12),
						location: Some(
							MultiLocation::new(1, X1(Parachain(parachains::khala::ID))).into(),
						),
					},
				),
			];

			// Insert assets
			for (id, metadata) in assets.iter() {
				orml_asset_registry::Pallet::<Runtime>::do_register_asset_without_asset_processor(
					metadata.clone(),
					*id,
				)
				.expect("should not fail");
			}

			// Set LastAssetId (zero index)
			let last_asset_id: TokenId = (assets.len() - 1).try_into().unwrap();
			LastAssetId::<Runtime>::set(Some(last_asset_id));

			log::info!(
				target: "asset_registry",
				"on_runtime_upgrade: New data inserted"
			);

			// Storage: AssetRegistry Metadata (r:1 w:1) per asset
			// Storage: AssetRegistry LocationToAssetId (r:1 w:1) per asset
			// Storage: AssetRegistry LastAssetId (r:1 w:1) x 1
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

			// Assert asset Metadata length is 0
			let asset_metadata_count =
				orml_asset_registry::Metadata::<Runtime>::iter().collect::<Vec<_>>().len();
			assert_eq!(asset_metadata_count, 0);

			// Assert asset LocationToAssetId length is 0
			let location_to_asset_id_count =
				orml_asset_registry::LocationToAssetId::<Runtime>::iter()
					.collect::<Vec<_>>()
					.len();
			assert_eq!(location_to_asset_id_count, 0);

			// Assert last asset id is 0
			if let Some(_last_asset_id) = LastAssetId::<Runtime>::get() {
				return Err("pre_upgrade check - LastAssetId should equal None".into())
			}

			Ok(())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			log::info!(
				target: "asset_registry",
				"post_upgrade check"
			);

			// Assert Metadata length
			let asset_metadata_count =
				orml_asset_registry::Metadata::<Runtime>::iter().collect::<Vec<_>>().len();
			assert_eq!(asset_metadata_count, 8);

			// Assert LocationToAssetId length
			let location_to_asset_id_count =
				orml_asset_registry::LocationToAssetId::<Runtime>::iter()
					.collect::<Vec<_>>()
					.len();
			assert_eq!(location_to_asset_id_count, 8);

			// Assert last asset id (zero index)
			if let Some(last_asset_id) = LastAssetId::<Runtime>::get() {
				assert_eq!(last_asset_id, 7);
			} else {
				return Err("post_upgrade check - LastAssetId should equal 7".into())
			}

			Ok(())
		}
	}

	pub struct MigrateTokensCurrencyId;
	impl OnRuntimeUpgrade for MigrateTokensCurrencyId {
		fn on_runtime_upgrade() -> Weight {
			log::info!(
				target: "orml_tokens",
				"on_runtime_upgrade: Attempted to apply tokens"
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
				tokens_accounts.len(),
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

			// Each tokens account/currency_id + Each tokens currency total issuance
			let total_rw = tokens_accounts.len() as u32 * 2;
			<Runtime as frame_system::Config>::DbWeight::get()
				.reads_writes(total_rw as Weight, total_rw as Weight)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			use frame_support::traits::OnRuntimeUpgradeHelpersExt;

			// Store old tokens accounts for comparing after migrations
			let old_tokens_accounts = Accounts::iter().collect::<Vec<_>>();
			Self::set_temp_storage::<Vec<(AccountId, CurrencyId, orml_tokens::AccountData<Balance>)>>(
				old_tokens_accounts.clone(),
				"old_tokens_accounts",
			);
			log::info!(
				target: "orml_tokens",
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
				target: "orml_tokens",
				"pre_upgrade tokens total issuance {:?}",
				old_total_issuance.len(),
			);

			// Reserves and Locks are not currently used. Ensure collections are empty and don't need migrating.
			let locks = Locks::<Runtime>::iter().collect::<Vec<_>>().len();
			assert_eq!(locks, 0);
			let reserves = Reserves::<Runtime>::iter().collect::<Vec<_>>().len();
			assert_eq!(reserves, 0);

			// XcmChainCurrencyData is not currently used. Ensure collection is empty and doesn't need migrating.
			let xcmp_chain_data = pallet_xcmp_handler::XcmChainCurrencyData::<Runtime>::iter()
				.collect::<Vec<_>>()
				.len();
			assert_eq!(xcmp_chain_data, 0);

			Ok(())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			use frame_support::traits::OnRuntimeUpgradeHelpersExt;

			// Compare tokens accounts from before and after upgrade
			let new_tokens_accounts = orml_tokens::Accounts::<Runtime>::iter().collect::<Vec<_>>();
			let old_tokens_accounts = Self::get_temp_storage::<
				Vec<(AccountId, CurrencyId, orml_tokens::AccountData<Balance>)>,
			>("old_tokens_accounts")
			.unwrap();
			// Assert length of accounts equals
			assert_eq!(old_tokens_accounts.len(), new_tokens_accounts.len());
			// Assert account/currency balance equals
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
