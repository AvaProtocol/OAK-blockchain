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

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", serde(deny_unknown_fields))]
pub struct AutostakingResult {
	pub period: i32,
	pub apy: String,
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AutomationAction {
	XCMP,
	AutoCompoundDelegatedStake,
}

#[derive(Debug, PartialEq, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct FeeDetails<Balance> {
	pub schedule_fee: Balance,
	pub execution_fee: Balance,
}

sp_api::decl_runtime_apis! {
	pub trait AutomationTimeApi<AccountId, Hash, Balance> where
		AccountId: Codec,
		Hash: Codec,
		Balance: Codec,
	{
		fn query_fee_details(uxt: Block::Extrinsic) -> Result<FeeDetails<Balance>, Vec<u8>>;
		fn calculate_optimal_autostaking(
			principal: i128,
			collator: AccountId
		) -> Result<AutostakingResult, Vec<u8>>;
		fn get_auto_compound_delegated_stake_task_ids(account_id: AccountId) -> Vec<Vec<u8>>;
	}
}
