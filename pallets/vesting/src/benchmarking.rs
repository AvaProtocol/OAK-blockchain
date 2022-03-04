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
use frame_support::traits::Currency;
use pallet_timestamp;
use sp_runtime::traits::{Saturating, SaturatedConversion};
use sp_std::{vec, vec::Vec};

use crate::Pallet as Vesting;

const FIRST_VEST_TIME: u64 = 3600;
const ED_MULTIPLIER: u32 = 10;

benchmarks! {
    vest {
        let v in 0 .. 20;

        if v > 0 {
            let mut scheduled_vests: Vec<(AccountOf<T>, BalanceOf<T>)> = vec![];
            let amount = T::Currency::minimum_balance().saturating_mul(ED_MULTIPLIER.into());
            for i in 0 .. v {
                let vesting_account = account("person", 0, i);
                scheduled_vests.push((vesting_account, amount.clone()));
            } 
            VestingSchedule::<T>::insert(FIRST_VEST_TIME, scheduled_vests);
        }

        let pallet_time: u32 = (FIRST_VEST_TIME * 1_000).saturated_into::<u32>();
        <pallet_timestamp::Pallet<T>>::set_timestamp(pallet_time.into());
    }: { Vesting::<T>::vest() }
} 