// This file is part of OAK Blockchain.

// Copyright (C) 2021 OAK Network
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
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

use crate::Pallet as AutomationTime;

const MAX_TASKS_PER_SLOT: u8 = 2;

fn schedule_tasks<T: Config>(owner: T::AccountId, time: u64, count: u8) -> T::Hash {
	let mut task_id: T::Hash = T::Hash::default();

	for i in 0..count {
		let provided_id: Vec<u8> = vec![i];
		task_id =
			AutomationTime::<T>::schedule_task(owner.clone(), provided_id.clone(), time).unwrap();
		let task = Task::<T>::create_event_task(owner.clone(), provided_id, time, vec![4, 5, 6]);
		<Tasks<T>>::insert(task_id, task);
	}
	task_id
}

fn set_overflow_tasks<T: Config>(owner: T::AccountId, time: u64, count: u8) -> T::Hash {
	let mut task_id: T::Hash = T::Hash::default();

	for i in 0..count {
		let provided_id: Vec<u8> = vec![i];
		let task_hash_input =
			TaskHashInput::<T>::create_hash_input(owner.clone(), provided_id.clone());
		task_id = T::Hashing::hash_of(&task_hash_input);
		let task =
			Task::<T>::create_event_task(owner.clone(), provided_id.clone(), time, vec![4, 5, 6]);
		<Tasks<T>>::insert(task_id, task);
		let mut task_ids: Vec<T::Hash> = vec![task_id];
		match AutomationTime::<T>::get_overflow_tasks() {
			None => <OverlflowTasks<T>>::put(task_ids),
			Some(mut overflow) => {
				overflow.append(&mut task_ids);
				<OverlflowTasks<T>>::put(overflow);
			},
		}
	}
	task_id
}

benchmarks! {
	// First task for a slot
	schedule_notify_task_new_slot {
		let caller = whitelisted_caller();
	}: schedule_notify_task(RawOrigin::Signed(caller), vec![50], 60, vec![4, 5, 6])

	// Second task for a slot
	schedule_notify_task_existing_slot {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 120;
		let count: u8 = 1;

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, 1);
	}: schedule_notify_task(RawOrigin::Signed(caller), vec![10], time, vec![4, 5])

	cancel_scheduled_task {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, 1);
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	cancel_scheduled_task_full {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;

		// Setup extra tasks
		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, MAX_TASKS_PER_SLOT);
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	cancel_overflow_task {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;

		let task_id: T::Hash = set_overflow_tasks::<T>(caller.clone(), time, MAX_TASKS_PER_SLOT);
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	force_cancel_scheduled_task {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, 1);
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_scheduled_task_full {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, MAX_TASKS_PER_SLOT);
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_overflow_task {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;

		let task_id: T::Hash = set_overflow_tasks::<T>(caller.clone(), time, MAX_TASKS_PER_SLOT);
	}: force_cancel_task(RawOrigin::Root, task_id)
}
