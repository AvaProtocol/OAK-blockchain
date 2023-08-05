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

//mod mock;
use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use pallet_timestamp::Pallet as Timestamp;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::{AccountIdConversion, Saturating};
use sp_std::cmp;
use xcm::latest::prelude::*;

use crate::{Config, MissedTaskV2Of, Pallet as AutomationTime, TaskOf};

const SEED: u32 = 0;
// existential deposit multiplier
const ED_MULTIPLIER: u32 = 1_000;
// ensure enough funds to execute tasks
const DEPOSIT_MULTIPLIER: u32 = 100_000_000;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn schedule_notify_tasks<T: Config>(owner: T::AccountId, times: Vec<u64>, count: u32) -> Vec<u8> {
	let transfer_amount = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
	T::Currency::deposit_creating(
		&owner,
		transfer_amount.clone().saturating_mul(DEPOSIT_MULTIPLIER.into()),
	);
	let time_moment: u32 = times[0].saturated_into();
	Timestamp::<T>::set_timestamp(time_moment.into());
	let mut task_id = vec![0u8];

	for _ in 0..count {
		task_id = increment_task_id(task_id);
		let task = TaskOf::<T>::create_event_task::<T>(
			owner.clone(),
			task_id.clone(),
			times.clone(),
			vec![4, 5, 6],
			vec![],
		)
		.unwrap();
		let task_id = AutomationTime::<T>::schedule_task(&task).unwrap();
		<AccountTasks<T>>::insert(owner.clone(), task_id, task);
	}

	task_id
}

fn schedule_xcmp_tasks<T: Config>(owner: T::AccountId, times: Vec<u64>, count: u32) -> Vec<u8> {
	let transfer_amount = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
	T::Currency::deposit_creating(
		&owner,
		transfer_amount.clone().saturating_mul(DEPOSIT_MULTIPLIER.into()),
	);
	let para_id: u32 = 2001;
	let time_moment: u32 = times[0].saturated_into();
	Timestamp::<T>::set_timestamp(time_moment.into());
	let mut task_id = vec![0u8];

	for _ in 0..count {
		task_id = increment_task_id(task_id);
		let task = TaskOf::<T>::create_xcmp_task::<T>(
			owner.clone(),
			task_id.clone(),
			times.clone(),
			MultiLocation::new(1, X1(Parachain(para_id))),
			MultiLocation::default(),
			AssetPayment {
				asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
				amount: 0,
			},
			vec![4, 5, 6],
			Weight::from_ref_time(5_000),
			Weight::from_ref_time(10_000),
			InstructionSequence::PayThroughSovereignAccount,
			vec![],
		)
		.unwrap();
		let task_id = AutomationTime::<T>::schedule_task(&task).unwrap();
		<AccountTasks<T>>::insert(owner.clone(), task_id, task);
	}

	task_id
}

fn schedule_auto_compound_delegated_stake_tasks<T: Config>(
	owner: T::AccountId,
	time: u64,
	count: u32,
) -> Vec<(TaskIdV2, TaskOf<T>)> {
	let time_moment: u32 = time.saturated_into();
	Timestamp::<T>::set_timestamp(time_moment.into());

	let mut tasks = Vec::with_capacity(count.try_into().unwrap());
	let mut task_id = vec![0u8];
	for i in 0..count {
		let collator: T::AccountId = account("collator", 0, i);
		let frequency = 3600;
		task_id = increment_task_id(task_id);
		let account_minimum = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		let task = TaskOf::<T>::create_auto_compound_delegated_stake_task::<T>(
			owner.clone(),
			task_id.clone(),
			time,
			frequency,
			collator,
			account_minimum,
			vec![],
		)
		.unwrap();
		AutomationTime::<T>::schedule_task(&task).unwrap();
		<AccountTasks<T>>::insert(owner.clone(), task_id.clone(), task);
		tasks.push((
			task_id.clone(),
			Pallet::<T>::get_account_task(owner.clone(), task_id.clone()).unwrap(),
		));
	}
	tasks
}

fn increment_task_id(mut task_id: Vec<u8>) -> Vec<u8> {
	let last = task_id.last_mut().unwrap();
	if let Some(next) = last.checked_add(1) {
		*last = next;
	} else {
		task_id.push(0u8);
	}
	task_id
}

benchmarks! {
	schedule_xcmp_task_full {
		let v in 1..T::MaxExecutionTimes::get();

		let mut max_tasks_per_slot: u32 = (
			T::MaxWeightPerSlot::get() / <T as Config>::WeightInfo::run_xcmp_task().ref_time() as u128
		).try_into().unwrap();
		max_tasks_per_slot = cmp::min(max_tasks_per_slot, T::MaxTasksPerSlot::get());

		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 7200;
		let currency_id: T::CurrencyId = 1u32.into();
		let para_id: u32 = 2110;
		let call = vec![4,5,6];

		let mut times: Vec<u64> = vec![];
		for i in 1..=v {
			let hour: u64 = (3600 * i).try_into().unwrap();
			times.push(hour);
		}
		let schedule = ScheduleParam::Fixed { execution_times: times.clone() };

		let destination = MultiLocation::new(1, X1(Parachain(para_id)));

		let schedule_fee = T::CurrencyIdConvert::convert(currency_id).expect("IncoveribleCurrencyId");

		let fee = AssetPayment { asset_location: MultiLocation::new(0, Here).into(), amount: 100u128 };

		let mut task_id = schedule_xcmp_tasks::<T>(caller.clone(), times, max_tasks_per_slot - 1);
		let foreign_currency_amount = T::MultiCurrency::minimum_balance(currency_id.into())
			.saturating_add(1u32.into())
			.saturating_mul(ED_MULTIPLIER.into())
			.saturating_mul(DEPOSIT_MULTIPLIER.into());
		let _ = T::MultiCurrency::deposit(currency_id.into(), &caller, foreign_currency_amount);
	}: schedule_xcmp_task(RawOrigin::Signed(caller), schedule, Box::new(destination.into()), Box::new(schedule_fee.into()), Box::new(fee), call, Weight::from_ref_time(1_000), Weight::from_ref_time(2_000))

	schedule_auto_compound_delegated_stake_task_full {
		let task_weight = <T as Config>::WeightInfo::run_auto_compound_delegated_stake_task().ref_time();
		let max_tasks_per_slot_by_weight: u32 = (T::MaxWeightPerSlot::get() / task_weight as u128).try_into().unwrap();
		let max_tasks_per_slot = max_tasks_per_slot_by_weight.min(T::MaxTasksPerSlot::get());

		let delegator: T::AccountId = account("delegator", 0, SEED);
		let collator: T::AccountId = account("collator", 0, max_tasks_per_slot);
		let account_minimum = T::Currency::minimum_balance();
		let starting_balance = account_minimum.saturating_mul(ED_MULTIPLIER.into())
			.saturating_add(task_weight.saturating_mul(T::ExecutionWeightFee::get().saturated_into()).saturated_into());
		let time: u64 = 3600;

		T::Currency::deposit_creating(&delegator, starting_balance.saturated_into());
		schedule_auto_compound_delegated_stake_tasks::<T>(delegator.clone(), time.clone(), max_tasks_per_slot - 1);
	}: schedule_auto_compound_delegated_stake_task(RawOrigin::Signed(delegator), time, 3600 , collator, account_minimum)

	schedule_dynamic_dispatch_task {
		let v in 1 .. T::MaxExecutionTimes::get();

		Timestamp::<T>::set_timestamp(1u32.into()); // Set to non-zero default for testing

		let times = (1..=v).map(|i| {
			3600 * i as UnixTime
		}).collect();
		let schedule = ScheduleParam::Fixed { execution_times: times };

		let caller: T::AccountId = account("caller", 0, SEED);
		let call: <T as Config>::Call = frame_system::Call::remark { remark: vec![] }.into();

		let account_min = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		T::Currency::deposit_creating(&caller, account_min.saturating_mul(DEPOSIT_MULTIPLIER.into()));

		let initial_event_count = frame_system::Pallet::<T>::event_count() as u8;
	}: schedule_dynamic_dispatch_task(RawOrigin::Signed(caller.clone()), schedule, Box::new(call))
	verify {
		{
			//panic!("events count: {:?} final {:?} data:{:?}", initial_event_count, frame_system::Pallet::<T>::event_count(), frame_system::Pallet::<T>::events());
			// the task id schedule on first block, first extrinsics, but depend on which path in unitest or in benchmark the initial_event_count might be different therefore we get it from the initial state
			assert_last_event::<T>(Event::TaskScheduled { who: caller, schedule_as: None, task_id: format!("1-0-{:?}", initial_event_count).as_bytes().to_vec(),   }.into())
		}
	}

	schedule_dynamic_dispatch_task_full {
		let v in 1 .. T::MaxExecutionTimes::get();

		Timestamp::<T>::set_timestamp(1u32.into()); // Set to non-zero default for testing

		let times: Vec<UnixTime> = (1..=v).map(|i| {
			3600 * i as UnixTime
		}).collect();
		let schedule = ScheduleParam::Fixed { execution_times: times.clone() };

		let caller: T::AccountId = account("caller", 0, SEED);
		let call: <T as Config>::Call = frame_system::Call::remark { remark: vec![] }.into();

		let account_min = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		T::Currency::deposit_creating(&caller, account_min.saturating_mul(DEPOSIT_MULTIPLIER.into()));

		// Almost fill up all time slots
		schedule_notify_tasks::<T>(caller.clone(), times, T::MaxTasksPerSlot::get() - 1);
		let initial_event_count = frame_system::Pallet::<T>::event_count() as u8;
	}: schedule_dynamic_dispatch_task(RawOrigin::Signed(caller.clone()), schedule, Box::new(call))
	verify {
		// the task id schedule on first block, first extrinsics, but depend on which path in unitest or in benchmark the initial_event_count might be different therefore we get it from the initial state
		assert_last_event::<T>(Event::TaskScheduled { who: caller, schedule_as: None, task_id: format!("1-0-{:?}", initial_event_count).as_bytes().to_vec(),   }.into())
	}

	cancel_scheduled_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let mut times: Vec<u64> = vec![];

		for i in 0..T::MaxExecutionTimes::get() {
			let hour: u64 = (3600 * (i + 1)).try_into().unwrap();
			times.push(hour);
		}

		let task_id = schedule_notify_tasks::<T>(caller.clone(), times, T::MaxTasksPerSlot::get());
	}: cancel_task(RawOrigin::Signed(caller), task_id)

	force_cancel_scheduled_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 10800;

		let task_id = schedule_notify_tasks::<T>(caller.clone(), vec![time], 1);
	}: force_cancel_task(RawOrigin::Root, caller, task_id)

	force_cancel_scheduled_task_full {
		let caller: T::AccountId = account("caller", 0, SEED);
		let mut times: Vec<u64> = vec![];

		for i in 0..T::MaxExecutionTimes::get() {
			let hour: u64 = (3600 * (i + 1)).try_into().unwrap();
			times.push(hour);
		}

		let task_id = schedule_notify_tasks::<T>(caller.clone(), times, T::MaxTasksPerSlot::get());
	}: force_cancel_task(RawOrigin::Root, caller, task_id)

	run_notify_task {
		let message = b"hello there".to_vec();
	}: { AutomationTime::<T>::run_notify_task(message.clone()) }
	verify {
		assert_last_event::<T>(Event::Notify{ message }.into())
	}

	run_xcmp_task {
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 10800;
		let para_id: u32 = 2001;
		let call = vec![4,5,6];

		let local_para_id: u32 = 2114;
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let local_sovereign_account: T::AccountId = Sibling::from(local_para_id).into_account_truncating();
		T::Currency::deposit_creating(
			&local_sovereign_account,
			T::Currency::minimum_balance().saturating_mul(DEPOSIT_MULTIPLIER.into()),
		);

		let fee = AssetPayment { asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(), amount: 1000u128 };

		let task_id = schedule_xcmp_tasks::<T>(caller.clone(), vec![time], 1);
	}: { AutomationTime::<T>::run_xcmp_task(destination, caller, fee, call, Weight::from_ref_time(100_000), Weight::from_ref_time(200_000), task_id.clone(), InstructionSequence::PayThroughSovereignAccount) }

	run_auto_compound_delegated_stake_task {
		let delegator: T::AccountId = account("delegator", 0, SEED);
		let collator: T::AccountId = account("collator", 0, SEED);
		let account_minimum = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());

		T::Currency::deposit_creating(&delegator, (10_000_000_000_000_000 as u128).saturated_into());
		T::Currency::deposit_creating(&collator, (100_000_000_000_000_000 as u128).saturated_into());
		T::DelegatorActions::setup_delegator(&collator, &delegator)?;

		let (task_id, task) = schedule_auto_compound_delegated_stake_tasks::<T>(delegator.clone(), 3600, 1).pop().unwrap();
	}: { AutomationTime::<T>::run_auto_compound_delegated_stake_task(delegator, collator, account_minimum, &task) }

	run_dynamic_dispatch_action {
		let caller: T::AccountId = account("caller", 0, SEED);
		let task_id = vec![49, 45, 48, 45, 52];
		let call: <T as Config>::Call = frame_system::Call::remark { remark: vec![] }.into();
		let encoded_call = call.encode();
	}: {
		let (_, error) = AutomationTime::<T>::run_dynamic_dispatch_action(caller.clone(), encoded_call);
		assert_eq!(error, None);
	}

	run_dynamic_dispatch_action_fail_decode {
		let caller: T::AccountId = account("caller", 0, SEED);
		let task_id = vec![49, 45, 48, 45, 52];

		let bad_encoded_call: Vec<u8> = vec![1];
	}: {
		let (_, error) = AutomationTime::<T>::run_dynamic_dispatch_action(caller.clone(), bad_encoded_call);
		assert_eq!(error, Some(DispatchError::from(Error::<T>::CallCannotBeDecoded)));
	}

	/*
	* This section is to test run_missed_tasks.
	* run_missed_tasks_many_found: measuring many existing tasks for linear progression
	* run_missed_tasks_many_missing: measuring many non existing tasks for linear progression
	*/
	run_missed_tasks_many_found {
		let v in 0 .. 1;

		Timestamp::<T>::set_timestamp(1u32.into()); // Set to non-zero default for testing

		let weight_left = Weight::from_ref_time(50_000_000_000);
		let mut missed_tasks = vec![];
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u32 = 10800;

		for i in 0..v {
			let task_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task = TaskOf::<T>::create_event_task::<T>(caller.clone(), task_id.clone(), vec![time.into()], vec![4, 5, 6], vec![]).unwrap();
			AutomationTime::<T>::schedule_task(&task).unwrap();
			let missed_task = MissedTaskV2Of::<T>::new(caller.clone(), task_id.clone(), time.into());
			<AccountTasks<T>>::insert(caller.clone(), task_id.clone(), task);
			missed_tasks.push(missed_task)
		}
	}: { AutomationTime::<T>::run_missed_tasks(missed_tasks, weight_left) }

	run_missed_tasks_many_missing {
		let v in 0 .. 1;

		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 10800;
		let mut missed_tasks = vec![];
		let weight_left = Weight::from_ref_time(500_000_000_000);

		for i in 0..v {
			let task_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let missed_task = MissedTaskV2Of::<T>::new(caller.clone(), task_id.clone(), time.into());
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

		Timestamp::<T>::set_timestamp(1u32.into()); // Set to non-zero default for testing

		let weight_left = Weight::from_ref_time(500_000_000_000);
		let mut task_ids = vec![];
		let caller: T::AccountId = account("caller", 0, SEED);
		let execution_time = 10800;

		for i in 0..v {
			let task_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			let task = TaskOf::<T>::create_event_task::<T>(caller.clone(), task_id.clone(), vec![execution_time], vec![65, 65.saturating_add(i as u8)], vec![]).unwrap();
			let task_id = AutomationTime::<T>::schedule_task(&task).unwrap();
			<AccountTasks<T>>::insert(caller.clone(), task_id.clone(), task);
			task_ids.push((caller.clone(), task_id.clone()))
		}
	}: { AutomationTime::<T>::run_tasks(task_ids, weight_left) }

	run_tasks_many_missing {
		let v in 0 .. 1;
		let caller: T::AccountId = account("caller", 0, SEED);
		let time: u64 = 10800;
		let mut task_ids = vec![];
		let weight_left = Weight::from_ref_time(500_000_000_000);

		for i in 0..v {
			let task_id: Vec<u8> = vec![i.saturated_into::<u8>()];
			task_ids.push((caller.clone(), task_id))
		}
	}: { AutomationTime::<T>::run_tasks(task_ids, weight_left) }

	/*
	* This section is to test update_task_queue. This only tests for 1 single missed slot.
	* update_task_queue_overhead: gets overhead of fn without any items
	* append_to_missed_tasks: measures appending to missed tasks
	* update_task_queue_max_current_and_next: measures fn overhead with both current and future tasks
	*/

	update_task_queue_overhead {
		let weight_left = Weight::from_ref_time(500_000_000_000);
	}: { AutomationTime::<T>::update_task_queue(weight_left) }

	append_to_missed_tasks {
		let v in 0 .. 2;

		Timestamp::<T>::set_timestamp(1u32.into()); // Set to non-zero default for testing

		let weight_left = Weight::from_ref_time(500_000_000_000);
		let caller: T::AccountId = account("callerName", 0, SEED);
		let last_time_slot: u64 = 3600;
		let time = last_time_slot;
		let time_change: u64 = (v * 3600).into();
		let current_time = last_time_slot + time_change;

		for i in 0..v {
			for j in 0..1 {
				let time = time.saturating_add(3600);
				let task_id: Vec<u8> = vec![i.saturated_into::<u8>(), j.saturated_into::<u8>()];
				let task = TaskOf::<T>::create_event_task::<T>(caller.clone(), task_id.clone(), vec![time.into()], vec![4, 5, 6], vec![]).unwrap();
				AutomationTime::<T>::schedule_task(&task).unwrap();
				<AccountTasks<T>>::insert(caller.clone(), task_id, task);
			}
		}
	}: { AutomationTime::<T>::append_to_missed_tasks(current_time, last_time_slot, weight_left) }

	update_scheduled_task_queue {
		let caller: T::AccountId = account("callerName", 0, SEED);
		let last_time_slot: u64 = 7200;
		let current_time = 10800;
		let mut task_id = vec![0u8];
		Timestamp::<T>::set_timestamp(current_time.saturated_into::<u32>().into());

		for i in 0..T::MaxTasksPerSlot::get() {
			task_id = increment_task_id(task_id);
			let task = TaskOf::<T>::create_event_task::<T>(caller.clone(), task_id.clone(), vec![current_time.into()], vec![4, 5, 6], vec![]).unwrap();
			AutomationTime::<T>::schedule_task(&task).unwrap();
			<AccountTasks<T>>::insert(caller.clone(), task_id.clone(), task);
		}
	}: { AutomationTime::<T>::update_scheduled_task_queue(current_time, last_time_slot) }


	shift_missed_tasks {
		let caller: T::AccountId = account("callerName", 0, SEED);
		let last_time_slot: u64 = 7200;
		let new_time_slot: u64 = 14400;
		let diff = 1;

		schedule_notify_tasks::<T>(caller.clone(), vec![new_time_slot], T::MaxTasksPerSlot::get());
	}: { AutomationTime::<T>::shift_missed_tasks(last_time_slot, diff) }

	impl_benchmark_test_suite!(
		AutomationTime,
		crate::mock::new_test_ext(crate::tests::START_BLOCK_TIME),
		crate::mock::Test
	);
}
