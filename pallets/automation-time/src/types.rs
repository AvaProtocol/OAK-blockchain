use frame_support::{pallet_prelude::*, traits::Get};

use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_std::prelude::*;

use cumulus_primitives_core::ParaId;

use pallet_automation_time_rpc_runtime_api::AutomationAction;

pub type Seconds = u64;
pub type UnixTime = u64;

/// The enum that stores all action specific data.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum Action<AccountId, Balance, CurrencyId> {
	Notify {
		message: Vec<u8>,
	},
	NativeTransfer {
		sender: AccountId,
		recipient: AccountId,
		amount: Balance,
	},
	XCMP {
		para_id: ParaId,
		currency_id: CurrencyId,
		encoded_call: Vec<u8>,
		encoded_call_weight: Weight,
	},
	AutoCompoundDelegatedStake {
		delegator: AccountId,
		collator: AccountId,
		account_minimum: Balance,
		frequency: Seconds,
	},
}

impl<AccountId: Clone + Decode, Balance: AtLeast32BitUnsigned, CurrencyId: Default>
	From<AutomationAction> for Action<AccountId, Balance, CurrencyId>
{
	fn from(a: AutomationAction) -> Self {
		let default_account =
			AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
				.expect("always valid");
		match a {
			AutomationAction::Notify => Action::Notify { message: "default".into() },
			AutomationAction::NativeTransfer => Action::NativeTransfer {
				sender: default_account.clone(),
				recipient: default_account,
				amount: 0u32.into(),
			},
			AutomationAction::XCMP => Action::XCMP {
				para_id: ParaId::from(2114 as u32),
				currency_id: CurrencyId::default(),
				encoded_call: vec![0],
				encoded_call_weight: 0,
			},
			AutomationAction::AutoCompoundDelegatedStake => Action::AutoCompoundDelegatedStake {
				delegator: default_account.clone(),
				collator: default_account,
				account_minimum: 0u32.into(),
				frequency: 0,
			},
		}
	}
}

/// The struct that stores all information needed for a task.
#[derive(Debug, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(MaxExecutionTimes))]
pub struct Task<AccountId, Balance, CurrencyId, MaxExecutionTimes: Get<u32>> {
	pub owner_id: AccountId,
	pub provided_id: Vec<u8>,
	pub execution_times: BoundedVec<UnixTime, MaxExecutionTimes>,
	pub executions_left: u32,
	pub action: Action<AccountId, Balance, CurrencyId>,
}

impl<AccountId: Ord, Balance: Ord, CurrencyId: Ord, MaxExecutionTimes: Get<u32>> PartialEq
	for Task<AccountId, Balance, CurrencyId, MaxExecutionTimes>
{
	fn eq(&self, other: &Self) -> bool {
		self.owner_id == other.owner_id &&
			self.provided_id == other.provided_id &&
			self.action == other.action &&
			self.executions_left == other.executions_left &&
			self.execution_times.len() == other.execution_times.len() &&
			self.execution_times.capacity() == other.execution_times.capacity() &&
			self.execution_times.to_vec() == other.execution_times.to_vec()
	}
}

impl<AccountId: Ord, Balance: Ord, CurrencyId: Ord, MaxExecutionTimes: Get<u32>> Eq
	for Task<AccountId, Balance, CurrencyId, MaxExecutionTimes>
{
}

impl<AccountId: Clone, Balance, CurrencyId, MaxExecutionTimes: Get<u32>>
	Task<AccountId, Balance, CurrencyId, MaxExecutionTimes>
{
	pub fn new(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		execution_times: BoundedVec<UnixTime, MaxExecutionTimes>,
		action: Action<AccountId, Balance, CurrencyId>,
	) -> Self {
		let executions_left: u32 = execution_times.len().try_into().unwrap();
		Self { owner_id, provided_id, execution_times, executions_left, action }
	}

	pub fn create_event_task(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		execution_times: BoundedVec<UnixTime, MaxExecutionTimes>,
		message: Vec<u8>,
	) -> Self {
		let action = Action::Notify { message };
		Self::new(owner_id, provided_id, execution_times, action)
	}

	pub fn create_native_transfer_task(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		execution_times: BoundedVec<UnixTime, MaxExecutionTimes>,
		recipient_id: AccountId,
		amount: Balance,
	) -> Self {
		let action =
			Action::NativeTransfer { sender: owner_id.clone(), recipient: recipient_id, amount };
		Self::new(owner_id, provided_id, execution_times, action)
	}

	pub fn create_xcmp_task(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		execution_times: BoundedVec<UnixTime, MaxExecutionTimes>,
		para_id: ParaId,
		currency_id: CurrencyId,
		encoded_call: Vec<u8>,
		encoded_call_weight: Weight,
	) -> Self {
		let action = Action::XCMP { para_id, currency_id, encoded_call, encoded_call_weight };
		Self::new(owner_id, provided_id, execution_times, action)
	}

	pub fn create_auto_compound_delegated_stake_task(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		execution_time: UnixTime,
		frequency: Seconds,
		collator_id: AccountId,
		account_minimum: Balance,
	) -> Self {
		let action = Action::AutoCompoundDelegatedStake {
			delegator: owner_id.clone(),
			collator: collator_id,
			account_minimum,
			frequency,
		};
		Self::new(owner_id, provided_id, vec![execution_time].try_into().unwrap(), action)
	}
}

#[derive(Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub struct MissedTaskV2<AccountId, TaskId> {
	pub owner_id: AccountId,
	pub task_id: TaskId,
	pub execution_time: UnixTime,
}

impl<AccountId, TaskId> MissedTaskV2<AccountId, TaskId> {
	pub fn new(owner_id: AccountId, task_id: TaskId, execution_time: UnixTime) -> Self {
		Self { owner_id, task_id, execution_time }
	}
}

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
