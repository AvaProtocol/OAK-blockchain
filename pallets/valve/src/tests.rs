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

use crate::{
	mock::{events, Call as OuterCall, ExtBuilder, Origin, Test},
	Call, Error, Event,
};
use frame_support::{assert_noop, assert_ok, dispatch::Dispatchable};

#[test]
fn can_remark_while_valve_open() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = frame_system::Call::remark { remark: vec![] }.into();
		assert_ok!(call.dispatch(Origin::signed(1)));
	})
}

#[test]
fn cannot_remark_while_valve_closed() {
	ExtBuilder::default().with_valve_closed(true).build().execute_with(|| {
		let call: OuterCall = frame_system::Call::remark { remark: vec![] }.into();
		assert_noop!(call.dispatch(Origin::signed(1)), frame_system::Error::<Test>::CallFiltered);
	})
}

#[test]
fn can_close_valve() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::close_valve {}.into();
		assert_ok!(call.dispatch(Origin::root()));
		assert_eq!(events(), vec![Event::ValveClosed,]);
	})
}

#[test]
fn cannot_close_valve_when_already_closed() {
	ExtBuilder::default().with_valve_closed(true).build().execute_with(|| {
		let call: OuterCall = Call::close_valve {}.into();
		assert_noop!(call.dispatch(Origin::root()), Error::<Test>::ValveAlreadyClosed);
	})
}

#[test]
fn can_close_pallet_gatee() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::close_pallet_gate { pallet_name: b"System".to_vec() }.into();

		assert_ok!(call.dispatch(Origin::root()));
		assert_eq!(
			events(),
			vec![Event::PalletGateClosed { pallet_name_bytes: b"System".to_vec() },]
		);

		let call: OuterCall = frame_system::Call::remark { remark: vec![] }.into();
		assert_noop!(call.dispatch(Origin::signed(1)), frame_system::Error::<Test>::CallFiltered);
	})
}

#[test]
fn cannot_close_valve_pallet_gate() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::close_pallet_gate { pallet_name: b"Valve".to_vec() }.into();
		assert_noop!(call.dispatch(Origin::root()), Error::<Test>::CannotCloseGate);
	})
}

#[test]
fn cannot_close_pallet_gate_when_valve_closed() {
	ExtBuilder::default().with_valve_closed(true).build().execute_with(|| {
		let call: OuterCall = Call::close_pallet_gate { pallet_name: b"System".to_vec() }.into();
		assert_noop!(call.dispatch(Origin::root()), Error::<Test>::ValveAlreadyClosed);
	})
}

#[test]
fn can_start_with_pallet_gate_closed() {
	ExtBuilder::default()
		.with_gate_closed(b"System".to_vec())
		.build()
		.execute_with(|| {
			let call: OuterCall = frame_system::Call::remark { remark: vec![] }.into();
			assert_noop!(
				call.dispatch(Origin::signed(1)),
				frame_system::Error::<Test>::CallFiltered
			);
		})
}

#[test]
fn can_open_valve() {
	ExtBuilder::default().with_valve_closed(true).build().execute_with(|| {
		let call: OuterCall = Call::open_valve {}.into();
		assert_ok!(call.dispatch(Origin::root()));

		assert_eq!(events(), vec![Event::ValveOpen,]);
	})
}

#[test]
fn can_open_pallet_gate() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::close_pallet_gate { pallet_name: b"System".to_vec() }.into();

		assert_ok!(call.dispatch(Origin::root()));
		assert_eq!(
			events(),
			vec![Event::PalletGateClosed { pallet_name_bytes: b"System".to_vec() },]
		);

		let call: OuterCall = frame_system::Call::remark { remark: vec![] }.into();
		assert_noop!(call.dispatch(Origin::signed(1)), frame_system::Error::<Test>::CallFiltered);

		let call: OuterCall = Call::open_pallet_gate { pallet_name: b"System".to_vec() }.into();
		assert_ok!(call.dispatch(Origin::root()));
		assert_eq!(
			events(),
			vec![Event::PalletGateOpen { pallet_name_bytes: b"System".to_vec() },]
		);

		let call: OuterCall = frame_system::Call::remark { remark: vec![] }.into();
		assert_ok!(call.dispatch(Origin::signed(1)));
	})
}

#[test]
fn cannot_open_pallet_gate_when_valve_closed() {
	ExtBuilder::default().with_valve_closed(true).build().execute_with(|| {
		let call: OuterCall = Call::close_pallet_gate { pallet_name: b"System".to_vec() }.into();
		assert_noop!(call.dispatch(Origin::root()), Error::<Test>::ValveAlreadyClosed);
	})
}

#[test]
fn opens_all_pallet_gates() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::close_pallet_gate { pallet_name: b"System".to_vec() }.into();
		assert_ok!(call.dispatch(Origin::root()));

		let call: OuterCall = frame_system::Call::remark { remark: vec![] }.into();
		assert_noop!(call.dispatch(Origin::signed(1)), frame_system::Error::<Test>::CallFiltered);

		let call: OuterCall = Call::open_valve {}.into();
		assert_ok!(call.dispatch(Origin::root()));

		let call: OuterCall = frame_system::Call::remark { remark: vec![] }.into();
		assert_ok!(call.dispatch(Origin::signed(1)));
	})
}

#[test]
fn stop_scheduled_tasks() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::stop_scheduled_tasks {}.into();
		assert_ok!(call.dispatch(Origin::root()));

		assert_eq!(events(), vec![Event::ScheduledTasksStopped]);
	})
}

#[test]
fn stop_scheduled_tasks_already_stopped() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::stop_scheduled_tasks {}.into();
		assert_ok!(call.dispatch(Origin::root()));

		let call: OuterCall = Call::stop_scheduled_tasks {}.into();
		assert_noop!(call.dispatch(Origin::root()), Error::<Test>::ScheduledTasksAlreadyStopped);
	})
}

#[test]
fn start_scheduled_tasks() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::stop_scheduled_tasks {}.into();
		assert_ok!(call.dispatch(Origin::root()));

		let call: OuterCall = Call::start_scheduled_tasks {}.into();
		assert_ok!(call.dispatch(Origin::root()));

		assert_eq!(events(), vec![Event::ScheduledTasksStopped, Event::ScheduledTasksResumed]);
	})
}

#[test]
fn start_scheduled_tasks_not_stopped() {
	ExtBuilder::default().build().execute_with(|| {
		let call: OuterCall = Call::start_scheduled_tasks {}.into();
		assert_noop!(call.dispatch(Origin::root()), Error::<Test>::ScheduledTasksAlreadyRunnung);
	})
}
