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

use crate::{mock::*, Event};
use frame_support::traits::OnInitialize;
use pallet_parachain_staking::AdditionalIssuance;

const FIRST_VEST_TIME: u64 = 1646028000;
const SECOND_VEST_TIME: u64 = FIRST_VEST_TIME + 3_600;

#[test]
fn genesis_default() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Vesting::total_unvested_allocation(), 0);
	})
}

#[test]
#[should_panic = "Invalid time"]
fn genesis_bad_time() {
	let mut scheduled_vests: Vec<(u64, Vec<(AccountId, Balance)>)> = vec![];
	let mut first_vest: Vec<(AccountId, Balance)> = vec![];
	first_vest.push((ALICE, 100));
	first_vest.push((BOB, 100));
	scheduled_vests.push((FIRST_VEST_TIME + 120, first_vest));

	ExtBuilder::default().schedule(scheduled_vests).build().execute_with(|| {})
}

#[test]
#[should_panic = "Cannot vest less than the existential deposit"]
fn genesis_low_amount() {
	let mut scheduled_vests: Vec<(u64, Vec<(AccountId, Balance)>)> = vec![];
	let mut first_vest: Vec<(AccountId, Balance)> = vec![];
	first_vest.push((ALICE, 1));
	first_vest.push((BOB, 100));
	scheduled_vests.push((FIRST_VEST_TIME, first_vest));

	ExtBuilder::default().schedule(scheduled_vests).build().execute_with(|| {})
}

#[test]
fn genesis() {
	let scheduled_vests = get_schedule();

	ExtBuilder::default().schedule(scheduled_vests).build().execute_with(|| {
		let first_vest = Vesting::get_scheduled_vest(FIRST_VEST_TIME);
		let second_vest = Vesting::get_scheduled_vest(SECOND_VEST_TIME);

		assert_eq!(first_vest.unwrap().len(), 2);
		assert_eq!(second_vest.unwrap().len(), 2);
		assert_eq!(Vesting::total_unvested_allocation(), 600);
	})
}

#[test]
fn on_initialize_with_default() {
	ExtBuilder::default().build().execute_with(|| {
		Timestamp::set_timestamp(FIRST_VEST_TIME * 1_000);

		assert_eq!(None, Vesting::get_scheduled_vest(FIRST_VEST_TIME));
		Vesting::on_initialize(1);
	})
}

#[test]
fn on_initialize_no_time_set() {
	let scheduled_vests = get_schedule();

	ExtBuilder::default().schedule(scheduled_vests).build().execute_with(|| {
		Vesting::on_initialize(1);

		let first_vest = Vesting::get_scheduled_vest(FIRST_VEST_TIME);
		let second_vest = Vesting::get_scheduled_vest(SECOND_VEST_TIME);
		assert_eq!(first_vest.unwrap().len(), 2);
		assert_eq!(second_vest.unwrap().len(), 2);

		assert_eq!(Vesting::total_unvested_allocation(), 600);
	})
}

#[test]
fn on_initialize() {
	let scheduled_vests = get_schedule();

	ExtBuilder::default().schedule(scheduled_vests).build().execute_with(|| {
		Timestamp::set_timestamp(FIRST_VEST_TIME * 1_000);

		let first_vest = Vesting::get_scheduled_vest(FIRST_VEST_TIME);
		let vest_events = vest_to_events(first_vest.unwrap());
		Vesting::on_initialize(1);

		let first_vest = Vesting::get_scheduled_vest(FIRST_VEST_TIME);
		let second_vest = Vesting::get_scheduled_vest(SECOND_VEST_TIME);
		assert_eq!(first_vest, None);
		assert_eq!(second_vest.unwrap().len(), 2);
		assert_eq!(events(), vest_events);
		assert_eq!(Balances::free_balance(ALICE), 100);
		assert_eq!(Balances::free_balance(BOB), 100);
		assert_eq!(Vesting::total_unvested_allocation(), 400);

		let second_vest = Vesting::get_scheduled_vest(SECOND_VEST_TIME);
		let vest_events = vest_to_events(second_vest.unwrap());
		Timestamp::set_timestamp(SECOND_VEST_TIME * 1_000);
		Vesting::on_initialize(2);

		let second_vest = Vesting::get_scheduled_vest(SECOND_VEST_TIME);
		assert_eq!(second_vest, None);
		assert_eq!(events(), vest_events);
		assert_eq!(Balances::free_balance(ALICE), 300);
		assert_eq!(Balances::free_balance(BOB), 300);
		assert_eq!(Vesting::total_unvested_allocation(), 0);

		Timestamp::set_timestamp(SECOND_VEST_TIME * 1_000);
		Vesting::on_initialize(2);
	})
}

#[test]
fn additional_issuance() {
	ExtBuilder::default().schedule(get_schedule()).build().execute_with(|| {
		assert_eq!(Vesting::additional_issuance(), 600);
	})
}

fn get_schedule() -> Vec<(u64, Vec<(AccountId, Balance)>)> {
	let mut scheduled_vests: Vec<(u64, Vec<(AccountId, Balance)>)> = vec![];
	let mut first_vest: Vec<(AccountId, Balance)> = vec![];
	first_vest.push((ALICE, 100));
	first_vest.push((BOB, 100));
	scheduled_vests.push((FIRST_VEST_TIME, first_vest));
	let mut second_vest: Vec<(AccountId, Balance)> = vec![];
	second_vest.push((ALICE, 200));
	second_vest.push((BOB, 200));
	scheduled_vests.push((SECOND_VEST_TIME, second_vest));

	scheduled_vests
}

fn vest_to_events(vests: Vec<(AccountId, Balance)>) -> Vec<Event<Test>> {
	let mut events: Vec<Event<Test>> = vec![];
	for (account, amount) in vests {
		let event = Event::Vested { account, amount };
		events.push(event);
	}
	events
}
