// This file is part of OAK Blockchain.

// Copyright (C) 2022 OAK Network
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// ! Traits and default implementation for paying execution fees.
use crate::{AccountOf, Action, ActionOf, Config, Error, MultiBalanceOf, MultiCurrencyId, Pallet};

use frame_support::traits::{Currency, OnUnbalanced};
use orml_traits::MultiCurrency;
use pallet_xcmp_handler::XcmpTransactor;
use sp_runtime::{
	traits::{CheckedSub, Saturating, Zero},
	DispatchError, DispatchResult, SaturatedConversion,
	TokenError::BelowMinimum,
};
use sp_std::marker::PhantomData;

type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

/// Handle execution fee payments in the context of automation actions
pub trait HandleFees<T: Config> {
	fn pay_checked_fees_for<R, F: FnOnce() -> Result<R, DispatchError>>(
		owner: &AccountOf<T>,
		action: &ActionOf<T>,
		executions: u32,
		prereq: F,
	) -> Result<R, DispatchError>;
}

pub struct FeeHandler<T: Config, OU> {
	owner: T::AccountId,
	pub currency_id: MultiCurrencyId<T>,
	pub execution_fee: MultiBalanceOf<T>,
	pub xcmp_fee: MultiBalanceOf<T>,
	_phantom_data: PhantomData<OU>,
}

impl<T, OU> HandleFees<T> for FeeHandler<T, OU>
where
	T: Config,
	OU: OnUnbalanced<NegativeImbalanceOf<T>>,
{
	fn pay_checked_fees_for<R, F: FnOnce() -> Result<R, DispatchError>>(
		owner: &AccountOf<T>,
		action: &ActionOf<T>,
		executions: u32,
		prereq: F,
	) -> Result<R, DispatchError> {
		let fee_handler = Self::new(owner, action, executions)?;
		// Note: will need to account for fees in non-native tokens once we start accepting them
		fee_handler.can_pay_fee().map_err(|_| Error::<T>::InsufficientBalance)?;
		let outcome = prereq()?;
		fee_handler.pay_fees()?;
		Ok(outcome)
	}
}

impl<T, OU> FeeHandler<T, OU>
where
	T: Config,
	OU: OnUnbalanced<NegativeImbalanceOf<T>>,
{
	/// Ensure the fee can be paid.
	fn can_pay_fee(&self) -> Result<(), DispatchError> {
		let fee = self.execution_fee.saturating_add(self.xcmp_fee);

		if fee.is_zero() {
			return Ok(())
		}

		// Manually check for ExistenceRequirement since MultiCurrency doesn't currently support it
		let free_balance = T::MultiCurrency::free_balance(self.currency_id, &self.owner);
		free_balance
			.checked_sub(&fee)
			.ok_or(DispatchError::Token(BelowMinimum))?
			.checked_sub(&T::MultiCurrency::minimum_balance(self.currency_id))
			.ok_or(DispatchError::Token(BelowMinimum))?;
		T::MultiCurrency::ensure_can_withdraw(self.currency_id.into(), &self.owner, fee)?;

		Ok(())
	}

	/// Withdraw the fee.
	fn withdraw_fee(&self) -> Result<(), DispatchError> {
		let fee = self.execution_fee.saturating_add(self.xcmp_fee);

		if fee.is_zero() {
			return Ok(())
		}

		match T::MultiCurrency::withdraw(self.currency_id, &self.owner, fee) {
			Ok(_) => {
				// TODO: fees need to go to multi currency enabled treasury
				Ok(())
			},
			Err(_) => Err(DispatchError::Token(BelowMinimum)),
		}
	}

	/// Builds an instance of the struct
	pub fn new(
		owner: &AccountOf<T>,
		action: &ActionOf<T>,
		executions: u32,
	) -> Result<Self, DispatchError> {
		let currency_id = action.currency_id::<T>().into();
		let execution_fee: u128 =
			Pallet::<T>::calculate_execution_fee(action, executions)?.saturated_into();

		let xcmp_fee = match *action {
			Action::XCMP { para_id, currency_id, encoded_call_weight, .. } =>
				T::XcmpTransactor::get_xcm_fee(
					u32::from(para_id),
					currency_id,
					encoded_call_weight.clone(),
				)?
				.saturating_mul(executions.into())
				.saturated_into(),
			_ => 0u32.saturated_into(),
		};

		Ok(Self {
			owner: owner.clone(),
			currency_id,
			execution_fee: execution_fee.saturated_into(),
			xcmp_fee,
			_phantom_data: Default::default(),
		})
	}

	/// Executes the fee handler
	fn pay_fees(self) -> DispatchResult {
		// This should never error if can_pay_fee passed.
		self.withdraw_fee().map_err(|_| Error::<T>::LiquidityRestrictions)?;

		if self.xcmp_fee > MultiBalanceOf::<T>::zero() {
			T::XcmpTransactor::pay_xcm_fee(self.owner, self.xcmp_fee.saturated_into())?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, Action};
	use frame_benchmarking::frame_support::assert_err;
	use frame_support::sp_runtime::AccountId32;

	#[test]
	fn pay_checked_fees_for_success() {
		new_test_ext(0).execute_with(|| {
			let alice = AccountId32::new(ALICE);
			get_funds(alice.clone());
			let starting_funds = Balances::free_balance(alice.clone());
			let mut spy = 0;
			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&Action::Notify { message: vec![0] },
				1,
				|| {
					spy += 1;
					Ok("called")
				},
			);
			assert_eq!(result.expect("success"), "called");
			assert_eq!(spy, 1);
			assert!(starting_funds > Balances::free_balance(alice))
		})
	}

	#[test]
	fn errors_when_not_enough_funds_for_fee() {
		new_test_ext(0).execute_with(|| {
			let alice = AccountId32::new(ALICE);
			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&Action::Notify { message: vec![0] },
				1,
				|| Ok(()),
			);
			assert_err!(result, Error::<Test>::InsufficientBalance);
		})
	}

	#[test]
	fn does_not_charge_fees_when_prereq_errors() {
		new_test_ext(0).execute_with(|| {
			let alice = AccountId32::new(ALICE);
			get_funds(alice.clone());
			let starting_funds = Balances::free_balance(alice.clone());
			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for::<(), _>(
				&alice,
				&Action::Notify { message: vec![0] },
				1,
				|| Err("error".into()),
			);
			assert_err!(result, DispatchError::Other("error"));
			assert_eq!(starting_funds, Balances::free_balance(alice))
		})
	}
}
