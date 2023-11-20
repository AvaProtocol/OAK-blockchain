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

use frame_support::traits::Get;
use frame_system::RawOrigin;
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

#[derive(Clone)]
pub struct FeePayment<T: Config> {
	pub asset_location: MultiLocation,
	pub amount: MultiBalanceOf<T>,
	pub is_local: bool,
}

pub struct FeeHandler<T: Config, TR> {
	owner: T::AccountId,
	pub schedule_fee: FeePayment<T>,
	pub execution_fee: Option<FeePayment<T>>,
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
	fn ensure_can_withdraw(
		&self,
		asset_location: MultiLocation,
		amount: MultiBalanceOf<T>,
	) -> Result<(), DispatchError> {
		if amount.is_zero() {
			return Ok(())
		}

		let currency_id = T::CurrencyIdConvert::convert(asset_location)
			.ok_or("IncoveribleMultilocation")?
			.into();
		let free_balance = T::MultiCurrency::free_balance(currency_id, &self.owner);
		let min_balance = T::MultiCurrency::minimum_balance(currency_id);

		free_balance
			.checked_sub(&amount)
			.and_then(|balance_minus_fee| balance_minus_fee.checked_sub(&min_balance))
			.ok_or(DispatchError::Token(BelowMinimum))?;

		T::MultiCurrency::ensure_can_withdraw(currency_id, &self.owner, amount)?;

		Ok(())
	}

	/// Ensure the fee can be paid.
	fn can_pay_fee(&self) -> Result<(), DispatchError> {
		match &self.execution_fee {
			Some(exec_fee) if exec_fee.is_local => {
				// If the locations of schedule_fee and execution_fee are equal,
				// we need to add the fees to check whether they are sufficient,
				// otherwise check them separately.
				let exec_fee_location = exec_fee
					.asset_location
					.reanchored(&T::SelfLocation::get(), T::UniversalLocation::get())
					.map_err(|_| Error::<T>::CannotReanchor)?;

				let schedule_fee_location = self
					.schedule_fee
					.asset_location
					.reanchored(&T::SelfLocation::get(), T::UniversalLocation::get())
					.map_err(|_| Error::<T>::CannotReanchor)?;

				if exec_fee_location == schedule_fee_location {
					let fee = self.schedule_fee.amount.saturating_add(exec_fee.amount);
					Self::ensure_can_withdraw(self, exec_fee.asset_location, fee)?;
				} else {
					Self::ensure_can_withdraw(
						self,
						self.schedule_fee.asset_location,
						self.schedule_fee.amount,
					)?;
					Self::ensure_can_withdraw(self, exec_fee.asset_location, exec_fee.amount)?;
				}
			},
			_ => {
				Self::ensure_can_withdraw(
					self,
					self.schedule_fee.asset_location,
					self.schedule_fee.amount,
				)?;
			},
		}

		Ok(())
	}

	/// Withdraw the fee.
	fn withdraw_fee(&self) -> Result<(), DispatchError> {
		log::debug!(target: "FeeHandler", "FeeHandler::withdraw_fee, self.schedule_fee.asset_location: {:?}, self.schedule_fee.amount: {:?}",
			self.schedule_fee.asset_location, self.schedule_fee.amount);
		// Withdraw schedule fee
		// When the expected deduction amount, schedule_fee_amount, is not equal to zero, execute the withdrawal process;
		// otherwise, there’s no need to deduct.
		if !self.schedule_fee.amount.is_zero() {
			let currency_id = T::CurrencyIdConvert::convert(self.schedule_fee.asset_location)
				.ok_or("InconvertibleMultilocation")?;

			T::MultiCurrency::withdraw(currency_id.into(), &self.owner, self.schedule_fee.amount)
				.map_err(|_| DispatchError::Token(BelowMinimum))?;

			TR::take_revenue(MultiAsset {
				id: AssetId::Concrete(self.schedule_fee.asset_location),
				fun: Fungibility::Fungible(self.schedule_fee.amount.saturated_into()),
			});
		}

		// Withdraw execution fee
		if let Some(execution_fee) = &self.execution_fee {
			if execution_fee.is_local {
				log::debug!(target: "FeeHandler", "FeeHandler::withdraw_fee, self.execution_fee.asset_location: {:?}, self.execution_fee.amount: {:?}",
					execution_fee.asset_location, execution_fee.amount);
				let currency_id = T::CurrencyIdConvert::convert(execution_fee.asset_location)
					.ok_or("InconvertibleMultilocation")?;

				let execution_fee_amount = execution_fee.amount;
				// When the expected deduction amount, execution_fee_amount, is not equal to zero, execute the withdrawal process;
				// otherwise, there’s no need to deduct.
				if !execution_fee_amount.is_zero() {
					T::XcmpTransactor::pay_xcm_fee(
						currency_id,
						self.owner.clone(),
						execution_fee_amount.saturated_into(),
					)?;
				}
			}
		}

		Ok(())
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

		let schedule_fee = FeePayment {
			asset_location: schedule_fee_location,
			amount: schedule_fee_amount.saturated_into(),
			is_local: true,
		};

		let execution_fee = match action.clone() {
			Action::XCMP { execution_fee, instruction_sequence, .. } => {
				let location = MultiLocation::try_from(execution_fee.asset_location)
					.map_err(|()| Error::<T>::BadVersion)?;
				let amount =
					execution_fee.amount.saturating_mul(executions.into()).saturated_into();
				Some(FeePayment {
					asset_location: location,
					amount,
					is_local: instruction_sequence ==
						InstructionSequence::PayThroughSovereignAccount,
				})
			},
			_ => None,
		};

		Ok(Self {
			owner: owner.clone(),
			schedule_fee,
			execution_fee,
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
	use crate::{mock::*, Action, AssetPayment, Weight};
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
			let mut has_callback_run = false;
			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&Action::DynamicDispatch { encoded_call: call.encode() },
				1,
				|| {
					has_callback_run = true;
					Ok("called")
				},
			);
			assert_eq!(result.expect("success"), "called");
			assert_eq!(has_callback_run, true);
			assert!(starting_funds > Balances::free_balance(alice))
		})
	}

	#[test]
	fn call_pay_checked_fees_for_with_normal_flow_and_enough_execution_fee_success() {
		new_test_ext(0).execute_with(|| {
			let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
			let alice = AccountId32::new(ALICE);
			let mut has_callback_run = false;
			get_multi_xcmp_funds(alice.clone());

			let action = Action::XCMP {
				destination,
				schedule_fee: NATIVE_LOCATION,
				execution_fee: AssetPayment { asset_location: destination.into(), amount: 10 },
				encoded_call: vec![3, 4, 5],
				encoded_call_weight: Weight::from_parts(100_000, 0),
				overall_weight: Weight::from_parts(200_000, 0),
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
			};

			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&action,
				1,
				|| {
					has_callback_run = true;
					Ok("called")
				},
			);
			assert_eq!(result.expect("success"), "called");
			assert_eq!(has_callback_run, true);
		})
	}

	#[test]
	fn call_pay_checked_fees_for_with_normal_flow_and_foreign_schedule_fee_success() {
		new_test_ext(0).execute_with(|| {
			let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
			let alice = AccountId32::new(ALICE);
			let mut has_callback_run = false;
			let _ = Currencies::update_balance(
				RawOrigin::Root.into(),
				alice.clone(),
				FOREIGN_CURRENCY_ID,
				XmpFee::get() as i64,
			);
			fund_account(&alice, 900_000_000, 1, Some(0));

			let action = Action::XCMP {
				destination,
				schedule_fee: destination.into(),
				execution_fee: AssetPayment { asset_location: NATIVE_LOCATION.into(), amount: 10 },
				encoded_call: vec![3, 4, 5],
				encoded_call_weight: Weight::from_parts(100_000, 0),
				overall_weight: Weight::from_parts(200_000, 0),
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
			};

			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&action,
				1,
				|| {
					has_callback_run = true;
					Ok("called")
				},
			);
			assert_eq!(result.expect("success"), "called");
			assert_eq!(has_callback_run, true);
		})
	}

	#[test]
	fn call_pay_checked_fees_for_with_normal_flow_and_foreign_schedule_fee_will_throw_insufficent_balance(
	) {
		new_test_ext(0).execute_with(|| {
			let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
			let alice = AccountId32::new(ALICE);
			fund_account(&alice, 900_000_000, 1, Some(0));

			let action = Action::XCMP {
				destination,
				schedule_fee: destination.clone().into(),
				execution_fee: AssetPayment { asset_location: NATIVE_LOCATION.into(), amount: 10 },
				encoded_call: vec![3, 4, 5],
				encoded_call_weight: Weight::from_parts(100_000, 0),
				overall_weight: Weight::from_parts(200_000, 0),
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
			};

			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&action,
				1,
				|| Ok(()),
			);
			assert_err!(result, Error::<Test>::InsufficientBalance);
		})
	}

	#[test]
	fn call_pay_checked_fees_for_with_normal_flow_and_insufficent_execution_fee_will_fail() {
		new_test_ext(0).execute_with(|| {
			let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
			let alice = AccountId32::new(ALICE);
			fund_account(&alice, 900_000_000, 1, Some(0));

			let action = Action::XCMP {
				destination,
				schedule_fee: NATIVE_LOCATION,
				execution_fee: AssetPayment { asset_location: destination.into(), amount: 10 },
				encoded_call: vec![3, 4, 5],
				encoded_call_weight: Weight::from_parts(100_000, 0),
				overall_weight: Weight::from_parts(200_000, 0),
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
			};

			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&action,
				1,
				|| Ok(()),
			);
			assert_err!(result, Error::<Test>::InsufficientBalance);
		})
	}

	#[test]
	fn call_pay_checked_fees_for_with_alternate_flow_and_no_execution_fee_success() {
		new_test_ext(0).execute_with(|| {
			let destination = MultiLocation::new(1, X1(Parachain(PARA_ID)));
			let alice = AccountId32::new(ALICE);
			let mut has_callback_run = false;
			fund_account(&alice, 900_000_000, 1, Some(0));

			let action = Action::XCMP {
				destination,
				schedule_fee: NATIVE_LOCATION,
				execution_fee: AssetPayment { asset_location: destination.into(), amount: 10 },
				encoded_call: vec![3, 4, 5],
				encoded_call_weight: Weight::from_parts(100_000, 0),
				overall_weight: Weight::from_parts(200_000, 0),
				schedule_as: None,
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			};

			let result = <Test as crate::Config>::FeeHandler::pay_checked_fees_for(
				&alice,
				&action,
				1,
				|| {
					has_callback_run = true;
					Ok("called")
				},
			);
			assert_eq!(result.expect("success"), "called");
			assert_eq!(has_callback_run, true);
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
