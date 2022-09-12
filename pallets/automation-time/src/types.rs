use crate::{weights::WeightInfo, Config, Error};

use frame_support::{pallet_prelude::*, traits::Get, weights::GetDispatchInfo};

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
	DynamicDispatch {
		encoded_call: Vec<u8>,
	},
}

impl<AccountId, Balance, CurrencyId> Action<AccountId, Balance, CurrencyId> {
	pub fn execution_weight<T: Config>(&self) -> Result<Weight, DispatchError> {
		let weight = match self {
			Action::Notify { .. } => <T as Config>::WeightInfo::run_notify_task(),
			Action::NativeTransfer { .. } => <T as Config>::WeightInfo::run_native_transfer_task(),
			// Adding 1 DB write that doesn't get accounted for in the benchmarks to run an xcmp task
			Action::XCMP { .. } => T::DbWeight::get()
				.writes(1)
				.saturating_add(<T as Config>::WeightInfo::run_xcmp_task()),
			Action::AutoCompoundDelegatedStake { .. } =>
				<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
			Action::DynamicDispatch { encoded_call } => {
				let scheduled_call: <T as Config>::Call = Decode::decode(&mut &**encoded_call)
					.map_err(|_| Error::<T>::CallCannotBeDecoded)?;
				<T as Config>::WeightInfo::run_dynamic_dispatch_action()
					.saturating_add(scheduled_call.get_dispatch_info().weight)
			},
		};
		Ok(weight)
	}
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

#[derive(Debug, Decode, Eq, Encode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ScheduledTasks<AccountId, TaskId> {
	pub tasks: Vec<(AccountId, TaskId)>,
	pub weight: u128,
}
impl<A, B> Default for ScheduledTasks<A, B> {
	fn default() -> Self {
		Self { tasks: vec![], weight: 0 }
	}
}
impl<AccountId, TaskId> ScheduledTasks<AccountId, TaskId> {
	pub fn try_push<T: Config, Balance>(
		&mut self,
		task_id: TaskId,
		task: &Task<AccountId, Balance, T::CurrencyId, T::MaxExecutionTimes>,
	) -> Result<&mut Self, DispatchError>
	where
		AccountId: Clone,
	{
		let action_weight = task.action.execution_weight::<T>()?;
		let weight =
			self.weight.checked_add(action_weight as u128).ok_or(Error::<T>::TimeSlotFull)?;
		// A hard limit on tasks/slot prevents unforseen performance consequences
		// that could occur when scheduling a huge number of lightweight tasks.
		// Also allows us to make reasonable assumptions for worst case benchmarks.
		if self.tasks.len() as u32 >= T::MaxTasksPerSlot::get() ||
			weight > T::MaxWeightPerSlot::get()
		{
			Err(Error::<T>::TimeSlotFull)?
		}

		self.weight = weight;
		self.tasks.push((task.owner_id.clone(), task_id));
		Ok(self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;

	mod scheduled_tasks {
		use super::*;
		use crate::{AccountTaskId, BalanceOf, ScheduledTasksOf, TaskId, TaskOf};
		use frame_support::assert_err;
		use sp_runtime::AccountId32;

		#[test]
		fn try_push_errors_when_slot_is_full_by_weight() {
			new_test_ext(0).execute_with(|| {
				let task = TaskOf::<Test>::create_event_task(
					AccountId32::new(ALICE),
					vec![0],
					vec![0].try_into().unwrap(),
					vec![0],
				);
				let task_id = TaskId::<Test>::default();
				assert_err!(
					ScheduledTasksOf::<Test> { tasks: vec![], weight: MaxWeightPerSlot::get() }
						.try_push::<Test, BalanceOf<Test>>(task_id, &task),
					Error::<Test>::TimeSlotFull
				);
			})
		}

		#[test]
		fn try_push_errors_when_slot_is_full_by_task_count() {
			new_test_ext(0).execute_with(|| {
				let alice = AccountId32::new(ALICE);
				let id = (alice.clone(), TaskId::<Test>::default());
				let task = TaskOf::<Test>::create_event_task(
					alice.clone(),
					vec![0],
					vec![0].try_into().unwrap(),
					vec![0],
				);
				let tasks = (0..MaxTasksPerSlot::get()).fold::<Vec<AccountTaskId<Test>>, _>(
					vec![],
					|mut tasks, _| {
						tasks.push(id.clone());
						tasks
					},
				);
				let task_id = TaskId::<Test>::default();
				assert_err!(
					ScheduledTasksOf::<Test> { tasks, weight: 0 }
						.try_push::<Test, BalanceOf<Test>>(task_id, &task),
					Error::<Test>::TimeSlotFull
				);
			})
		}

		#[test]
		fn try_push_works_when_slot_is_not_full() {
			new_test_ext(0).execute_with(|| {
				let task = TaskOf::<Test>::create_event_task(
					AccountId32::new(ALICE),
					vec![0],
					vec![0].try_into().unwrap(),
					vec![0],
				);
				let task_id = TaskId::<Test>::default();
				let mut scheduled_tasks = ScheduledTasksOf::<Test>::default();
				scheduled_tasks
					.try_push::<Test, BalanceOf<Test>>(task_id, &task)
					.expect("slot is not full");
				assert_eq!(scheduled_tasks.tasks, vec![(task.owner_id, task_id)]);
				assert_eq!(scheduled_tasks.weight, 20_000);
			})
		}
	}
}
