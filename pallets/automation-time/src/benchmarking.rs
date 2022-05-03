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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use pallet_timestamp;
use sp_runtime::traits::Saturating;

use crate::Pallet as AutomationTime;

const SEED: u32 = 0;
// existential deposit multiplier
const ED_MULTIPLIER: u32 = 10;

fn schedule_notify_tasks<T: Config>(owner: T::AccountId, times: Vec<u64>, count: u32) -> T::Hash {
	let transfer_amount =
		T::NativeTokenExchange::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
	T::NativeTokenExchange::deposit_creating(&owner, transfer_amount.clone());
	let time_moment: u32 = times[0].saturated_into();
	<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
	let mut task_id: T::Hash = T::Hash::default();

	for i in 0..count {
		let provided_id: Vec<u8> = vec![(i/256).try_into().unwrap(), (i%256).try_into().unwrap()];
		task_id =
			AutomationTime::<T>::schedule_task(owner.clone(), provided_id.clone(), times.clone()).unwrap();
		let task = Task::<T>::create_event_task(owner.clone(), provided_id, times.clone().try_into().unwrap(), vec![4, 5, 6]);
		<Tasks<T>>::insert(task_id, task);
	}
	task_id
}

fn schedule_transfer_tasks<T: Config>(owner: T::AccountId, time: u64, count: u32) -> T::Hash {
	let time_moment: u32 = time.saturated_into();
	<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
	let mut task_id: T::Hash = T::Hash::default();
	let recipient: T::AccountId = account("recipient", 0, SEED);
	let amount: BalanceOf<T> =
		T::NativeTokenExchange::minimum_balance().saturating_mul(ED_MULTIPLIER.into());

	for i in 0..count {
		let provided_id: Vec<u8> = vec![(i/256).try_into().unwrap(), (i%256).try_into().unwrap()];
		task_id =
			AutomationTime::<T>::schedule_task(owner.clone(), provided_id.clone(), vec![time]).unwrap();
		let task = Task::<T>::create_native_transfer_task(
			owner.clone(),
			provided_id,
			vec![time].try_into().unwrap(),
			recipient.clone(),
			amount.clone(),
		);
		<Tasks<T>>::insert(task_id, task);
	}
	task_id
}

benchmarks! {
	schedule_notify_task_empty {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 7200;
		let time_moment: u32 = time.try_into().unwrap();
		<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
		let transfer_amount = T::NativeTokenExchange::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		T::NativeTokenExchange::deposit_creating(&caller, transfer_amount.clone());
	}: schedule_notify_task(RawOrigin::Signed(caller), vec![10], vec![time], vec![4, 5])

	schedule_notify_task_full {
		let v in 1 .. T::MaxExecutionTimes::get();

		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 7200;

		let mut times: Vec<u64> = vec![];
		for i in 0..v {
			let hour: u64 = (3600 * (i + 1)).try_into().unwrap();
			times.push(hour);
		}

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), times.clone(), T::MaxTasksPerSlot::get() - 1);
		let provided_id: Vec<u8> = vec![(T::MaxTasksPerSlot::get()/256).try_into().unwrap(), (T::MaxTasksPerSlot::get()%256).try_into().unwrap()];
		let transfer_amount = T::NativeTokenExchange::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		T::NativeTokenExchange::deposit_creating(&caller, transfer_amount.clone());
	}: schedule_notify_task(RawOrigin::Signed(caller), provided_id, times, vec![4, 5])

	schedule_native_transfer_task_empty{
		let caller: T::AccountId = account("caller", 0, SEED);
		let recipient: T::AccountId = account("to", 0, SEED);
		let time: u64 = 7200;
		let transfer_amount = T::NativeTokenExchange::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		let time_moment: u32 = time.try_into().unwrap();
		<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
		T::NativeTokenExchange::deposit_creating(&caller, transfer_amount.clone());
	}: schedule_native_transfer_task(RawOrigin::Signed(caller), vec![10], vec![time], recipient, transfer_amount)

	schedule_native_transfer_task_full{
		let v in 1 .. T::MaxExecutionTimes::get();

		let caller: T::AccountId = account("caller", 0, SEED);
		let recipient: T::AccountId = account("to", 0, SEED);
		let transfer_amount = T::NativeTokenExchange::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		T::NativeTokenExchange::deposit_creating(&caller, transfer_amount.clone());

		let mut times: Vec<u64> = vec![];
		for i in 0..v {
			let hour: u64 = (3600 * (i + 1)).try_into().unwrap();
			times.push(hour);
		}

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), times.clone(), T::MaxTasksPerSlot::get() - 1);
		let provided_id: Vec<u8> = vec![(T::MaxTasksPerSlot::get()/256).try_into().unwrap(), (T::MaxTasksPerSlot::get()%256).try_into().unwrap()];
	}: schedule_native_transfer_task(RawOrigin::Signed(caller), provided_id, times, recipient, transfer_amount)

	cancel_scheduled_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let mut times: Vec<u64> = vec![];

		for i in 0..T::MaxExecutionTimes::get() {
			let hour: u64 = (3600 * (i + 1)).try_into().unwrap();
			times.push(hour);
		}

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), times, T::MaxTasksPerSlot::get());
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	force_cancel_scheduled_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 10800;

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), vec![time], 1);
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_scheduled_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let mut times: Vec<u64> = vec![];

		for i in 0..T::MaxExecutionTimes::get() {
			let hour: u64 = (3600 * (i + 1)).try_into().unwrap();
			times.push(hour);
		}

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), times, T::MaxTasksPerSlot::get());
	}: force_cancel_task(RawOrigin::Root, task_id)

	run_notify_task {
		let message = b"hello there".to_vec();
	}: { AutomationTime::<T>::run_notify_task(message) }

	run_native_transfer_task {
		let starting_multiplier: u32 = 20;
		let amount_starting: BalanceOf<T> = T::NativeTokenExchange::minimum_balance().saturating_mul(starting_multiplier.into());
		let caller: T::AccountId = account("caller", 0, SEED);
		T::NativeTokenExchange::deposit_creating(&caller, amount_starting.clone());
		let time: u64 = 10800;
		let recipient: T::AccountId = account("recipient", 0, SEED);
		let amount_transferred: BalanceOf<T> = T::NativeTokenExchange::minimum_balance().saturating_mul(ED_MULTIPLIER.into());

		let task_id: T::Hash = schedule_transfer_tasks::<T>(caller.clone(), time, 1);
	}: { AutomationTime::<T>::run_native_transfer_task(caller, recipient, amount_transferred, task_id) }

	/*
	* This section is to test run_missed_tasks.
	* run_missed_tasks_many_found: measuring many existing tasks for linear progression
	* run_missed_tasks_many_missing: measuring many non existing tasks for linear progression
	*/
	run_missed_tasks_many_found {
		let v in 0 .. 1;

		let weight_left = 50_000_000_000;
		let mut missed_tasks = vec![];
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u32 = 10800;

		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), vec![time.into()]).unwrap();
			let task = Task::<T>::create_event_task(caller.clone(), provided_id, vec![time.into()].try_into().unwrap(), vec![4, 5, 6]);
			let missed_task = MissedTask::<T>::create_missed_task(task_id, time.into());
			<Tasks<T>>::insert(task_id, task);
			missed_tasks.push(missed_task)
		}
	}: { AutomationTime::<T>::run_missed_tasks(missed_tasks, weight_left) }

	run_missed_tasks_many_missing {
		let v in 0 .. 1;

		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 10800;
		let mut missed_tasks = vec![];
		let weight_left = 500_000_000_000;

		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::generate_task_id(caller.clone(), provided_id);
			let missed_task = MissedTask::<T>::create_missed_task(task_id, time.into());
			missed_tasks.push(missed_task)
		}
	}: { AutomationTime::<T>::run_missed_tasks(missed_tasks, weight_left) }

	/*
	* This section is to test run_tasks.
	* run_tasks_many_found: tests many existing tasks for linear progression
	* run_tasks_many_missing: tests many non existing tasks for linear progression
	*/
	run_tasks_many_found {
		let v in 0 .. 1;
		let weight_left = 500_000_000_000;
		let mut task_ids = vec![];
		let caller: T::AccountId = account("caller", 0, SEED);
		let time = 10800;
		let recipient: T::AccountId = account("to", 0, SEED);
		let starting_multiplier: u32 = 20;
		let transfer_amount = T::NativeTokenExchange::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		let starting_amount = T::NativeTokenExchange::minimum_balance().saturating_mul(starting_multiplier.into());
		T::NativeTokenExchange::deposit_creating(&caller, starting_amount.clone());

		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), vec![time.into()]).unwrap();
			let task = Task::<T>::create_native_transfer_task(caller.clone(), provided_id, vec![time].try_into().unwrap(), recipient.clone(), transfer_amount.clone());
			<Tasks<T>>::insert(task_id, task);
			task_ids.push(task_id)
		}
	}: { AutomationTime::<T>::run_tasks(task_ids, weight_left) }

	run_tasks_many_missing {
		let v in 0 .. 1;
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 10800;
		let mut task_ids = vec![];
		let weight_left = 500_000_000_000;

		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::generate_task_id(caller.clone(), provided_id);
			task_ids.push(task_id)
		}
	}: { AutomationTime::<T>::run_tasks(task_ids, weight_left) }

	/*
	* This section is to test update_task_queue. This only tests for 1 single missed slot.
	* update_task_queue_overhead: gets overhead of fn without any items
	* append_to_missed_tasks: measures appending to missed tasks
	* update_task_queue_max_current_and_next: measures fn overhead with both current and future tasks
	*/

	update_task_queue_overhead {
		let weight_left = 500_000_000_000;
	}: { AutomationTime::<T>::update_task_queue(weight_left) }

	append_to_missed_tasks {
		let weight_left = 500_000_000_000;
		let v in 0 .. 2;
		let caller: T::AccountId = account("callerName", 0, SEED);
		let last_time_slot: u64 = 3600;
		let time = last_time_slot;
		let time_change: u64 = (v * 3600).into();
		let current_time = last_time_slot + time_change;

		for i in 0..v {
			for j in 0..1 {
				let time = time.saturating_add(3600);
				let provided_id: Vec<u8> = vec![i.saturated_into::<u8>(), j.saturated_into::<u8>()];
				let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), vec![time.into()]).unwrap();
				let task = Task::<T>::create_event_task(caller.clone(), provided_id, vec![time.into()].try_into().unwrap(), vec![4, 5, 6]);
				<Tasks<T>>::insert(task_id, task);
			}
		}
	}: { AutomationTime::<T>::append_to_missed_tasks(current_time, last_time_slot, weight_left) }

	update_scheduled_task_queue {
		let caller: T::AccountId = account("callerName", 0, SEED);
		let last_time_slot: u64 = 7200;
		let current_time = 10800;

		for i in 0..T::MaxTasksPerSlot::get() {
			let provided_id: Vec<u8> = vec![(i/256).try_into().unwrap(), (i%256).try_into().unwrap()];
			let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), vec![current_time.into()]).unwrap();
			let task = Task::<T>::create_event_task(caller.clone(), provided_id, vec![current_time.into()].try_into().unwrap(), vec![4, 5, 6]);
			<Tasks<T>>::insert(task_id, task);
		}
	}: { AutomationTime::<T>::update_scheduled_task_queue(current_time, last_time_slot) }


	shift_missed_tasks {
		let caller: T::AccountId = account("callerName", 0, SEED);
		let last_time_slot: u64 = 7200;
		let new_time_slot: u64 = 14400;
		let diff = 1;

		schedule_notify_tasks::<T>(caller.clone(), vec![new_time_slot], T::MaxTasksPerSlot::get());
	}: { AutomationTime::<T>::shift_missed_tasks(last_time_slot, diff) }
}
