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

use super::Balance;

/// Trait to make it easier to work with tokens.
/// Just needs `get_decimals` to be implemented.
pub trait TokenInfo {
	fn get_decimals(&self) -> u32;

	fn dollar(&self) -> Balance {
		let decimals = self.get_decimals();
		10_u128.pow(decimals)
	}

	fn cent(&self) -> Balance {
		self.dollar() / 100
	}

	fn millicent(&self) -> Balance {
		self.cent() / 1_000
	}
}

/// Allows you to translate between different tokens.
/// This is necessary as tokens use different amounts of "decimals".
pub fn convert_to_token<C: TokenInfo>(source: C, target: C, amount: Balance) -> Balance {
	let current_decimals = source.get_decimals();
	let target_decimals = target.get_decimals();
	if current_decimals >= target_decimals {
		let diff = current_decimals - target_decimals;
		return amount / 10_u128.pow(diff)
	} else {
		let diff = target_decimals - current_decimals;
		return amount * 10_u128.pow(diff)
	}
}
