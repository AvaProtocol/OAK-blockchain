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

use sp_core::H256;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiAddress, MultiSignature,
};
use sp_std::{marker::PhantomData, prelude::*};

use frame_support::traits::Get;

use orml_traits::location::{RelativeReserveProvider, Reserve};
use xcm::latest::prelude::*;
use xcm::v3::MultiAsset;

pub mod assets;

pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// The signed version of `Balance`
pub type Amount = i128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = H256;

/// An index to a block.
pub type BlockNumber = u32;

/// Identifier of a token or asset
pub type TokenId = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Opaque, encoded, unchecked extrinsic.
pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

pub trait EnsureProxy<AccountId> {
	fn ensure_ok(delegator: AccountId, delegatee: AccountId) -> Result<(), &'static str>;
}

pub trait TransferCallCreator<AccountId, Balance, RuntimeCall> {
	fn create_transfer_call(dest: AccountId, value: Balance) -> RuntimeCall;
}

/// `MultiAsset` reserve location provider. It's based on `RelativeReserveProvider` and in
/// addition will convert self absolute location to relative location.
pub struct AbsoluteAndRelativeReserveProvider<AbsoluteLocation>(PhantomData<AbsoluteLocation>);
impl<AbsoluteLocation: Get<MultiLocation>> Reserve
	for AbsoluteAndRelativeReserveProvider<AbsoluteLocation>
{
	fn reserve(asset: &MultiAsset) -> Option<MultiLocation> {
		RelativeReserveProvider::reserve(asset).map(|reserve_location| {
			if reserve_location == AbsoluteLocation::get() {
				MultiLocation::here()
			} else {
				reserve_location
			}
		})
	}
}

// https://github.com/AstarNetwork/Astar/blob/2a8df8d098f6abc3fdf5aaf6514e94f615f9bdab/primitives/src/xcm/mod.rs#L276-L418
// TODO: remove this after uplift to `polkadot-v0.9.44` or beyond, and replace it with code in XCM builder.

use codec::{Compact, Encode};
use sp_io::hashing::blake2_256;
use xcm_executor::traits::ConvertLocation;

pub type DescribeAccountIdTerminal =
	(xcm_builder::DescribeAccountId32Terminal, xcm_builder::DescribeAccountKey20Terminal);

pub struct DescribeBodyTerminal;
impl xcm_builder::DescribeLocation for DescribeBodyTerminal {
	fn describe_location(l: &MultiLocation) -> Option<Vec<u8>> {
		match (l.parents, &l.interior) {
			(0, X1(Plurality { id, part })) => Some((b"Body", id, part).encode()),
			_ => None,
		}
	}
}

pub type DescribeAllTerminal = (
	xcm_builder::DescribeTerminus,
	xcm_builder::DescribePalletTerminal,
	xcm_builder::DescribeAccountId32Terminal,
	xcm_builder::DescribeAccountKey20Terminal,
	DescribeBodyTerminal,
);

pub struct HashedDescription<AccountId, Describe>(PhantomData<(AccountId, Describe)>);

impl<AccountId: From<[u8; 32]> + Clone, Describe: xcm_builder::DescribeLocation>
	ConvertLocation<AccountId> for HashedDescription<AccountId, Describe>
{
	fn convert_location(location: &MultiLocation) -> Option<AccountId> {
		if let Some(description) = Describe::describe_location(location) {
			Some(blake2_256(&description).into())
		} else {
			None
		}
	}
}
