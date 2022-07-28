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

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
pub use pallet_automation_time_rpc_runtime_api::AutomationTimeApi as AutomationTimeRuntimeApi;
use pallet_automation_time_rpc_runtime_api::{AutomationAction, AutostakingResult};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, AccountId32};
use std::{fmt::Debug, sync::Arc};

/// An RPC endpoint to provide information about tasks.
#[rpc(client, server)]
pub trait AutomationTimeApi<BlockHash, AccountId, Hash, Balance> {
	/// Generates the task_id given the account_id and provided_id.
	#[method(name = "automationTime_generateTaskId")]
	fn generate_task_id(
		&self,
		account: AccountId,
		provided_id: String,
		at: Option<BlockHash>,
	) -> RpcResult<Hash>;

	#[method(name = "automationTime_getTimeAutomationFees")]
	fn get_time_automation_fees(
		&self,
		action: AutomationAction,
		executions: u32,
		at: Option<BlockHash>,
	) -> RpcResult<u64>;

	/// Returns optimal autostaking period based on principal and a target collator.
	#[method(name = "automationTime_calculateOptimalAutostaking")]
	fn caclulate_optimal_autostaking(
		&self,
		principal: i128,
		collator: AccountId,
	) -> RpcResult<AutostakingResult>;

	#[method(name = "automationTime_getAutoCompoundDelegatedStakeTaskIds")]
	fn get_auto_compound_delegated_stake_task_ids(
		&self,
		account: AccountId,
	) -> RpcResult<Vec<Hash>>;

	#[method(name = "automationTime_crosssChainAccount")]
	fn cross_chain_account(&self, account: AccountId32) -> RpcResult<String>;
}

/// An implementation of Automation-specific RPC methods on full client.
pub struct AutomationTime<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> AutomationTime<C, B> {
	/// Create new `AutomationTaskUtility` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i32 {
	fn from(e: Error) -> i32 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

#[async_trait]
impl<C, Block, AccountId, Hash, Balance>
	AutomationTimeApiServer<<Block as BlockT>::Hash, AccountId, Hash, Balance>
	for AutomationTime<C, Block>
where
	Block: BlockT,
	Balance: Codec + Copy + TryInto<u64> + Debug,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AutomationTimeRuntimeApi<Block, AccountId, Hash, Balance>,
	AccountId: Codec,
	Hash: Codec,
{
	fn generate_task_id(
		&self,
		account: AccountId,
		provided_id: String,
		at: Option<Block::Hash>,
	) -> RpcResult<Hash> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let runtime_api_result =
			api.generate_task_id(&at, account, provided_id.as_bytes().to_vec());
		runtime_api_result.map_err(|e| {
			JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to generate task_id",
				Some(format!("{:?}", e)),
			)))
		})
	}

	fn get_time_automation_fees(
		&self,
		action: AutomationAction,
		executions: u32,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<u64> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		let runtime_api_result =
			api.get_time_automation_fees(&at, action, executions).map_err(|e| {
				CallError::Custom(ErrorObject::owned(
					Error::RuntimeError.into(),
					"Unable to get time automation fees",
					Some(e.to_string()),
				))
			})?;
		runtime_api_result.try_into().map_err(|_| {
			JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"RPC value doesn't fit in u64 representation",
				Some(format!("RPC value cannot be translated into u64 representation")),
			)))
		})
	}

	fn caclulate_optimal_autostaking(
		&self,
		principal: i128,
		collator: AccountId,
	) -> RpcResult<AutostakingResult> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		let runtime_api_result = api.calculate_optimal_autostaking(&at, principal, collator);
		let mapped_err = |message| -> JsonRpseeError {
			JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to calculate optimal autostaking",
				Some(message),
			)))
		};
		runtime_api_result
			.map_err(|e| mapped_err(format!("{:?}", e)))
			.map(|r| r.map_err(|e| mapped_err(String::from_utf8(e).unwrap_or(String::default()))))?
	}

	fn get_auto_compound_delegated_stake_task_ids(
		&self,
		account: AccountId,
	) -> RpcResult<Vec<Hash>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);

		let runtime_api_result = api.get_auto_compound_delegated_stake_task_ids(&at, account);
		runtime_api_result.map_err(|e| {
			JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get AutoCompoundDelegatedStakeTask ids",
				Some(format!("{:?}", e)),
			)))
		})
	}

	fn cross_chain_account(&self, account_id: AccountId32) -> RpcResult<String> {
		use codec::{Decode, Encode};
		use sp_io::hashing::blake2_256;
		use sp_runtime::AccountId32;
		use xcm::latest::{prelude::*, Junction::*, Junctions::*, MultiLocation, NetworkId};
		let account = AccountId32::from(account_id);
		let oak_multiloc = MultiLocation::new(
			1,
			X2(Parachain(2114), Junction::AccountId32 { network: Any, id: account.into() }),
		);

		let cross_chain_account: AccountId32 =
			("multiloc", oak_multiloc).using_encoded(blake2_256).into();
		Ok(format!("{:}", cross_chain_account))
	}
}
