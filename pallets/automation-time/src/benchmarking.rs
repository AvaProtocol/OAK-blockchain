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

fn schedule_tasks<T: Config>(owner: T::AccountId, time: u64, count: u32) -> T::Hash {
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
		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, T::MaxTasksPerSlot::get() - 1);
	}: schedule_notify_task(RawOrigin::Signed(caller), vec![10], time, vec![4, 5])

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

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, T::MaxTasksPerSlot::get() - 1);
	}: schedule_native_transfer_task(RawOrigin::Signed(caller), vec![10], time, recipient, transfer_amount)

	cancel_scheduled_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, 1);
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	cancel_scheduled_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, T::MaxTasksPerSlot::get());
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	cancel_overflow_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = set_task_queue::<T>(caller.clone(), time, T::MaxTasksPerSlot::get());
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	force_cancel_scheduled_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, 1);
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_scheduled_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = schedule_tasks::<T>(caller.clone(), time, T::MaxTasksPerSlot::get());
	}: force_cancel_task(RawOrigin::Root, task_id)

	force_cancel_overflow_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 180;

		let task_id: T::Hash = set_task_queue::<T>(caller.clone(), time, T::MaxTasksPerSlot::get());
	}: force_cancel_task(RawOrigin::Root, task_id)
}
