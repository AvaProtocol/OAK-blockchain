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
use sp_std::{vec, vec::Vec};

#[derive(Clone)]
pub struct IntervalRow {
	interval_earnings_after_fee: i128,
	stake: i128,
	ownership: f64,
}

pub fn do_calculate_optimal_autostaking(
	principal: i128,
	collator_stake: i128,
	fee: i128,
	duration: i32,
	daily_collator_awards: i128,
) -> (i32, f64) {
	let initial_interval_row = IntervalRow {
		interval_earnings_after_fee: 0,
		stake: principal,
		ownership: principal as f64 / collator_stake as f64,
	};

	let mut best_earnings = 0;
	let mut best_period = 0;
	let mut best_apy = 0.0;
	for period in 1..=duration {
		let mut interval_table: Vec<IntervalRow> = vec![initial_interval_row.clone()];

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
		let remainder_earnings = (interval_table.last().unwrap().ownership
			* (extra_days as i128 * daily_collator_awards) as f64) as i128;
		let total_earnings = period_earnings_after_fee + remainder_earnings;

		if total_earnings > best_earnings {
			best_earnings = total_earnings;
			best_period = period;
			best_apy = (total_earnings as f64 / principal as f64) * (365_f64 / duration as f64);
		}
	}

	(best_period, best_apy)
}

#[cfg(test)]
mod tests {
	use super::*;

	const DOLLAR: i128 = 10_000_000_000;

	const AVERAGE_STAKING_DURATION: i32 = 90;
	const ANNUAL_REWARDS_PER_COLLATOR: i128 = 1_041_667 * DOLLAR;
	const DAILY_COLLATOR_AWARDS: i128 = ANNUAL_REWARDS_PER_COLLATOR / 365;

	// The specific tests below do not represent any specifc scenarios and are only an assortment
	// selected to adjust enough of the inputs to completely validate the calculation.

	#[test]
	fn test_1() {
		let principal = 50 * DOLLAR;
		let collator_stake = 500_000 * DOLLAR;
		let fee = 1 * DOLLAR;

		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			AVERAGE_STAKING_DURATION,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, (32, 2.255207252238667))
	}

	#[test]
	fn test_2() {
		let principal = 250_000 * DOLLAR;
		let collator_stake = 500_000 * DOLLAR;
		let fee = 1 * DOLLAR;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			AVERAGE_STAKING_DURATION,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, (1, 2.322886915924501))
	}

	#[test]
	fn test_3() {
		let principal = 250_000 * DOLLAR;
		let collator_stake = 500_000 * DOLLAR;
		let fee = 50 * DOLLAR;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			AVERAGE_STAKING_DURATION,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, (5, 2.2971572933834423))
	}

	#[test]
	fn test_4() {
		let principal = 250_000 * DOLLAR;
		let collator_stake = 500_000 * DOLLAR;
		let fee = 10 * DOLLAR;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			365,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, (3, 2.8130749814046747))
	}

	#[test]
	fn test_5() {
		let principal = 40_000 * DOLLAR;
		let collator_stake = 2_000_000 * DOLLAR;
		let fee = 35 * DOLLAR;
		let result =
			do_calculate_optimal_autostaking(principal, collator_stake, fee, 180, 2_900 * DOLLAR);
		assert_eq!(result, (31, 0.5786527967406478))
	}

	#[test]
	fn test_6() {
		let principal = 10_000 * DOLLAR;
		let collator_stake = 500_000 * DOLLAR;
		let fee = 50 * DOLLAR;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			90,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, (19, 2.4399220232480014))
	}

	#[test]
	fn test_7() {
		let principal = 490_000 * DOLLAR;
		let collator_stake = 500_000 * DOLLAR;
		let fee = 10 * DOLLAR;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			180,
			DAILY_COLLATOR_AWARDS,
		);
		assert_eq!(result, (13, 2.0951444282821376))
	}

	#[test]
	fn test_8() {
		let principal = 50 * DOLLAR;
		let collator_stake = 250_000 * DOLLAR;
		let fee = 1 * DOLLAR;
		let daily_collator_awards = 500_000 / 365 * DOLLAR;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			360,
			daily_collator_awards,
		);
		assert_eq!(result, (28, 4.65808623150761))
	}

	#[test]
	fn test_9() {
		// current issuance at some point
		let money_supply: i128 = 580_000_000_099_999_996;

		let principal = 5_000 * DOLLAR;
		let collator_stake = 20_000_000_000_000_000;
		let fee = 1 * DOLLAR;
		let daily_collator_awards = (money_supply as f64 * 0.025) as i128 / 24 / 365;
		let result = do_calculate_optimal_autostaking(
			principal,
			collator_stake,
			fee,
			90,
			daily_collator_awards,
		);
		assert_eq!(result, (46, 0.029450359762913332))
	}
}
