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

pub mod time {
	use primitives::BlockNumber;

	/// This determines the average expected block time that we are targeting.
	/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
	/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
	/// up by `pallet_aura` to implement `fn slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLISECS_PER_BLOCK: u64 = 12000;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	// Time is measured by number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}

pub mod currency {
	use primitives::Balance;

	pub const TOKEN_DECIMALS: u32 = 10;
	const TOKEN_BASE: u128 = 10;
	// Unit = the base number of indivisible units for balances
	pub const UNIT: Balance = TOKEN_BASE.pow(TOKEN_DECIMALS); // 10_000_000_000
	pub const DOLLAR: Balance = UNIT; // 10_000_000_000
	pub const CENT: Balance = DOLLAR / 100; // 100_000_000
	pub const MILLICENT: Balance = CENT / 1_000; // 100_000

	/// The existential deposit. Set to 1/100 of the Connected Relay Chain.
	pub const EXISTENTIAL_DEPOSIT: Balance = CENT;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 2_000 * CENT + (bytes as Balance) * 100 * MILLICENT
	}
}

pub mod fees {
	use frame_support::parameter_types;
	use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
	use sp_runtime::{traits::Bounded, FixedPointNumber, Perquintill};

	parameter_types! {
		/// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
		/// than this will decrease the weight and more will increase.
		pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(1);
		/// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
		/// change the fees more rapidly.
		pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
		/// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
		/// that combined with `AdjustmentVariable`, we can recover from the minimum.
		/// See `multiplier_can_grow_from_zero`.
		pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000u128);
		pub MaximumMultiplier: Multiplier = Bounded::max_value();
	}

	/// Parameterized slow adjusting fee updated based on
	/// https://w3f-research.readthedocs.io/en/latest/polkadot/overview/2-token-economics.html#-2.-slow-adjusting-mechanism // editorconfig-checker-disable-line
	///
	/// The adjustment algorithm boils down to:
	///
	/// diff = (previous_block_weight - target) / maximum_block_weight
	/// next_multiplier = prev_multiplier * (1 + (v * diff) + ((v * diff)^2 / 2))
	/// assert(next_multiplier > min)
	///     where: v is AdjustmentVariable
	///            target is TargetBlockFullness
	///            min is MinimumMultiplier
	pub type SlowAdjustingFeeUpdate<R> = TargetedFeeAdjustment<
		R,
		TargetBlockFullness,
		AdjustmentVariable,
		MinimumMultiplier,
		MaximumMultiplier,
	>;
}

pub mod weight_ratios {
	use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};
	use sp_runtime::Perbill;

	/// We use at most 5% of the block weight running scheduled tasks during `on_initialize`.
	pub const SCHEDULED_TASKS_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

	/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
	/// used to limit the maximal weight of a single extrinsic.
	pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

	/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
	/// `Operational` extrinsics.
	pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

	/// We allow for 0.5 seconds of compute with a 12 second average block time.
	pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
		WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
		polkadot_primitives::v2::MAX_POV_SIZE as u64,
	);
}
