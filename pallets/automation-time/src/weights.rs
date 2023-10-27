// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
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

//! Autogenerated weights for pallet_automation_time
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-08-07, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `dashing-dolphin`, CPU: `Intel(R) Xeon(R) E-2388G CPU @ 3.20GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("turing-dev"), DB CACHE: 1024

// Executed Command:
// ./oak-collator
// benchmark
// pallet
// --chain
// turing-dev
// --execution
// wasm
// --wasm-execution
// compiled
// --pallet
// pallet_automation_time
// --extrinsic
// *
// --repeat
// 20
// --steps
// 50
// --output
// ./automation_time-raw-weights.rs
// --template
// ./.maintain/frame-weight-template.hbs

// Summary:
//:schedule_xcmp_task_full 130_899_385
//:schedule_auto_compound_delegated_stake_task_full 95_776_000
//:schedule_dynamic_dispatch_task 72_653_946
//:schedule_dynamic_dispatch_task_full 82_034_398
//:cancel_scheduled_task_full 979_075_000
//:force_cancel_scheduled_task 27_445_000
//:force_cancel_scheduled_task_full 982_347_000
//:run_xcmp_task 41_572_000
//:run_auto_compound_delegated_stake_task 63_602_000
//:run_dynamic_dispatch_action 8_164_000
//:run_dynamic_dispatch_action_fail_decode 785_000
//:run_missed_tasks_many_found 311_838
//:run_missed_tasks_many_missing 294_793
//:run_tasks_many_found 3_768_036
//:run_tasks_many_missing 2_862_924
//:update_task_queue_overhead 2_761_000
//:append_to_missed_tasks 3_209_620
//:update_scheduled_task_queue 35_844_000
//:shift_missed_tasks 32_373_000

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_automation_time.
pub trait WeightInfo {
	fn schedule_xcmp_task_full(v: u32, ) -> Weight;
	fn schedule_auto_compound_delegated_stake_task_full() -> Weight;
	fn schedule_dynamic_dispatch_task(v: u32, ) -> Weight;
	fn schedule_dynamic_dispatch_task_full(v: u32, ) -> Weight;
	fn cancel_scheduled_task_full() -> Weight;
	fn force_cancel_scheduled_task() -> Weight;
	fn force_cancel_scheduled_task_full() -> Weight;
	fn cancel_task_by_schedule_as() -> Weight;
	fn run_xcmp_task() -> Weight;
	fn run_auto_compound_delegated_stake_task() -> Weight;
	fn run_dynamic_dispatch_action() -> Weight;
	fn run_dynamic_dispatch_action_fail_decode() -> Weight;
	fn run_missed_tasks_many_found(v: u32, ) -> Weight;
	fn run_missed_tasks_many_missing(v: u32, ) -> Weight;
	fn run_tasks_many_found(v: u32, ) -> Weight;
	fn run_tasks_many_missing(v: u32, ) -> Weight;
	fn update_task_queue_overhead() -> Weight;
	fn append_to_missed_tasks(v: u32, ) -> Weight;
	fn update_scheduled_task_queue() -> Weight;
	fn shift_missed_tasks() -> Weight;
}

/// Weights for pallet_automation_time using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	// Storage: AssetRegistry Metadata (r:1 w:0)
	// Proof Skipped: AssetRegistry Metadata (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens Accounts (r:2 w:2)
	// Proof: Tokens Accounts (max_values: None, max_size: Some(108), added: 2583, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens TotalIssuance (r:1 w:1)
	// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(28), added: 2503, mode: MaxEncodedLen)
	// Storage: System Account (r:3 w:1)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// The range of component `v` is `[1, 36]`.
	fn schedule_xcmp_task_full(v: u32, ) -> Weight {
		Weight::from_ref_time(130_899_385 as u64)
			// Standard Error: 13_282
			.saturating_add(Weight::from_ref_time(21_013_588 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().reads(11 as u64))
			.saturating_add(T::DbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
			.saturating_add(T::DbWeight::get().writes(5 as u64))
			.saturating_add(T::DbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:2 w:2)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn schedule_auto_compound_delegated_stake_task_full() -> Weight {
		Weight::from_ref_time(95_776_000 as u64)
			.saturating_add(T::DbWeight::get().reads(7 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:2 w:2)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[1, 36]`.
	fn schedule_dynamic_dispatch_task(v: u32, ) -> Weight {
		Weight::from_ref_time(72_653_946 as u64)
			// Standard Error: 45_934
			.saturating_add(Weight::from_ref_time(3_179_104 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
			.saturating_add(T::DbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:2 w:2)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[1, 36]`.
	fn schedule_dynamic_dispatch_task_full(v: u32, ) -> Weight {
		Weight::from_ref_time(82_034_398 as u64)
			// Standard Error: 16_577
			.saturating_add(Weight::from_ref_time(26_754_074 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
			.saturating_add(T::DbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: AutomationTime LastTimeSlot (r:1 w:0)
	// Proof Skipped: AutomationTime LastTimeSlot (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn cancel_scheduled_task_full() -> Weight {
		Weight::from_ref_time(979_075_000 as u64)
			.saturating_add(T::DbWeight::get().reads(39 as u64))
			.saturating_add(T::DbWeight::get().writes(37 as u64))
	}
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: AutomationTime LastTimeSlot (r:1 w:0)
	// Proof Skipped: AutomationTime LastTimeSlot (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn force_cancel_scheduled_task() -> Weight {
		Weight::from_ref_time(27_445_000 as u64)
			.saturating_add(T::DbWeight::get().reads(4 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: AutomationTime LastTimeSlot (r:1 w:0)
	// Proof Skipped: AutomationTime LastTimeSlot (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn force_cancel_scheduled_task_full() -> Weight {
		Weight::from_ref_time(982_347_000 as u64)
			.saturating_add(T::DbWeight::get().reads(39 as u64))
			.saturating_add(T::DbWeight::get().writes(37 as u64))
	}

	fn cancel_task_by_schedule_as() -> Weight {
		Weight::zero()
	}

	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: UnknownTokens ConcreteFungibleBalances (r:1 w:0)
	// Proof Skipped: UnknownTokens ConcreteFungibleBalances (max_values: None, max_size: None, mode: Measured)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	fn run_xcmp_task() -> Weight {
		Weight::from_ref_time(41_572_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
	}
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Account (r:1 w:1)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: ParachainStaking DelegatorState (r:1 w:1)
	// Proof Skipped: ParachainStaking DelegatorState (max_values: None, max_size: None, mode: Measured)
	// Storage: ParachainStaking DelegationScheduledRequests (r:1 w:0)
	// Proof Skipped: ParachainStaking DelegationScheduledRequests (max_values: None, max_size: None, mode: Measured)
	// Storage: Balances Locks (r:1 w:1)
	// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode: MaxEncodedLen)
	// Storage: ParachainStaking CandidateInfo (r:1 w:1)
	// Proof Skipped: ParachainStaking CandidateInfo (max_values: None, max_size: None, mode: Measured)
	// Storage: ParachainStaking TopDelegations (r:1 w:1)
	// Proof Skipped: ParachainStaking TopDelegations (max_values: None, max_size: None, mode: Measured)
	// Storage: ParachainStaking CandidatePool (r:1 w:1)
	// Proof Skipped: ParachainStaking CandidatePool (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: ParachainStaking Total (r:1 w:1)
	// Proof Skipped: ParachainStaking Total (max_values: Some(1), max_size: None, mode: Measured)
	fn run_auto_compound_delegated_stake_task() -> Weight {
		Weight::from_ref_time(63_602_000 as u64)
			.saturating_add(T::DbWeight::get().reads(9 as u64))
			.saturating_add(T::DbWeight::get().writes(7 as u64))
	}
	// Storage: Valve ValveClosed (r:1 w:0)
	// Proof Skipped: Valve ValveClosed (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: Valve ClosedPallets (r:1 w:0)
	// Proof Skipped: Valve ClosedPallets (max_values: None, max_size: None, mode: Measured)
	fn run_dynamic_dispatch_action() -> Weight {
		Weight::from_ref_time(8_164_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
	}
	fn run_dynamic_dispatch_action_fail_decode() -> Weight {
		Weight::from_ref_time(785_000 as u64)
	}
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[0, 1]`.
	fn run_missed_tasks_many_found(v: u32, ) -> Weight {
		Weight::from_ref_time(311_838 as u64)
			// Standard Error: 13_844
			.saturating_add(Weight::from_ref_time(18_771_761 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
			.saturating_add(T::DbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: AutomationTime AccountTasks (r:1 w:0)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[0, 1]`.
	fn run_missed_tasks_many_missing(v: u32, ) -> Weight {
		Weight::from_ref_time(294_793 as u64)
			// Standard Error: 4_008
			.saturating_add(Weight::from_ref_time(8_120_906 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: Valve ValveClosed (r:1 w:0)
	// Proof Skipped: Valve ValveClosed (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: Valve ClosedPallets (r:1 w:0)
	// Proof Skipped: Valve ClosedPallets (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[0, 1]`.
	fn run_tasks_many_found(v: u32, ) -> Weight {
		Weight::from_ref_time(3_768_036 as u64)
			// Standard Error: 18_719
			.saturating_add(Weight::from_ref_time(37_278_163 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().reads((3 as u64).saturating_mul(v as u64)))
			.saturating_add(T::DbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// The range of component `v` is `[0, 1]`.
	fn run_tasks_many_missing(v: u32, ) -> Weight {
		Weight::from_ref_time(2_862_924 as u64)
			// Standard Error: 14_386
			.saturating_add(Weight::from_ref_time(33_975 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().reads(1 as u64))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	fn update_task_queue_overhead() -> Weight {
		Weight::from_ref_time(2_761_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
	}
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime MissedQueueV2 (r:1 w:1)
	// Proof Skipped: AutomationTime MissedQueueV2 (max_values: Some(1), max_size: None, mode: Measured)
	/// The range of component `v` is `[0, 2]`.
	fn append_to_missed_tasks(v: u32, ) -> Weight {
		Weight::from_ref_time(3_209_620 as u64)
			// Standard Error: 57_546
			.saturating_add(Weight::from_ref_time(1_069_751 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: AutomationTime TaskQueueV2 (r:1 w:1)
	// Proof Skipped: AutomationTime TaskQueueV2 (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime MissedQueueV2 (r:1 w:1)
	// Proof Skipped: AutomationTime MissedQueueV2 (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn update_scheduled_task_queue() -> Weight {
		Weight::from_ref_time(35_844_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn shift_missed_tasks() -> Weight {
		Weight::from_ref_time(32_373_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	// Storage: AssetRegistry Metadata (r:1 w:0)
	// Proof Skipped: AssetRegistry Metadata (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens Accounts (r:2 w:2)
	// Proof: Tokens Accounts (max_values: None, max_size: Some(108), added: 2583, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens TotalIssuance (r:1 w:1)
	// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(28), added: 2503, mode: MaxEncodedLen)
	// Storage: System Account (r:3 w:1)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// The range of component `v` is `[1, 36]`.
	fn schedule_xcmp_task_full(v: u32, ) -> Weight {
		Weight::from_ref_time(130_899_385 as u64)
			// Standard Error: 13_282
			.saturating_add(Weight::from_ref_time(21_013_588 as u64).saturating_mul(v as u64))
			.saturating_add(RocksDbWeight::get().reads(11 as u64))
			.saturating_add(RocksDbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
			.saturating_add(RocksDbWeight::get().writes(5 as u64))
			.saturating_add(RocksDbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:2 w:2)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn schedule_auto_compound_delegated_stake_task_full() -> Weight {
		Weight::from_ref_time(95_776_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(7 as u64))
			.saturating_add(RocksDbWeight::get().writes(4 as u64))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:2 w:2)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[1, 36]`.
	fn schedule_dynamic_dispatch_task(v: u32, ) -> Weight {
		Weight::from_ref_time(72_653_946 as u64)
			// Standard Error: 45_934
			.saturating_add(Weight::from_ref_time(3_179_104 as u64).saturating_mul(v as u64))
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
			.saturating_add(RocksDbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:2 w:2)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[1, 36]`.
	fn schedule_dynamic_dispatch_task_full(v: u32, ) -> Weight {
		Weight::from_ref_time(82_034_398 as u64)
			// Standard Error: 16_577
			.saturating_add(Weight::from_ref_time(26_754_074 as u64).saturating_mul(v as u64))
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
			.saturating_add(RocksDbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: AutomationTime LastTimeSlot (r:1 w:0)
	// Proof Skipped: AutomationTime LastTimeSlot (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn cancel_scheduled_task_full() -> Weight {
		Weight::from_ref_time(979_075_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(39 as u64))
			.saturating_add(RocksDbWeight::get().writes(37 as u64))
	}
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: AutomationTime LastTimeSlot (r:1 w:0)
	// Proof Skipped: AutomationTime LastTimeSlot (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn force_cancel_scheduled_task() -> Weight {
		Weight::from_ref_time(27_445_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: AutomationTime LastTimeSlot (r:1 w:0)
	// Proof Skipped: AutomationTime LastTimeSlot (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:36 w:36)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn force_cancel_scheduled_task_full() -> Weight {
		Weight::from_ref_time(982_347_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(39 as u64))
			.saturating_add(RocksDbWeight::get().writes(37 as u64))
	}

	fn cancel_task_by_schedule_as() -> Weight {
		Weight::zero()
	}

	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: UnknownTokens ConcreteFungibleBalances (r:1 w:0)
	// Proof Skipped: UnknownTokens ConcreteFungibleBalances (max_values: None, max_size: None, mode: Measured)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:0)
	// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	fn run_xcmp_task() -> Weight {
		Weight::from_ref_time(41_572_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
	}
	// Storage: ParachainInfo ParachainId (r:1 w:0)
	// Proof: ParachainInfo ParachainId (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Account (r:1 w:1)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: ParachainStaking DelegatorState (r:1 w:1)
	// Proof Skipped: ParachainStaking DelegatorState (max_values: None, max_size: None, mode: Measured)
	// Storage: ParachainStaking DelegationScheduledRequests (r:1 w:0)
	// Proof Skipped: ParachainStaking DelegationScheduledRequests (max_values: None, max_size: None, mode: Measured)
	// Storage: Balances Locks (r:1 w:1)
	// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode: MaxEncodedLen)
	// Storage: ParachainStaking CandidateInfo (r:1 w:1)
	// Proof Skipped: ParachainStaking CandidateInfo (max_values: None, max_size: None, mode: Measured)
	// Storage: ParachainStaking TopDelegations (r:1 w:1)
	// Proof Skipped: ParachainStaking TopDelegations (max_values: None, max_size: None, mode: Measured)
	// Storage: ParachainStaking CandidatePool (r:1 w:1)
	// Proof Skipped: ParachainStaking CandidatePool (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: ParachainStaking Total (r:1 w:1)
	// Proof Skipped: ParachainStaking Total (max_values: Some(1), max_size: None, mode: Measured)
	fn run_auto_compound_delegated_stake_task() -> Weight {
		Weight::from_ref_time(63_602_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(9 as u64))
			.saturating_add(RocksDbWeight::get().writes(7 as u64))
	}
	// Storage: Valve ValveClosed (r:1 w:0)
	// Proof Skipped: Valve ValveClosed (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: Valve ClosedPallets (r:1 w:0)
	// Proof Skipped: Valve ClosedPallets (max_values: None, max_size: None, mode: Measured)
	fn run_dynamic_dispatch_action() -> Weight {
		Weight::from_ref_time(8_164_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
	}
	fn run_dynamic_dispatch_action_fail_decode() -> Weight {
		Weight::from_ref_time(785_000 as u64)
	}
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[0, 1]`.
	fn run_missed_tasks_many_found(v: u32, ) -> Weight {
		Weight::from_ref_time(311_838 as u64)
			// Standard Error: 13_844
			.saturating_add(Weight::from_ref_time(18_771_761 as u64).saturating_mul(v as u64))
			.saturating_add(RocksDbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
			.saturating_add(RocksDbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: AutomationTime AccountTasks (r:1 w:0)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[0, 1]`.
	fn run_missed_tasks_many_missing(v: u32, ) -> Weight {
		Weight::from_ref_time(294_793 as u64)
			// Standard Error: 4_008
			.saturating_add(Weight::from_ref_time(8_120_906 as u64).saturating_mul(v as u64))
			.saturating_add(RocksDbWeight::get().reads((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	// Storage: AutomationTime AccountTasks (r:1 w:1)
	// Proof Skipped: AutomationTime AccountTasks (max_values: None, max_size: None, mode: Measured)
	// Storage: Valve ValveClosed (r:1 w:0)
	// Proof Skipped: Valve ValveClosed (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: Valve ClosedPallets (r:1 w:0)
	// Proof Skipped: Valve ClosedPallets (max_values: None, max_size: None, mode: Measured)
	/// The range of component `v` is `[0, 1]`.
	fn run_tasks_many_found(v: u32, ) -> Weight {
		Weight::from_ref_time(3_768_036 as u64)
			// Standard Error: 18_719
			.saturating_add(Weight::from_ref_time(37_278_163 as u64).saturating_mul(v as u64))
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().reads((3 as u64).saturating_mul(v as u64)))
			.saturating_add(RocksDbWeight::get().writes((1 as u64).saturating_mul(v as u64)))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// The range of component `v` is `[0, 1]`.
	fn run_tasks_many_missing(v: u32, ) -> Weight {
		Weight::from_ref_time(2_862_924 as u64)
			// Standard Error: 14_386
			.saturating_add(Weight::from_ref_time(33_975 as u64).saturating_mul(v as u64))
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	fn update_task_queue_overhead() -> Weight {
		Weight::from_ref_time(2_761_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
	}
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	// Storage: AutomationTime MissedQueueV2 (r:1 w:1)
	// Proof Skipped: AutomationTime MissedQueueV2 (max_values: Some(1), max_size: None, mode: Measured)
	/// The range of component `v` is `[0, 2]`.
	fn append_to_missed_tasks(v: u32, ) -> Weight {
		Weight::from_ref_time(3_209_620 as u64)
			// Standard Error: 57_546
			.saturating_add(Weight::from_ref_time(1_069_751 as u64).saturating_mul(v as u64))
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: AutomationTime TaskQueueV2 (r:1 w:1)
	// Proof Skipped: AutomationTime TaskQueueV2 (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime MissedQueueV2 (r:1 w:1)
	// Proof Skipped: AutomationTime MissedQueueV2 (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn update_scheduled_task_queue() -> Weight {
		Weight::from_ref_time(35_844_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: AutomationTime ScheduledTasksV3 (r:1 w:1)
	// Proof Skipped: AutomationTime ScheduledTasksV3 (max_values: None, max_size: None, mode: Measured)
	fn shift_missed_tasks() -> Weight {
		Weight::from_ref_time(32_373_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}