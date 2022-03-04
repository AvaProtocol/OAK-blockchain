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

fn schedule_notify_tasks<T: Config>(owner: T::AccountId, time: u64, count: u32) -> T::Hash {
	let time_moment: u32 = time.try_into().unwrap();
	<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
	let mut task_id: T::Hash = T::Hash::default();
	let converted_count: u8 = count.try_into().unwrap();

	for i in 0..converted_count {
		let provided_id: Vec<u8> = vec![i];
		task_id =
			AutomationTime::<T>::schedule_task(owner.clone(), provided_id.clone(), time).unwrap();
		let task = Task::<T>::create_event_task(owner.clone(), provided_id, time, vec![4, 5, 6]);
		<Tasks<T>>::insert(task_id, task);
	}
	task_id
}

fn schedule_transfer_tasks<T: Config>(owner: T::AccountId, time: u64, count: u32, recipient: T::AccountId, amount: BalanceOf<T>) -> T::Hash {
	let time_moment: u32 = time.try_into().unwrap();
	<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
	let mut task_id: T::Hash = T::Hash::default();
	let converted_count: u8 = count.try_into().unwrap();

	for i in 0..converted_count {
		let provided_id: Vec<u8> = vec![i];
		task_id =
			AutomationTime::<T>::schedule_task(owner.clone(), provided_id.clone(), time).unwrap();
		let task = Task::<T>::create_native_transfer_task(owner.clone(), provided_id, time, recipient.clone(), amount.clone());
		<Tasks<T>>::insert(task_id, task);
	}
	task_id
}

fn set_task_queue<T: Config>(owner: T::AccountId, time: u64, count: u32) -> T::Hash {
	let mut task_id: T::Hash = T::Hash::default();
	let converted_count: u8 = count.try_into().unwrap();

	for i in 0..converted_count {
		let provided_id: Vec<u8> = vec![i];
		let task_hash_input =
			TaskHashInput::<T>::create_hash_input(owner.clone(), provided_id.clone());
		task_id = T::Hashing::hash_of(&task_hash_input);
		let task =
			Task::<T>::create_event_task(owner.clone(), provided_id.clone(), time, vec![4, 5, 6]);
		<Tasks<T>>::insert(task_id, task);
		let mut task_ids: Vec<T::Hash> = vec![task_id];
		let mut task_queue = AutomationTime::<T>::get_task_queue();
		task_queue.append(&mut task_ids);
		<TaskQueue<T>>::put(task_queue);
	}
	task_id
}

benchmarks! {
	schedule_notify_task_empty {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 120;
		let time_moment: u32 = time.try_into().unwrap();
		<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
	}: schedule_notify_task(RawOrigin::Signed(caller), vec![10], time, vec![4, 5])

	schedule_notify_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 120;
		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), time, T::MaxTasksPerSlot::get() - 1);
		let provided_id = (T::MaxTasksPerSlot::get()).saturated_into::<u8>();
	}: schedule_notify_task(RawOrigin::Signed(caller), vec![provided_id], time, vec![4, 5])

	schedule_native_transfer_task_empty{
		let caller: T::AccountId = account("caller", 0, SEED);
		let recipient: T::AccountId = account("to", 0, SEED);
		let time: u64 = 120;
		let transfer_amount = T::ExistentialDeposit::get().saturating_mul(ED_MULTIPLIER.into());
		let time_moment: u32 = time.try_into().unwrap();
		<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
	}: schedule_native_transfer_task(RawOrigin::Signed(caller), vec![10], time, recipient, transfer_amount)

	schedule_native_transfer_task_full{
		let caller: T::AccountId = account("caller", 0, SEED);
		let recipient: T::AccountId = account("to", 0, SEED);
		let time: u64 = 120;
		let transfer_amount = T::ExistentialDeposit::get().saturating_mul(ED_MULTIPLIER.into());

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), time, T::MaxTasksPerSlot::get() - 1);
		let provided_id = (T::MaxTasksPerSlot::get()).saturated_into::<u8>();
	}: schedule_native_transfer_task(RawOrigin::Signed(caller), vec![provided_id], time, recipient, transfer_amount)

	cancel_scheduled_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), time, 1);
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	cancel_scheduled_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), time, T::MaxTasksPerSlot::get());
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	cancel_overflow_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = set_task_queue::<T>(caller.clone(), time, T::MaxTasksPerSlot::get());
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	force_cancel_scheduled_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), time, 1);
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_scheduled_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = schedule_notify_tasks::<T>(caller.clone(), time, T::MaxTasksPerSlot::get());
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_overflow_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = set_task_queue::<T>(caller.clone(), time, T::MaxTasksPerSlot::get());
	}: force_cancel_task(RawOrigin::Root, task_id)

	run_notify_task {
		let message = b"hello there".to_vec();
	}: { AutomationTime::<T>::run_notify_task(message) }

	run_native_transfer_task {
		let starting_multiplier: u32 = 20;
		let amount_starting: BalanceOf<T> = T::ExistentialDeposit::get().saturating_mul(starting_multiplier.into());
		let caller: T::AccountId = account("caller", 0, SEED);
		T::Currency::deposit_creating(&caller, amount_starting.clone());
		let recipient: T::AccountId = account("recipient", 0, SEED);
		let time: u64 = 180;

		let amount_transferred: BalanceOf<T> = T::ExistentialDeposit::get().saturating_mul(ED_MULTIPLIER.into());
		let task_id: T::Hash = schedule_transfer_tasks::<T>(caller.clone(), time, 1, recipient.clone(), amount_transferred.clone());
	}: { AutomationTime::<T>::run_native_transfer_task(caller, recipient, amount_transferred, task_id) }

	/*
	* This section is to test run_missed_tasks.
	* run_missed_tasks_none: measuring base overhead of run_missed_tasks
	* run_missed_tasks_many_found: measuring many tasks for linear progression
	* run_missed_tasks_many_missing: measuring number of tasks if not found in queue, to find slope of growth
	*/
	run_missed_tasks_none {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;
		let weight_left = 500_000_000_000;
		let task_ids = vec![];
	}: { AutomationTime::<T>::run_missed_tasks(task_ids, weight_left) }

	run_missed_tasks_many_found {
		let v in 0 .. 1;

		let weight_left = 50_000_000_000;
		let mut task_ids = vec![];
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u32 = 180;
		
		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), time.into()).unwrap();
			let task = Task::<T>::create_event_task(caller.clone(), provided_id, time.into(), vec![4, 5, 6]);
			<Tasks<T>>::insert(task_id, task);
			task_ids.push(task_id)
		}
	}: { AutomationTime::<T>::run_missed_tasks(task_ids, weight_left) }

	run_missed_tasks_many_missing {
		let v in 0 .. 1;

		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;
		let mut task_ids = vec![];
		let weight_left = 500_000_000_000;

		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::generate_task_id(caller.clone(), provided_id);
			task_ids.push(task_id)
		}
	}: { AutomationTime::<T>::run_missed_tasks(task_ids, weight_left) }

	run_missed_tasks_split_off {
		let v in 2 .. 10;

		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;
		let mut task_ids = vec![];
		let weight_left = 20_000;

		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::generate_task_id(caller.clone(), provided_id);
			task_ids.push(task_id)
		}
	}: { AutomationTime::<T>::run_missed_tasks(task_ids, weight_left) }

	/*
	* This section is to test run_tasks.
	* run_tasks_none: tests base framework of run_tasks
	* run_tasks_many_found: tests many tasks for linear progression
	* run_tasks_many_missing: tests number of tasks if not found in queue, to find slope of growth
	*/
	run_tasks_none {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;
		let weight_left = 500_000_000_000;
		let task_ids = vec![];
	}: { AutomationTime::<T>::run_tasks(task_ids, weight_left) }

	run_tasks_many_found {
		let v in 0 .. 1;
		let weight_left = 500_000_000_000;
		let mut task_ids = vec![];
		let caller: T::AccountId = account("caller", 0, SEED);
		let time = 180;
		let recipient: T::AccountId = account("to", 0, SEED);
		let transfer_amount = T::ExistentialDeposit::get().saturating_mul(ED_MULTIPLIER.into());
		T::Currency::deposit_creating(&caller, transfer_amount.clone());
		
		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), time.into()).unwrap();
			let task = Task::<T>::create_native_transfer_task(caller.clone(), provided_id, time, recipient.clone(), transfer_amount.clone());
			<Tasks<T>>::insert(task_id, task);
			task_ids.push(task_id)
		}
	}: { AutomationTime::<T>::run_tasks(task_ids, weight_left) }

	run_tasks_many_missing {
		let v in 0 .. 1;
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;
		let mut task_ids = vec![];
		let weight_left = 500_000_000_000;

		for i in 0..v {
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::generate_task_id(caller.clone(), provided_id);
			task_ids.push(task_id)
		}
	}: { AutomationTime::<T>::run_tasks(task_ids, weight_left) }

	/*
	* This section is to test update_task_queue.
	* update_task_queue_max_current: 100 current time expired, 0 next time slot
	* update_task_queue_min_current: 1 current time expired, 0 next time slot
	* update_task_queue_max_next: 0 current time expired, 100 next time slot
	* update_task_queue_min_next: 0 current time expired, 1 next time slot 
	*/

	update_task_queue_overhead {
	}: { AutomationTime::<T>::update_task_queue() }

	update_task_queue_max_current {
		let caller: T::AccountId = account("callerName", 0, SEED);
		let current_time: u64 = 180;
		let next_time: u64 = 240;
		
		schedule_notify_tasks::<T>(caller.clone(), current_time, T::MaxTasksPerSlot::get());
	}: { AutomationTime::<T>::update_task_queue() }

	update_task_queue_min_current {
		let caller: T::AccountId = account("caller", 0, SEED);
		let current_time: u64 = 180;
		let next_time: u64 = 240;

		schedule_notify_tasks::<T>(caller.clone(), current_time, 1);
	}: { AutomationTime::<T>::update_task_queue() }

	update_task_queue_max_next {
		let caller: T::AccountId = account("callerName", 0, SEED);
		let current_time: u64 = 180;
		let next_time: u64 = 240;
		
		schedule_notify_tasks::<T>(caller.clone(), current_time, T::MaxTasksPerSlot::get());
		let time_moment: u32 = current_time.try_into().unwrap();
		<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
	}: { AutomationTime::<T>::update_task_queue() }

	update_task_queue_min_next {
		let caller: T::AccountId = account("caller", 0, SEED);
		let current_time: u64 = 180;
		let next_time: u64 = 240;

		schedule_notify_tasks::<T>(caller.clone(), current_time, 1);
		let time_moment: u32 = current_time.try_into().unwrap();
		<pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into());
	}: { AutomationTime::<T>::update_task_queue() }

	/*
	* This section is to measure trigger_tasks overhead.
	* trigger_tasks_overhead: min trigger, no tasks anywhere
	*/

	trigger_tasks_overhead {
		let weight_left = 500_000_000_000;
	}: { AutomationTime::<T>::trigger_tasks(weight_left) }

	test_missing_tasks_remove_events {
		let v in 0 .. 100;
		let mut task_ids = vec![];
		let caller: T::AccountId = account("caller", 0, SEED);
		
		for i in 0..v {
			let time = 180 + i;
			let provided_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), time.into()).unwrap();
			let task = Task::<T>::create_event_task(caller.clone(), provided_id, time.into(), vec![4, 5, 6]);
			<Tasks<T>>::insert(task_id, task);
			task_ids.push(task_id)
		}
	}: { AutomationTime::<T>::test_missing_tasks_remove_events(task_ids) }

	/*
	* Utils: read and write
	* read: read action
	* write: write action
	*/
	write {
		let time: u64 = 120;
		let time_moment: u32 = time.try_into().unwrap();
	}: { <pallet_timestamp::Pallet<T>>::set_timestamp(time_moment.into()) }
	read {}: { <pallet_timestamp::Pallet<T>>::get().saturated_into::<u64>() }
}
