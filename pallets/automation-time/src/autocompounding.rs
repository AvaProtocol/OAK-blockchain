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
use sp_std::vec::Vec;

#[derive(Clone)]
pub struct IntervalRow {
	interval_earnings_after_fee: i128,
	stake: i128,
	ownership: f64,
}

// API signature should be
// calculateOptimalAutostaking(principal: i128, collator: AccountId)
// - principal passed directly through
// - collator_stake: chain state parachainStaking.candidateInfo(AccountId).totalCounted
// - fee: TBD; provided by autocompounding
// - duration: constant passed in; 90 to start
// - daily_collator_awards: chain state for balances.totalIssuance * 0.025 / 24 (# of collators)
pub fn do_calculate_optimal_autostaking(
	principal: i128,
	collator_stake: i128,
	fee: i128,
	duration: i32,
	daily_collator_awards: i128,
) -> i32 {
	let initial_interval_row = IntervalRow {
		interval_earnings_after_fee: 0,
		stake: principal,
		ownership: principal as f64 / collator_stake as f64,
	};

	let mut best_earnings = 0;
	let mut best_period = 0;
	for period in 1..=duration {
		let mut interval_table = Vec::new();
		interval_table.push(initial_interval_row.clone());

		for interval in 1..=(duration / period) {
			let IntervalRow { stake: previous_stake, ownership: previous_ownership, .. } =
				interval_table[(interval - 1) as usize];

			let interval_earnings =
				(previous_ownership * (daily_collator_awards * period as i128) as f64) as i128;
			let interval_earnings_after_fee = interval_earnings - fee;
			let stake = previous_stake + interval_earnings_after_fee;
			let ownership = stake as f64 / (collator_stake - principal + stake) as f64;

			interval_table.push(IntervalRow { interval_earnings_after_fee, stake, ownership });
		}

		let period_earnings_after_fee: i128 =
			interval_table.iter().map(|row| row.interval_earnings_after_fee).sum();
		let extra_days = duration % period;
		let remainder_earnings = (interval_table.last().unwrap().ownership *
			(extra_days as i128 * daily_collator_awards) as f64) as i128;
		let total_earnings = period_earnings_after_fee + remainder_earnings;

		if total_earnings > best_earnings {
			best_earnings = total_earnings;
			best_period = period;
		}
	}

	return best_period
}

#[cfg(test)]
mod tests {
	use super::*;

	const PLANCK: i128 = 10_000_000_000;

	const AVERAGE_STAKING_DURATION: i32 = 90;
	const ANNUAL_REWARDS_PER_COLLATOR: i128 = 1_041_667 * PLANCK;
	const DAILY_COLLATOR_AWARDS: i128 = ANNUAL_REWARDS_PER_COLLATOR / 365;

	#[test]
	fn test_1() {
		let principal = 50 * PLANCK;
		let collator_stake = 500_000 * PLANCK;
		let fee = 1 * PLANCK;

		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			AVERAGE_STAKING_DURATION,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, 32)
	}

	#[test]
	fn test_2() {
		let principal = 250_000 * PLANCK;
		let collator_stake = 500_000 * PLANCK;
		let fee = 1 * PLANCK;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			AVERAGE_STAKING_DURATION,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, 1)
	}

	#[test]
	fn test_3() {
		let principal = 250_000 * PLANCK;
		let collator_stake = 500_000 * PLANCK;
		let fee = 50 * PLANCK;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			AVERAGE_STAKING_DURATION,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, 5)
	}

	#[test]
	fn test_4() {
		let principal = 250_000 * PLANCK;
		let collator_stake = 500_000 * PLANCK;
		let fee = 10 * PLANCK;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			365,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, 3)
	}

	#[test]
	fn test_5() {
		let principal = 40_000 * PLANCK;
		let collator_stake = 2_000_000 * PLANCK;
		let fee = 35 * PLANCK;
		let result =
			do_calculate_optimal_autostaking(principal, collator_stake, fee, 180, 2_900 * PLANCK);
		assert_eq!(result, 31)
	}
}
