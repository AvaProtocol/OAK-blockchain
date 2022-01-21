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
// use crate::{ AutomationTime, Task, Tasks};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

use crate::Pallet as AutomationTime;

benchmarks! {
	// First task for a slot
	schedule_notify_task_new_slot {
		let caller = whitelisted_caller();
	}: schedule_notify_task(RawOrigin::Signed(caller), vec![50], 60, vec![4, 5, 6])

	// Second task for a slot
	schedule_notify_task_existing_slot {
		let caller: T::AccountId = whitelisted_caller();
		let provided_id: Vec<u8> = vec![40];
		let time: u64 = 120;

		let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), time).unwrap();
		let task = Task::<T>::create_event_task(caller.clone(), provided_id.clone(), time, vec![4, 5, 6]);
		<Tasks<T>>::insert(task_id, task);
	}: schedule_notify_task(RawOrigin::Signed(caller), vec![50], time, vec![4, 5])

	cancel_scheduled_task {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;

		let provided_id: Vec<u8> = vec![40];
		let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), time).unwrap();
		let task = Task::<T>::create_event_task(caller.clone(), provided_id.clone(), time, vec![4, 5, 6]);
		<Tasks<T>>::insert(task_id, task);
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	cancel_scheduled_task_full {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;
		let mut task_id: T::Hash = T::Hash::default();

		// Setup extra tasks
		for i in 0 .. 2 {
			let provided_id: Vec<u8> = vec![i];
			task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), time).unwrap();
			let task = Task::<T>::create_event_task(caller.clone(), provided_id.clone(), time, vec![4, 5, 6]);
			<Tasks<T>>::insert(task_id, task);
		}
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	cancel_overflow_task {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;
		let mut task_id: T::Hash = T::Hash::default();

		// Setup extra tasks
		for i in 0 .. 2 {
			let provided_id: Vec<u8> = vec![i];
			let task_hash_input = TaskHashInput::<T>::create_hash_input(caller.clone(), provided_id.clone());
			task_id = T::Hashing::hash_of(&task_hash_input);
			let task = Task::<T>::create_event_task(caller.clone(), provided_id.clone(), time, vec![4, 5, 6]);
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
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	force_cancel_scheduled_task {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;

		let provided_id: Vec<u8> = vec![40];
		let task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), time).unwrap();
		let task = Task::<T>::create_event_task(caller.clone(), provided_id.clone(), time, vec![4, 5, 6]);
		<Tasks<T>>::insert(task_id, task);
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_scheduled_task_full {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;
		let mut task_id: T::Hash = T::Hash::default();

		// Setup extra tasks
		for i in 0 .. 2 {
			let provided_id: Vec<u8> = vec![i];
			task_id = AutomationTime::<T>::schedule_task(caller.clone(), provided_id.clone(), time).unwrap();
			let task = Task::<T>::create_event_task(caller.clone(), provided_id.clone(), time, vec![4, 5, 6]);
			<Tasks<T>>::insert(task_id, task);
		}
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_overflow_task {
		let caller: T::AccountId = whitelisted_caller();
		let time: u64 = 180;
		let mut task_id: T::Hash = T::Hash::default();

		// Setup extra tasks
		for i in 0 .. 2 {
			let provided_id: Vec<u8> = vec![i];
			let task_hash_input = TaskHashInput::<T>::create_hash_input(caller.clone(), provided_id.clone());
			task_id = T::Hashing::hash_of(&task_hash_input);
			let task = Task::<T>::create_event_task(caller.clone(), provided_id.clone(), time, vec![4, 5, 6]);
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
	}: force_cancel_task(RawOrigin::Root, task_id)
}
