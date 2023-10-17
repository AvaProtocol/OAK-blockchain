// This file is part of OAK Blockchain.

// Copyright (C) 2023 OAK Network
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

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct FeeDetails<Balance> {
	pub schedule_fee: Balance,
	pub execution_fee: Balance,
}

sp_api::decl_runtime_apis! {
	pub trait AutomationPriceApi<AccountId, Hash, Balance> where
		AccountId: Codec,
		Hash: Codec,
		Balance: Codec,
	{
		fn query_fee_details(uxt: Block::Extrinsic) -> Result<FeeDetails<Balance>, Vec<u8>>;
	}
}
