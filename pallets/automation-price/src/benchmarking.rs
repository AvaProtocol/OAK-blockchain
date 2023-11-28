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

use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::{AccountIdConversion, Saturating};

use xcm::latest::{prelude::*, MultiLocation};

use crate::{
	pallet::{Task, TaskId},
	Config, Pallet as AutomationPrice,
};

const SEED: u32 = 0;
// existential deposit multiplier
const ED_MULTIPLIER: u32 = 1_000;
// ensure enough funds to execute tasks
const DEPOSIT_MULTIPLIER: u32 = 100_000_000;

const chain: &[u8] = "chain".as_bytes();
const exchange: &[u8] = "exchange".as_bytes();
const asset_tur: &[u8] = "TUR".as_bytes();
const asset_usd: &[u8] = "USD".as_bytes();
const decimal: u8 = 10_u8;

// a helper function to prepare asset when setting up tasks or price because asset needs to be
// defined before updating price
fn setup_asset<T: Config>(authorized_wallets: Vec<T::AccountId>) {
	AutomationPrice::<T>::initialize_asset(
		RawOrigin::Root.into(),
		chain.to_vec(),
		exchange.to_vec(),
		asset_tur.to_vec(),
		asset_usd.to_vec(),
		decimal,
		authorized_wallets,
	);
}

// a helper method to schedule task with a set of default params to support benchmark easier
fn schedule_xcmp_task<T: Config>(
	para_id: u32,
	owner: T::AccountId,
	call: Vec<u8>,
	expired_at: u128,
) {
	AutomationPrice::<T>::schedule_xcmp_task(
		RawOrigin::Signed(owner).into(),
		chain.to_vec(),
		exchange.to_vec(),
		asset_tur.to_vec(),
		asset_usd.to_vec(),
		6000u128,
		"gt".as_bytes().to_vec(),
		vec![2000],
		Box::new(MultiLocation::new(1, X1(Parachain(para_id))).into()),
		Box::new(MultiLocation::default().into()),
		Box::new(AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		}),
		call,
		Weight::from_parts(100_000, 0),
		Weight::from_parts(200_000, 0),
	);
}

// direct_task_schedule push the task directly to the task registry and relevant setup,
// by pass the normal extrinsic execution.
// This funciton should be used to prepare data for benchmark
fn direct_task_schedule<T: Config>(
	creator: T::AccountId,
	task_id: TaskId,
	expired_at: u128,
	trigger_function: Vec<u8>,
	price_target: u128,
	encoded_call: Vec<u8>,
) -> Result<(), Error<T>> {
	let para_id: u32 = 2000;
	let destination = MultiLocation::new(1, X1(Parachain(para_id)));
	let schedule_fee = MultiLocation::default();
	let execution_fee = AssetPayment {
		asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
		amount: 0,
	};
	let encoded_call_weight = Weight::from_parts(100_000, 0);
	let overall_weight = Weight::from_parts(200_000, 0);
	let schedule_as = account("caller", 0, SEED);

	let action = Action::XCMP {
		destination,
		schedule_fee,
		execution_fee,
		encoded_call,
		encoded_call_weight,
		overall_weight,
		schedule_as: Some(schedule_as),
		instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
	};

	let task: Task<T> = Task::<T> {
		owner_id: creator,
		task_id,
		chain: chain.to_vec(),
		exchange: exchange.to_vec(),
		asset_pair: (asset_tur.to_vec(), asset_usd.to_vec()),
		expired_at,
		trigger_function,
		trigger_params: vec![price_target],
		action,
	};

	AutomationPrice::<T>::validate_and_schedule_task(task)
}

benchmarks! {
	initialize_asset_extrinsic {
		let v in 1..5;
		let asset_pair = (asset_tur.to_vec(), asset_usd.to_vec());

		let mut authorized_wallets: Vec<T::AccountId> = vec![];
		for i in 1..=v {
			authorized_wallets.push(account("caller", i, SEED));
		}
	} : {
		AutomationPrice::<T>::initialize_asset(
			RawOrigin::Root.into(),
			chain.to_vec(), exchange.to_vec(),
			asset_tur.to_vec(), asset_usd.to_vec(), decimal, authorized_wallets);
	}

	asset_price_update_extrinsic {
		// Depend on the size of the input, the weight change, ideally scale linearly
		// Therefore we can simulate v from 1..100 and substrate will agg those value
		let v in 1..100;
		let sender : T::AccountId = account("caller", 0, SEED);

		setup_asset::<T>(vec![sender.clone()]);

		let mut chains: Vec<Vec<u8>> = vec![];
		let mut exchanges: Vec<Vec<u8>> = vec![];
		let mut assets1: Vec<Vec<u8>> = vec![];
		let mut assets2: Vec<Vec<u8>> = vec![];
		let mut prices: Vec<u128> = vec![];
		let mut submitted_ats: Vec<u128> = vec![];
		let mut rounds: Vec<u128> = vec![];

		for i in 1..=v {
			chains.push(format!("chain:{:?}", i).as_bytes().to_vec());
			exchanges.push(format!("exchange:{:?}", i).as_bytes().to_vec());
			assets1.push(format!("ASSET1{:?}", i).as_bytes().to_vec());
			assets2.push(format!("ASSET2{:?}", i).as_bytes().to_vec());
			prices.push(i as u128);
			submitted_ats.push(i as u128);
			rounds.push(i as u128);
		}
	} : {
		AutomationPrice::<T>::update_asset_prices(
			RawOrigin::Signed(sender.clone()).into(),
			chains,
			exchanges,
			assets1,
			assets2,
			prices,
			submitted_ats,
			rounds
		);
	}

	schedule_xcmp_task_extrinsic {
		let sender : T::AccountId = account("caller", 0, SEED);
		let para_id: u32 = 1000;
		let call: Vec<u8> = vec![2, 4, 5];
		setup_asset::<T>(vec![sender.clone()]);
		let transfer_amount = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		T::Currency::deposit_creating(
			&sender,
			transfer_amount.saturating_mul(DEPOSIT_MULTIPLIER.into()),
		);

	} : {
		schedule_xcmp_task::<T>(para_id, sender, call, 12345);
	}

	cancel_task_extrinsic {
		let creator : T::AccountId = account("caller", 0, SEED);
		let para_id: u32 = 1000;
		let call: Vec<u8> = vec![2, 4, 5];
		setup_asset::<T>(vec![creator.clone()]);
		let transfer_amount = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		T::Currency::deposit_creating(
			&creator,
			transfer_amount.saturating_mul(DEPOSIT_MULTIPLIER.into()),
		);

		// Schedule 10000 Task, This is just an arbitrary number to simular a big task registry
		// Because of using StoragMap, and avoid dealing with vector
		// our task look up will always be O(1) for time
		let mut task_ids: Vec<TaskId> = vec![];
		for i in 1..100 {
		  // Fund the account so we can schedule task
		  let account_min = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		  T::Currency::deposit_creating(&creator, account_min.saturating_mul(DEPOSIT_MULTIPLIER.into()));
		  direct_task_schedule::<T>(creator.clone(), format!("{:?}", i).as_bytes().to_vec(), i, "gt".as_bytes().to_vec(), i, vec![100, 200, (i % 256) as u8]);
		  task_ids.push(format!("{:?}", i).as_bytes().to_vec());
		}

		let task_id_to_cancel = "1".as_bytes().to_vec();
	} : {
		AutomationPrice::<T>::cancel_task(RawOrigin::Signed(creator).into(), task_id_to_cancel.clone());
	}
	verify {
	}

	run_xcmp_task {
		let creator: T::AccountId = account("caller", 0, SEED);
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
	}: {
		AutomationPrice::<T>::run_xcmp_task(destination, creator, fee, call, Weight::from_parts(100_000, 0), Weight::from_parts(200_000, 0), InstructionSequence::PayThroughSovereignAccount)
	}

	remove_task {
		let creator : T::AccountId = account("caller", 0, SEED);
		let para_id: u32 = 1000;
		let call: Vec<u8> = vec![2, 4, 5];
		setup_asset::<T>(vec![creator.clone()]);
		let transfer_amount = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
		T::Currency::deposit_creating(
			&creator,
			transfer_amount.saturating_mul(DEPOSIT_MULTIPLIER.into()),
		);

		let para_id: u32 = 2000;
		let destination = MultiLocation::new(1, X1(Parachain(para_id)));
		let schedule_fee = MultiLocation::default();
		let execution_fee = AssetPayment {
			asset_location: MultiLocation::new(1, X1(Parachain(para_id))).into(),
			amount: 0,
		};
		let encoded_call_weight = Weight::from_ref_time(100_000);
		let overall_weight = Weight::from_ref_time(200_000);
		let schedule_as: T::AccountId = account("caller", 0, SEED);

		// Schedule 10000 Task, This is just an arbitrary number to simular a big task registry
		// Because of using StoragMap, and avoid dealing with vector
		// our task look up will always be O(1) for time
		let mut task_ids: Vec<TaskId> = vec![];
		let mut tasks: Vec<Task<T>> = vec![];
		for i in 1..100 {
		  let task_id = format!("{:?}", i).as_bytes().to_vec();
		  let expired_at = i;
		  let trigger_function = "gt".as_bytes().to_vec();
		  let price_target: u128 = i;
		  let encoded_call = vec![100, 200, (i % 256) as u8];

		  task_ids.push(format!("{:?}", i).as_bytes().to_vec());
			let action = Action::XCMP {
				destination,
				schedule_fee,
				execution_fee: execution_fee.clone(),
				encoded_call,
				encoded_call_weight,
				overall_weight,
				schedule_as: Some(schedule_as.clone()),
				instruction_sequence: InstructionSequence::PayThroughRemoteDerivativeAccount,
			};

			let task: Task<T> = Task::<T> {
				owner_id: creator.clone(),
				task_id: task_id.clone(),
				chain: chain.to_vec(),
				exchange: exchange.to_vec(),
				asset_pair: (asset_tur.to_vec(), asset_usd.to_vec()),
				expired_at,
				trigger_function,
				trigger_params: vec![price_target],
				action,
			};
			AutomationPrice::<T>::validate_and_schedule_task(task.clone());
			tasks.push(task);
		}

		let task = tasks.pop().unwrap();
	}: {
		// remove a task at the end to simulate the worst case
		AutomationPrice::<T>::remove_task(&task, Some(crate::Event::<T>::TaskSweep {
			owner_id: task.owner_id.clone(),
			task_id: task.task_id.clone(),
			condition: crate::TaskCondition::AlreadyExpired {
				expired_at: task.expired_at,
				now: 100,
			}
		}));
	}


	emit_event {
		let owner_id: T::AccountId = account("call", 1, SEED);
		let schedule_as: T::AccountId = account("schedule_as", 1, SEED);
		let task_id: TaskId = vec![1,2,3];
	} : {
		AutomationPrice::<T>::deposit_event(crate::Event::<T>::TaskScheduled {
				owner_id,
				task_id,
				schedule_as: Some(schedule_as),
			});
	}

	impl_benchmark_test_suite!(
		AutomationPrice,
		crate::mock::new_test_ext(crate::tests::START_BLOCK_TIME),
		crate::mock::Test
	)
}
