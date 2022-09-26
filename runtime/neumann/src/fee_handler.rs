use crate::{AssetRegistry, Balance, Call, XcmpHandler, NATIVE_TOKEN_ID, TOKEN_DECIMALS};
use frame_support::{
	traits::{Currency, ExistenceRequirement, Imbalance, OnUnbalanced, WithdrawReasons},
	unsigned::TransactionValidityError,
};
use orml_asset_registry::AssetMetadata;
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use pallet_xcmp_handler::XcmCurrencyData;
use primitives::{assets::CustomMetadata, TokenId};
use sp_runtime::{
	traits::{DispatchInfoOf, Get, PostDispatchInfoOf, Saturating, Zero},
	transaction_validity::InvalidTransaction,
	SaturatedConversion,
};
use sp_std::marker::PhantomData;

#[derive(Debug)]
pub struct FeeInformation {
	token_id: TokenId,
	xcm_data: Option<XcmCurrencyData>,
	asset_metadata: Option<AssetMetadata<Balance, CustomMetadata>>,
}
pub trait CallParser<Call> {
	fn fee_information(call: &Call) -> FeeInformation;
}
pub struct FeeCallParser;
impl CallParser<Call> for FeeCallParser {
	fn fee_information(c: &Call) -> FeeInformation {
		if let Call::AutomationTime(pallet_automation_time::Call::schedule_xcmp_task {
			para_id,
			currency_id,
			..
		}) = c.clone()
		{
			let xcm_data = XcmpHandler::get_xcm_chain_data(u32::from(para_id), currency_id);
			let asset_metadata = AssetRegistry::metadata(currency_id);

			FeeInformation { token_id: currency_id, xcm_data, asset_metadata }
		} else {
			FeeInformation { token_id: NATIVE_TOKEN_ID, xcm_data: None, asset_metadata: None }
		}
	}
}

pub struct ForeignFeeProcessor<T, MC, TA>(PhantomData<(T, MC, TA)>);
impl<T, MC, TA> ForeignFeeProcessor<T, MC, TA>
where
	T: pallet_transaction_payment::Config,
	MC: MultiCurrency<<T as frame_system::Config>::AccountId>,
	MC::CurrencyId: From<TokenId>,
	TA: Get<T::AccountId>,
{
	fn withdraw_fee(
		who: &T::AccountId,
		fee: MC::Balance,
		call_information: FeeInformation,
	) -> Result<(), TransactionValidityError> {
		if fee.is_zero() {
			return Ok(())
		}

		if call_information.xcm_data.is_none() || call_information.asset_metadata.is_none() {
			return Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
		}

		let currency_id = call_information.token_id.into();
		let foreign_fee: MC::Balance = call_information
			.asset_metadata
			.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?
			.convert_fee_into_foreign(fee.saturated_into())
			.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?
			.saturated_into();

		// orml_tokens doesn't provide withdraw that allows setting existence so prevent
		// withdraw if they don't have enough to avoid reaping
		MC::ensure_can_withdraw(
			currency_id,
			who,
			foreign_fee.saturating_add(MC::minimum_balance(currency_id)),
		)
		.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		MC::withdraw(currency_id, who, foreign_fee)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		MC::deposit(currency_id, &TA::get(), foreign_fee) // treasury account
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		// We dealt with imbalance here so don't let `correct_and_deposit_fee` do it
		Ok(())
	}
}

pub type CallOf<T> = <T as frame_system::Config>::Call;
type NegativeImbalanceOf<C, T> =
	<C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub struct CurrencyAdapter<MC, C, OU, TA, FCP>(PhantomData<(MC, C, OU, TA, FCP)>);

impl<T, MC, C, OU, TA, FCP> OnChargeTransaction<T> for CurrencyAdapter<MC, C, OU, TA, FCP>
where
	T: pallet_transaction_payment::Config,
	C: Currency<<T as frame_system::Config>::AccountId>,
	C::PositiveImbalance: Imbalance<
		<C as Currency<<T as frame_system::Config>::AccountId>>::Balance,
		Opposite = C::NegativeImbalance,
	>,
	C::NegativeImbalance: Imbalance<
		<C as Currency<<T as frame_system::Config>::AccountId>>::Balance,
		Opposite = C::PositiveImbalance,
	>,
	MC: MultiCurrency<<T as frame_system::Config>::AccountId>,
	MC::CurrencyId: From<TokenId>,
	MC::Balance: From<C::Balance>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	TA: Get<T::AccountId>,
	FCP: CallParser<CallOf<T>>,
{
	type LiquidityInfo = Option<NegativeImbalanceOf<C, T>>;
	type Balance = <C as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Withdraw the prxedicted fee from the transaction origin.
	///
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		call: &T::Call,
		_info: &DispatchInfoOf<T::Call>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		let call_information = FCP::fee_information(call.clone());

		if call_information.token_id == NATIVE_TOKEN_ID {
			// NativeFeeProcessor::<T, C>::withdraw_fee(who, fee, tip)
			<pallet_transaction_payment::CurrencyAdapter<C, OU> as OnChargeTransaction<T>>::withdraw_fee(
                who, call, _info, fee, tip
            )
		} else {
			ForeignFeeProcessor::<T, MC, TA>::withdraw_fee(who, fee.into(), call_information)?;
			Ok(None)
		}
	}

	/// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may
	/// be refunded.
	///
	/// Note: The `corrected_fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::Call>,
		_post_info: &PostDispatchInfoOf<T::Call>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		<pallet_transaction_payment::CurrencyAdapter<C, OU> as OnChargeTransaction<T>>::correct_and_deposit_fee(
			who, _dispatch_info, _post_info, corrected_fee, tip, already_withdrawn
		)
	}
}

pub trait FeeConversion {
	fn convert_fee_into_foreign(&self, fee: Balance) -> Option<Balance>;
}
impl FeeConversion for orml_asset_registry::AssetMetadata<Balance, CustomMetadata> {
	fn convert_fee_into_foreign(&self, fee: Balance) -> Option<Balance> {
		let decimaled_fee = if self.decimals >= TOKEN_DECIMALS {
			fee / 10_u128.pow(self.decimals - TOKEN_DECIMALS)
		} else {
			fee * 10_u128.pow(TOKEN_DECIMALS - self.decimals)
		};

		match self.additional.conversion_rate {
			Some(value) => Some(decimaled_fee * value.native as Balance / value.foreign as Balance),
			None => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use primitives::assets::ConversionRate;

	macro_rules! asset_struct {
		($decimals:literal, $custom_metadata:tt) => {
			AssetMetadata {
				decimals: $decimals,
				name: b"Foreign".to_vec(),
				symbol: b"FU".to_vec(),
				existential_deposit: 1,
				location: None,
				additional: CustomMetadata $custom_metadata,
			}
		};
	}

	macro_rules! test_asset {
		($decimals:literal) => {
			asset_struct!($decimals, {
				fee_per_second: Some(1),
				conversion_rate: None,
			})
		};
		($decimals:literal, $native:literal : $foreign:literal) => {
			asset_struct!($decimals, {
				fee_per_second: Some(1),
				conversion_rate: Some(ConversionRate { native: $native, foreign: $foreign }),
			})
		};
	}

	#[test]
	fn ensure_none_if_no_conversion_rate() {
		let asset = test_asset!(10);

		assert_eq!(asset.convert_fee_into_foreign(1), None)
	}

	#[test]
	fn check_simple_conversion() {
		let asset = test_asset!(10, 1:50);

		assert_eq!(asset.convert_fee_into_foreign(200), Some(4))
	}

	#[test]
	fn check_decimal_conversion() {
		let asset = test_asset!(12, 1:50);

		assert_eq!(asset.convert_fee_into_foreign(20000), Some(4))
	}

	#[test]
	fn check_decimal_conversion_with_bigger_native() {
		let asset = test_asset!(8, 1:50);

		assert_eq!(asset.convert_fee_into_foreign(200), Some(400))
	}

	#[test]
	fn works_with_large_decimal_Diff() {
		let asset = test_asset!(20, 10000:1);
		assert_eq!(asset.convert_fee_into_foreign(1000000000), Some(1000))
	}
}
