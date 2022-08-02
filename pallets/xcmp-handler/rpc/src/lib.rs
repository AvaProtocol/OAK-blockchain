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
use frame_support::parameter_types;
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
pub use pallet_xcmp_handler_rpc_runtime_api::XcmpHandlerApi as XcmpHandlerRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, AccountId32};
use std::{fmt::Debug, sync::Arc};
use xcm::{
	latest::{prelude::*, MultiLocation, NetworkId},
	v1::Junction::Parachain,
};
use xcm_builder::Account32Hash;
use xcm_executor::traits::Convert;

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Any;
}

/// An RPC endpoint to provide information about xcmp.
#[rpc(client, server)]
pub trait XcmpHandlerApi<Block, Hash, Balance> {
	#[method(name = "xcmpHandler_crossChainAccount")]
	fn cross_chain_account(&self, account: AccountId32) -> RpcResult<AccountId32>;
	#[method(name = "xcmpHandler_fees")]
	fn fees(&self) -> RpcResult<u64>;
}

/// An implementation of XCMP-specific RPC methods on full client.
pub struct XcmpHandler<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> XcmpHandler<C, B> {
	/// Create new `XcmpHandlerUtility` with the given reference to the client.
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
impl<C, Block, Hash, Balance> XcmpHandlerApiServer<<Block as BlockT>::Hash, Hash, Balance>
	for XcmpHandler<C, Block>
where
	Block: BlockT,
	Balance: Codec + Copy + TryInto<u64> + Debug,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: XcmpHandlerRuntimeApi<Block, Hash, Balance>,
	Hash: Codec,
{
	fn cross_chain_account(&self, account_id: AccountId32) -> RpcResult<AccountId32> {
		let multiloc = MultiLocation::new(
			1,
			X2(Parachain(2114), Junction::AccountId32 { network: Any, id: account_id.into() }),
		);
		Account32Hash::<RelayNetwork, sp_runtime::AccountId32>::convert_ref(multiloc).map_err(|e| {
			JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get cross chain AccountId",
				Some(format!("{:?}", e)),
			)))
		})
	}

	fn fees(&self) -> RpcResult<u64> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		let runtime_api_result = api.fees(&at).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get fees",
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
}
