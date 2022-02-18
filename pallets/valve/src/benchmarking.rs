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
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;

benchmarks! {
    close_valve {

    }: _(RawOrigin::Root)

    open_valve {
        ClosedPallets::<T>::insert(b"System".to_vec(), ());
        ClosedPallets::<T>::insert(b"Balances".to_vec(), ());
    }: _(RawOrigin::Root)

    close_pallet_gate {

    }: _(RawOrigin::Root, b"System".to_vec())

    open_pallet_gate {
        let pallet_name = b"System".to_vec();
        ClosedPallets::<T>::insert(pallet_name.clone(), ());
    }: _(RawOrigin::Root, pallet_name)

    stop_scheduled_tasks {

    }: _(RawOrigin::Root)

    start_scheduled_tasks {
        <pallet_automation_time::Shutdown<T>>::put(true);
    }: _(RawOrigin::Root)
}
