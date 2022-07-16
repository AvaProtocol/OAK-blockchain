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
use crate::{BalanceOf, Config};

use sp_runtime::{
	traits::{CheckedSub, Zero},
	DispatchError,
	TokenError::BelowMinimum,
};
use sp_std::marker::PhantomData;

use frame_support::traits::{Currency, ExistenceRequirement, OnUnbalanced, WithdrawReasons};

type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

/// Handle withdrawing, refunding and depositing of transaction fees.
pub trait HandleFees<T: Config> {
	/// Ensure the fee can be paid.
	fn can_pay_fee(who: &T::AccountId, fee: BalanceOf<T>) -> Result<(), DispatchError>;

	/// Once the task has been scheduled we need to charge for the execution cost.
	fn withdraw_fee(who: &T::AccountId, fee: BalanceOf<T>) -> Result<(), DispatchError>;
}

pub struct FeeHandler<OU>(PhantomData<OU>);

/// Implements the transaction payment for a pallet implementing the `Currency`
/// trait (eg. the pallet_balances) using an unbalance handler (implementing
/// `OnUnbalanced`).
impl<T, OU> HandleFees<T> for FeeHandler<OU>
where
	T: Config,
	OU: OnUnbalanced<NegativeImbalanceOf<T>>,
{
	// Ensure the fee can be paid.
	fn can_pay_fee(who: &T::AccountId, fee: BalanceOf<T>) -> Result<(), DispatchError> {
		if fee.is_zero() {
			return Ok(())
		}

		let free_balance = T::Currency::free_balance(who);
		let new_amount =
			free_balance.checked_sub(&fee).ok_or(DispatchError::Token(BelowMinimum))?;
		T::Currency::ensure_can_withdraw(who, fee, WithdrawReasons::FEE, new_amount)?;

		Ok(())
	}

	/// Withdraw the fee.
	fn withdraw_fee(who: &T::AccountId, fee: BalanceOf<T>) -> Result<(), DispatchError> {
		if fee.is_zero() {
			return Ok(())
		}

		let withdraw_reason = WithdrawReasons::FEE;

		match T::Currency::withdraw(who, fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
			Ok(imbalance) => {
				OU::on_unbalanceds(Some(imbalance).into_iter());
				Ok(())
			},
			Err(_) => Err(DispatchError::Token(BelowMinimum)),
		}
	}
}
