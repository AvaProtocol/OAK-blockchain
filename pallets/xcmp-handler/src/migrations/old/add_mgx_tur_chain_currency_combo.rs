use core::marker::PhantomData;

use crate::{Config, XcmChainCurrencyData, XcmCurrencyData};
use frame_support::{
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
};

const MANGATA_PARA_ID: u32 = 2110;
const FEE_PER_SECOND: u128 = 5_376_000_000_000;

pub struct AddMgxTurChainCurrencyCombo<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for AddMgxTurChainCurrencyCombo<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "xcmp-handler", "AddMgxTurChainCurrencyCombo migration");

		XcmChainCurrencyData::<T>::insert(
			MANGATA_PARA_ID,
			T::GetNativeCurrencyId::get(),
			XcmCurrencyData {
				native: false,
				fee_per_second: FEE_PER_SECOND,
				instruction_weight: 1_000_000_000,
			},
		);

		T::DbWeight::get().writes(1u64)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		// noop
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let xcm_data =
			XcmChainCurrencyData::<T>::get(MANGATA_PARA_ID, T::GetNativeCurrencyId::get())
				.ok_or("ChainCurrencyComboNotFound")?;
		assert_eq!(xcm_data.fee_per_second, FEE_PER_SECOND);

		log::info!(
			target: "xcmp-handler",
			"migration: AddMgxTurChainCurrencyCombo POST migration checks succesful!",
		);

		Ok(())
	}
}
