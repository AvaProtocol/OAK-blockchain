// Holding the old data structure so we can share them betwen multiple migrations
use crate::{
	weights::WeightInfo, AccountOf, AccountTaskId, Action, ActionOf, AssetPayment, BalanceOf,
	Config, InstructionSequence, MissedTaskV2, MissedTaskV2Of, Schedule, TaskIdV2, TaskOf,
	UnixTime,
};
use frame_system::pallet_prelude::*;

use codec::{Decode, Encode};
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::EncodeLike, storage::types::ValueQuery, traits::OnRuntimeUpgrade, weights::Weight,
	Twox64Concat,
};

use scale_info::{prelude::format, TypeInfo};
use sp_runtime::traits::Convert;
use sp_std::{vec, vec::Vec};
use xcm::{latest::prelude::*, VersionedMultiLocation};

// These are H256/BlakeTwo256 hex generate from our old task id generation from hashing
// These cons are used for our unit test
// (Account, ProvidedID)
pub const TEST_TASKID1: &str = "0xd1263842e34adeb00be1146b30bc6527951f0a0f5d5a3f7a758537735b8bcb04";
pub const TEST_TASKID2: &str = "0xf76acf0b1d8ef503450a4d5c1a451f1921a906e8711f551c2945e09fb44de5ff";

// Old format
pub type OldTaskId<T> = <T as frame_system::Config>::Hash;
pub type OldAccountTaskId<T> = (AccountOf<T>, OldTaskId<T>);

#[derive(Debug, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(MaxExecutionTimes))]
pub struct OldTask<T: Config> {
	pub owner_id: T::AccountId,
	pub provided_id: Vec<u8>,
	pub schedule: Schedule,
	pub action: OldAction<T>,
}

impl<T: Config> From<OldTask<T>> for TaskOf<T> {
	fn from(task: OldTask<T>) -> Self {
		// Previously, our task id is a blake256 hash of account_id + provided_id
		// Now, our task id is generate use a different formula without relying on `provided_id`
		// for existing task in our storage, the type of the TaskID was T::Hash.
		// When emitting on the event, it can be re-present as hex string of the hash.
		// so in new task, we simply take that hash and convert it to the hex string, and convert
		// to a Vec<u8> which is our new taskidv2 format.
		let old_task_id =
			deprecate::generate_old_task_id::<T>(task.owner_id.clone(), task.provided_id.clone());

		TaskOf::<T> {
			owner_id: task.owner_id,
			task_id: deprecate::old_taskid_to_idv2::<T>(&old_task_id),
			schedule: task.schedule,
			action: task.action.into(),
		}
	}
}

/// The enum that stores all action specific data.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum OldAction<T: Config> {
	Notify {
		message: Vec<u8>,
	},
	NativeTransfer {
		sender: T::AccountId,
		recipient: T::AccountId,
		amount: BalanceOf<T>,
	},
	XCMP {
		para_id: ParaId,
		currency_id: T::CurrencyId,
		xcm_asset_location: VersionedMultiLocation,
		encoded_call: Vec<u8>,
		encoded_call_weight: Weight,
		schedule_as: Option<T::AccountId>,
	},
	AutoCompoundDelegatedStake {
		delegator: T::AccountId,
		collator: T::AccountId,
		account_minimum: BalanceOf<T>,
	},
	DynamicDispatch {
		encoded_call: Vec<u8>,
	},
}

pub type OldScheduledTasksOf<T> = OldScheduledTasks<AccountOf<T>, OldTaskId<T>>;

#[derive(Debug, Decode, Eq, Encode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct OldScheduledTasks<AccountId, OldTaskId> {
	pub tasks: Vec<(AccountId, OldTaskId)>,
	pub weight: u128,
}

impl<A, B> Default for OldScheduledTasks<A, B> {
	fn default() -> Self {
		Self { tasks: vec![], weight: 0 }
	}
}

#[derive(Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub struct OldMissedTaskV2<AccountId, TaskId> {
	pub owner_id: AccountId,
	pub task_id: TaskId,
	pub execution_time: UnixTime,
}

pub type OldMissedTaskV2Of<T> = OldMissedTaskV2<AccountOf<T>, OldTaskId<T>>;

pub mod deprecate {
	use super::*;

	use sp_runtime::traits::Hash;

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

	pub fn old_taskid_to_idv2<T: frame_system::Config>(old_task_id: &OldTaskId<T>) -> Vec<u8> {
		let task_id = format!("{:?}", old_task_id);
		return task_id.as_bytes().to_vec()
	}

	pub fn generate_old_task_id<T: frame_system::Config>(
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
	) -> OldTaskId<T> {
		let task_hash_input = TaskHashInput::new(owner_id, provided_id);
		T::Hashing::hash_of(&task_hash_input)
	}
}
