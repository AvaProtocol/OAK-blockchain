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

use codec::{Codec, Decode};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
pub use pallet_automation_time_rpc_runtime_api::AutomationTimeApi as AutomationTimeRuntimeApi;
use pallet_automation_time_rpc_runtime_api::{AutomationAction, AutostakingResult, FeeDetails};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay},
};
use std::sync::Arc;

/// An RPC endpoint to provide information about tasks.
#[rpc(client, server)]
pub trait AutomationTimeApi<BlockHash, AccountId, Hash, Balance> {
	#[method(name = "automationTime_queryFeeDetails")]
	fn query_fee_details(
		&self,
		encoded_xt: Bytes,
		at: Option<BlockHash>,
	) -> RpcResult<FeeDetails<NumberOrHex>>;

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
	) -> RpcResult<Vec<Vec<u8>>>;
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
	Balance:
		Codec + MaybeDisplay + Copy + TryInto<NumberOrHex> + TryInto<u64> + Send + Sync + 'static,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AutomationTimeRuntimeApi<Block, AccountId, Hash, Balance>,
	AccountId: Codec,
	Hash: Codec,
{
	fn query_fee_details(
		&self,
		encoded_xt: Bytes,
		at: Option<Block::Hash>,
	) -> RpcResult<FeeDetails<NumberOrHex>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let uxt: Block::Extrinsic = Decode::decode(&mut &*encoded_xt).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				format!("Unable to decode extrinsic."),
				Some(format!("{:?}", e)),
			))
		})?;
		let fee_details = api
			.query_fee_details(&at, uxt)
			.map_err(|e| {
				CallError::Custom(ErrorObject::owned(
					Error::RuntimeError.into(),
					format!("Unable to query fee details."),
					Some(format!("{:?}", e)),
				))
			})?
			.map_err(|e| {
				JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					Error::RuntimeError.into(),
					"Unable to get fees.",
					Some(String::from_utf8(e).unwrap_or(String::default())),
				)))
			})?;

		let try_into_rpc_balance = |value: Balance| {
			value.try_into().map_err(|_| {
				JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					Error::RuntimeError.into(),
					format!("{} doesn't fit in NumberOrHex representation", value),
					None::<()>,
				)))
			})
		};

		Ok(FeeDetails {
			schedule_fee: try_into_rpc_balance(fee_details.schedule_fee)?,
			execution_fee: try_into_rpc_balance(fee_details.execution_fee)?,
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
	) -> RpcResult<Vec<Vec<u8>>> {
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
}
