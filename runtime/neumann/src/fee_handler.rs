use crate::*;
use frame_support::{ traits::{Currency, ExistenceRequirement, WithdrawReasons},
	unsigned::TransactionValidityError,
};
use pallet_transaction_payment::OnChargeTransaction;
use pallet_xcmp_handler::XcmCurrencyData;
use sp_runtime::{
	traits::{DispatchInfoOf, PostDispatchInfoOf, Saturating, Zero},
	transaction_validity::InvalidTransaction,
};
use sp_std::marker::PhantomData;
use orml_asset_registry::AssetMetadata;

#[derive(Debug)]
pub struct FeeInformation {
	token_id: TokenId,
	xcm_data: Option<XcmCurrencyData>,
	asset_metadata: Option<AssetMetadata<Balance, CustomMetadata>>,
}
impl Default for FeeInformation {
	fn default() -> FeeInformation {
		FeeInformation { token_id: NATIVE_TOKEN_ID, xcm_data: None, asset_metadata: None }
	}
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
			FeeInformation::default()
		}
	}
}
pub type CallOf<T> = <T as frame_system::Config>::Call;
type NegativeImbalanceOf<C, T> =
	<C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

use orml_traits::MultiCurrency;

pub struct DuplicateCurrencyAdapter<MC, C, OU, FCP>(PhantomData<(MC, C, OU, FCP)>);

impl<T, MC, C, OU, FCP> OnChargeTransaction<T> for DuplicateCurrencyAdapter<MC, C, OU, FCP>
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
	MC::CurrencyId: From<TokenId>,
	MC::Balance: From<C::Balance>,
	MC: MultiCurrency<<T as frame_system::Config>::AccountId>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	FCP: CallParser<CallOf<T>>,
{
	type LiquidityInfo = Option<(MC::CurrencyId, NegativeImbalanceOf<C, T>)>;
	type Balance = <C as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Withdraw the predicted fee from the transaction origin.
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

		// cross it
		let call_name = FCP::fee_information(call.clone());

		let currency_id = call_name.token_id.into();
		// Check existential deposit
		if call_name.token_id != NATIVE_TOKEN_ID {
			MC::ensure_can_withdraw(
				currency_id,
				who,
				fee.saturating_add(MC::minimum_balance(currency_id).into()).into(),
			)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			match MC::withdraw(currency_id, who, fee.into()) {
				// TODO: real imbalance?
				Ok(()) => Ok(Some((currency_id, C::NegativeImbalance::zero()))),
				Err(_) => Err(InvalidTransaction::Payment.into()),
			}
		} else {
			let withdraw_reason = if tip.is_zero() {
				WithdrawReasons::TRANSACTION_PAYMENT
			} else {
				WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
			};

			match C::withdraw(who, fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
				Ok(imbalance) => Ok(Some((currency_id, imbalance))),
				Err(_) => Err(InvalidTransaction::Payment.into()),
			}
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
			let paid = paid.1;
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

