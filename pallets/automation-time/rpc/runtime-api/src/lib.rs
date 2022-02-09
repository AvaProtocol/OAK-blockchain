#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait AutomationTimeApi<AccountId, Hash> where
		AccountId: Codec,
		Hash: Codec,
	{
		fn generate_task_id(account_id: AccountId, provided_id: Vec<u8>) -> Hash;
	}
}
