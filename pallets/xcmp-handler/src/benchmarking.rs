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

use super::*;

#[allow(unused)]
use crate::Pallet as XcmpHandler;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, Box};
use frame_system::RawOrigin;

use xcm::latest::prelude::*;

benchmarks! {
	set_transact_info {
		let location = MultiLocation::new(1, X1(Parachain(1000)));
		let transact_info = XcmTransactInfo { flow: XcmFlow::Normal };
	}: set_transact_info(RawOrigin::Root, Box::new(location.into()), transact_info.clone())
	verify {
		assert_eq!(TransactInfo::<T>::get(location).unwrap(), transact_info);
	}

	remove_transact_info {
		let location = MultiLocation::new(1, X1(Parachain(1000)));
		let transact_info = XcmTransactInfo { flow: XcmFlow::Normal };
		TransactInfo::<T>::insert(location, transact_info);

	}: remove_transact_info(RawOrigin::Root, Box::new(location.into()))
	verify {
		if let Some(_) = TransactInfo::<T>::get(location) {
			panic!("There should be no data set")
		};
	}
}

impl_benchmark_test_suite!(XcmpHandler, crate::mock::new_test_ext(None), crate::mock::Test,);
