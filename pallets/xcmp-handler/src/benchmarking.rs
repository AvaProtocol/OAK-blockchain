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

use xcm::{latest::prelude::*, VersionedMultiLocation};

benchmarks! {
	set_asset_config {
		let location = MultiLocation::new(1, X1(Parachain(1000)));
		let versioned_location: VersionedMultiLocation = location.clone().into();
		let xcm_data = XcmAssetConfig { fee_per_second: 100, instruction_weight: 1_000, flow: XcmFlow::Normal };
	}: set_asset_config(RawOrigin::Root, Box::new(versioned_location), xcm_data.clone())
	verify {
		assert_eq!(DestinationAssetConfig::<T>::get(location).unwrap(), xcm_data);
	}

	remove_asset_config {
		let location = MultiLocation::new(1, X1(Parachain(1000)));
		let versioned_location: VersionedMultiLocation = location.clone().into();
		let xcm_data =
			XcmAssetConfig { fee_per_second: 100, instruction_weight: 1_000, flow: XcmFlow::Normal };
		DestinationAssetConfig::<T>::insert(location.clone(), xcm_data);

	}: remove_asset_config(RawOrigin::Root, Box::new(versioned_location))
	verify {
		if let Some(_) = DestinationAssetConfig::<T>::get(location) {
			panic!("There should be no data set")
		};
	}
}

impl_benchmark_test_suite!(XcmpHandler, crate::mock::new_test_ext(None), crate::mock::Test,);
