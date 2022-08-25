// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
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

//! Autogenerated weights for pallet_xcmp_handler
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-08-25, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `actions-runner-1`, CPU: `Intel(R) Xeon(R) E-2388G CPU @ 3.20GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("turing-dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/oak-collator
// benchmark
// pallet
// --chain
// turing-dev
// --execution
// wasm
// --wasm-execution
// compiled
// --pallet
// pallet_xcmp_handler
// --extrinsic
// *
// --repeat
// 20
// --steps
// 50
// --output
// ./xcmp_handler-raw-weights.rs
// --template
// ./.maintain/frame-weight-template.hbs

// Summary:
//:add_chain_currency_data 14_936_000
//:remove_chain_currency_data 16_291_000

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_xcmp_handler.
pub trait WeightInfo {
	fn add_chain_currency_data() -> Weight;
	fn remove_chain_currency_data() -> Weight;
}

/// Weights for pallet_xcmp_handler using the Substrate node and recommended hardware.
pub struct pallet_xcmp_handlerWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for pallet_xcmp_handlerWeight<T> {
	// Storage: XcmpHandler XcmChainCurrencyData (r:0 w:1)
	fn add_chain_currency_data() -> Weight {
		(14_936_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: XcmpHandler XcmChainCurrencyData (r:1 w:1)
	fn remove_chain_currency_data() -> Weight {
		(16_291_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: XcmpHandler XcmChainCurrencyData (r:0 w:1)
	fn add_chain_currency_data() -> Weight {
		(14_936_000 as Weight)
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: XcmpHandler XcmChainCurrencyData (r:1 w:1)
	fn remove_chain_currency_data() -> Weight {
		(16_291_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
}
