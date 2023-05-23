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
use frame_support::traits::Get;
use orml_traits::currency::MutationHooks;
use sp_std::marker::PhantomData;

pub mod constants;
pub mod fees;

pub struct CurrencyHooks<T, DustAccount>(PhantomData<T>, DustAccount);
impl<T, DustAccount> MutationHooks<T::AccountId, T::CurrencyId, T::Balance>
	for CurrencyHooks<T, DustAccount>
where
	T: orml_tokens::Config,
	DustAccount: Get<<T as frame_system::Config>::AccountId>,
{
	type OnDust = orml_tokens::TransferDust<T, DustAccount>;
	type OnSlash = ();
	type PreDeposit = ();
	type PostDeposit = ();
	type PreTransfer = ();
	type PostTransfer = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}
