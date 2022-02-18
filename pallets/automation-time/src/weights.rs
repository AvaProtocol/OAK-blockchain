
//! Autogenerated weights for `pallet_automation_time`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-01-20, STEPS: `1`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/neumann-collator
// benchmark
// --chain
// dev
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
// --raw
// --output
// ./pallets/automation-time/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_automation_time.
pub trait WeightInfo {
	fn schedule_notify_task_new_slot() -> Weight;
	fn schedule_notify_task_existing_slot() -> Weight;
	fn schedule_native_transfer_task_existing_slot() -> Weight;
	fn cancel_scheduled_task() -> Weight;
	fn cancel_scheduled_task_full() -> Weight;
	fn cancel_overflow_task() -> Weight;
	fn force_cancel_scheduled_task() -> Weight;
	fn force_cancel_scheduled_task_full() -> Weight;
	fn force_cancel_overflow_task() -> Weight;
}

/// Weight functions for `pallet_automation_time`.
pub struct AutomationWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for AutomationWeight<T> {
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:1)
	fn schedule_notify_task_new_slot() -> Weight {
		(16_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:1)
	fn schedule_notify_task_existing_slot() -> Weight {
		(18_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:1)
	// TODO: this is placeholder, jzhou will fix
	fn schedule_native_transfer_task_existing_slot() -> Weight {
		(18_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:1)
	fn cancel_scheduled_task() -> Weight {
		(16_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:1)
	fn cancel_scheduled_task_full() -> Weight {
		(16_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:0)
	// Storage: AutomationTime OverlflowTasks (r:1 w:1)
	fn cancel_overflow_task() -> Weight {
		(17_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:1)
	fn force_cancel_scheduled_task() -> Weight {
		(15_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:1)
	fn force_cancel_scheduled_task_full() -> Weight {
		(16_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: AutomationTime Tasks (r:1 w:1)
	// Storage: AutomationTime ScheduledTasks (r:1 w:0)
	// Storage: AutomationTime OverlflowTasks (r:1 w:1)
	fn force_cancel_overflow_task() -> Weight {
		(17_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}
