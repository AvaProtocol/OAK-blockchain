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
use crate::{AccountOf, Action, ActionOf, Config, Error, MultiBalanceOf, Pallet};

use orml_traits::MultiCurrency;
use pallet_xcmp_handler::{InstructionSequence, XcmpTransactor};
use sp_runtime::{
	traits::{CheckedSub, Convert, Saturating, Zero},
	DispatchError, DispatchResult, SaturatedConversion,
	TokenError::BelowMinimum,
};
use sp_std::marker::PhantomData;
use xcm::latest::prelude::*;
use xcm_builder::TakeRevenue;

/// Handle execution fee payments in the context of automation actions
pub trait HandleFees<T: Config> {
	fn pay_checked_fees_for<R, F: FnOnce() -> Result<R, DispatchError>>(
		owner: &AccountOf<T>,
		action: &ActionOf<T>,
		executions: u32,
		prereq: F,
	) -> Result<R, DispatchError>;
}

pub struct FeeHandler<T: Config, TR> {
	owner: T::AccountId,
	pub schedule_fee_location: MultiLocation,
	pub schedule_fee_amount: MultiBalanceOf<T>,
	pub execution_fee_amount: MultiBalanceOf<T>,
	_phantom_data: PhantomData<TR>,
}

impl<T, TR> HandleFees<T> for FeeHandler<T, TR>
where
	T: Config,
	TR: TakeRevenue,
{
	fn pay_checked_fees_for<R, F: FnOnce() -> Result<R, DispatchError>>(
		owner: &AccountOf<T>,
		action: &ActionOf<T>,
		executions: u32,
		prereq: F,
	) -> Result<R, DispatchError> {
		let fee_handler = Self::new(owner, action, executions)?;
		fee_handler.can_pay_fee().map_err(|_| Error::<T>::InsufficientBalance)?;
		let outcome = prereq()?;
		fee_handler.pay_fees()?;
		Ok(outcome)
	}
}

impl<T, TR> FeeHandler<T, TR>
where
	T: Config,
	TR: TakeRevenue,
{
	/// Ensure the fee can be paid.
	fn can_pay_fee(&self) -> Result<(), DispatchError> {
		let fee = self.schedule_fee_amount.saturating_add(self.execution_fee_amount);

		if fee.is_zero() {
			return Ok(())
		}

		// Manually check for ExistenceRequirement since MultiCurrency doesn't currently support it
		let currency_id = T::CurrencyIdConvert::convert(self.schedule_fee_location)
			.ok_or("IncoveribleMultilocation")?;
		let currency_id = currency_id.into();
		let free_balance = T::MultiCurrency::free_balance(currency_id, &self.owner);

		free_balance
			.checked_sub(&fee)
			.ok_or(DispatchError::Token(BelowMinimum))?
			.checked_sub(&T::MultiCurrency::minimum_balance(currency_id))
			.ok_or(DispatchError::Token(BelowMinimum))?;
		T::MultiCurrency::ensure_can_withdraw(currency_id, &self.owner, fee)?;
		Ok(())
	}

	/// Withdraw the fee.
	fn withdraw_fee(&self) -> Result<(), DispatchError> {
		let fee = self.schedule_fee_amount.saturating_add(self.execution_fee_amount);

		if fee.is_zero() {
			return Ok(())
		}

		let currency_id = T::CurrencyIdConvert::convert(self.schedule_fee_location)
			.ok_or("IncoveribleMultilocation")?;
		let currency_id = currency_id.into();

		match T::MultiCurrency::withdraw(currency_id, &self.owner, fee) {
			Ok(_) => {
				TR::take_revenue(MultiAsset {
					id: AssetId::Concrete(self.schedule_fee_location),
					fun: Fungibility::Fungible(self.schedule_fee_amount.saturated_into()),
				});

				if self.execution_fee_amount > MultiBalanceOf::<T>::zero() {
					T::XcmpTransactor::pay_xcm_fee(
						self.owner.clone(),
						self.execution_fee_amount.saturated_into(),
					)?;
				}

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
		let schedule_fee_location = action.schedule_fee_location::<T>();

		let schedule_fee_amount: u128 =
			Pallet::<T>::calculate_schedule_fee_amount(action, executions)?.saturated_into();

		let execution_fee_amount = match action.clone() {
			Action::XCMP { execution_fee, instruction_sequence, .. }
				if instruction_sequence == InstructionSequence::PayThroughSovereignAccount =>
				execution_fee.amount.saturating_mul(executions.into()).saturated_into(),
			_ => 0u32.saturated_into(),
		};

		Ok(Self {
			owner: owner.clone(),
			schedule_fee_location,
			schedule_fee_amount: schedule_fee_amount.saturated_into(),
			execution_fee_amount,
			_phantom_data: Default::default(),
		})
	}

	/// Executes the fee handler
	fn pay_fees(self) -> DispatchResult {
		// This should never error if can_pay_fee passed.
		self.withdraw_fee().map_err(|_| Error::<T>::LiquidityRestrictions)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, Action};
	use codec::Encode;
	use frame_benchmarking::frame_support::assert_err;
	use frame_support::sp_runtime::AccountId32;

	#[test]
	fn pay_checked_fees_for_success() {
		new_test_ext(0).execute_with(|| {
			let alice = AccountId32::new(ALICE);
			fund_account(&alice, 900_000_000, 1, Some(0));
			let starting_funds = Balances::free_balance(alice.clone());

			let call: <Test as frame_system::Config>::RuntimeCall =
				frame_system::Call::remark_with_event { remark: vec![50] }.into();
			let mut spy = 0;
			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&Action::DynamicDispatch { encoded_call: call.encode() },
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
			let call: <Test as frame_system::Config>::RuntimeCall =
				frame_system::Call::remark_with_event { remark: vec![50] }.into();
			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&Action::DynamicDispatch { encoded_call: call.encode() },
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
			fund_account(&alice, 900_000_000, 1, Some(0));

			let starting_funds = Balances::free_balance(alice.clone());
			let call: <Test as frame_system::Config>::RuntimeCall =
				frame_system::Call::remark_with_event { remark: vec![50] }.into();

			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for::<(), _>(
				&alice,
				&Action::DynamicDispatch { encoded_call: call.encode() },
				1,
				|| Err("error".into()),
			);
			assert_err!(result, DispatchError::Other("error"));
			assert_eq!(starting_funds, Balances::free_balance(alice))
		})
	}
}
