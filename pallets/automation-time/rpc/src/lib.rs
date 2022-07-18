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
use pallet_automation_time_rpc_runtime_api::AutostakingResult;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

/// An RPC endpoint to provide information about tasks.
#[rpc(client, server)]
pub trait AutomationTimeApi<BlockHash, AccountId, Hash> {
	/// Generates the task_id given the account_id and provided_id.
	#[method(name = "automationTime_generateTaskId")]
	fn generate_task_id(
		&self,
		account: AccountId,
		provided_id: String,
		at: Option<BlockHash>,
	) -> RpcResult<Hash>;

	/// Returns optimal autostaking period based on principal and a target collator.
	#[method(name = "automationTime_calculateOptimalAutostaking")]
	fn caclulate_optimal_autostaking(
		&self,
		principal: i128,
		collator: AccountId,
	) -> RpcResult<AutostakingResult>;
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
impl<C, Block, AccountId, Hash> AutomationTimeApiServer<<Block as BlockT>::Hash, AccountId, Hash>
	for AutomationTime<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AutomationTimeRuntimeApi<Block, AccountId, Hash>,
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

	fn caclulate_optimal_autostaking(
		&self,
		principal: i128,
		collator: AccountId,
	) -> RpcResult<AutostakingResult> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		let runtime_api_result = api.calculate_optimal_autostaking(&at, principal, collator);
		runtime_api_result.map_err(|e| {
			JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to calculate optimal autostaking",
				Some(format!("{:?}", e)),
			)))
		})
	}
}
