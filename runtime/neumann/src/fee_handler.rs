use crate::{AssetRegistry, Balance, Call, NATIVE_TOKEN_ID, TOKEN_DECIMALS, XcmpHandler};
use primitives::{TokenId, assets::CustomMetadata};
use frame_support::{
	traits::{Currency, ExistenceRequirement, Imbalance, OnUnbalanced, WithdrawReasons},
	unsigned::TransactionValidityError,
};
use orml_asset_registry::AssetMetadata;
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use pallet_xcmp_handler::XcmCurrencyData;
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
pub type CallOf<T> = <T as frame_system::Config>::Call;
type NegativeImbalanceOf<C, T> =
	<C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub struct CurrencyAdapter<MC, C, OU, TA, FCP>(PhantomData<(MC, C, OU, TA, FCP)>);

impl<T, MC, C, OU, TA, FCP> OnChargeTransaction<T> for CurrencyAdapter<MC, C, OU, TA, FCP>
where
	T: pallet_transaction_payment::Config,
	C: Currency<<T as frame_system::Config>::AccountId>,
	C::Balance: From<MC::Balance>,
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
		if fee.is_zero() {
			return Ok(None)
		}

		let call_information = FCP::fee_information(call.clone());

		if call_information.token_id != NATIVE_TOKEN_ID && call_information.xcm_data == None ||
			call_information.asset_metadata == None
		{
			return Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
		}

		if call_information.token_id == NATIVE_TOKEN_ID {
			let withdraw_reason = if tip.is_zero() {
				WithdrawReasons::TRANSACTION_PAYMENT
			} else {
				WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
			};

			match C::withdraw(who, fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
				Ok(imbalance) => Ok(Some(imbalance)),
				Err(_) => Err(InvalidTransaction::Payment.into()),
			}
		} else {
			let currency_id = call_information.token_id.into();
			let foreign_fee = call_information
				.asset_metadata
				.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?
				.convert_fee_into_foreign(fee.saturated_into())
				.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			// orml_tokens doesn't provide withdraw that allows setting existence so prevent
			// withdraw if they don't have enough to avoid reaping
			MC::ensure_can_withdraw(
				currency_id,
				who,
				foreign_fee
					.saturating_add(MC::minimum_balance(currency_id).saturated_into())
					.saturated_into(),
			)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			MC::withdraw(currency_id, who, foreign_fee.saturated_into())
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			MC::deposit(currency_id, &TA::get(), foreign_fee.saturated_into()) // treasury account
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			// TODO: Fire event for deposit

			// We dealt with imbalance here so don't let `correct_and_deposit_fee` do it
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
		if let Some(paid) = already_withdrawn {
			// Calculate how much refund we should return
			let refund_amount = paid.peek().saturating_sub(corrected_fee);
			// refund to the the account that paid the fees. If this fails, the
			// account might have dropped below the existential balance. In
			// that case we don't refund anything.
			let refund_imbalance = C::deposit_into_existing(who, refund_amount)
				.unwrap_or_else(|_| C::PositiveImbalance::zero());
			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid = paid
				.offset(refund_imbalance)
				.same()
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
			// Call someone else to handle the imbalance (fee and tip separately)
			let (tip, fee) = adjusted_paid.split(tip);
			OU::on_unbalanceds(Some(fee).into_iter().chain(Some(tip)));
		}
		Ok(())
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

	macro_rules! test_asset {
		($decimals:literal) => {
			AssetMetadata {
				decimals: $decimals,
				name: b"Foreign".to_vec(),
				symbol: b"FU".to_vec(),
				existential_deposit: 1,
				location: None,
				additional: CustomMetadata { fee_per_second: Some(1), conversion_rate: None },
			}
		};
		($decimals:literal, $native:literal, $foreign:literal) => {
			AssetMetadata {
				decimals: $decimals,
				name: b"Foreign".to_vec(),
				symbol: b"FU".to_vec(),
				existential_deposit: 1,
				location: None,
				additional: CustomMetadata {
					fee_per_second: Some(1),
					conversion_rate: Some(ConversionRate { native: $native, foreign: $foreign }),
				},
			}
		};
	}

	#[test]
	fn ensure_none_if_no_conversion_rate() {
		let asset = test_asset!(10);

		assert_eq!(asset.convert_fee_into_foreign(1), None)
	}

	#[test]
	fn check_simple_conversion() {
		let asset = test_asset!(10, 1, 50);

		assert_eq!(asset.convert_fee_into_foreign(200), Some(4))
	}

	#[test]
	fn check_decimal_conversion() {
		let asset = test_asset!(12, 1, 50);

		assert_eq!(asset.convert_fee_into_foreign(20000), Some(4))
	}

	#[test]
	fn check_decimal_conversion_with_bigger_native() {
		let asset = test_asset!(8, 1, 50);

		assert_eq!(asset.convert_fee_into_foreign(200), Some(400))
	}
}
