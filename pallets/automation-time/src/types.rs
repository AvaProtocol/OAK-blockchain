use frame_support::pallet_prelude::*;

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct TaskHashInput<AccountId> {
	owner_id: AccountId,
	provided_id: Vec<u8>,
}

impl<AccountId> TaskHashInput<AccountId> {
	pub fn new(owner_id: AccountId, provided_id: Vec<u8>) -> Self {
		Self { owner_id, provided_id }
	}
}
