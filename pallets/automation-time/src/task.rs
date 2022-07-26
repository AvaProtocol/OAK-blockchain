use crate::{
	pallet::{AccountOf, BalanceOf, Config, Seconds, Tasks, UnixTime},
	Action, Decode, Encode, ParaId, TypeInfo,
};

use frame_support::{weights::Weight, BoundedVec};
use sp_std::{vec, vec::Vec};

/// The struct that stores all information needed for a task.
#[derive(Debug, Eq, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct Task<T: Config> {
	pub owner_id: AccountOf<T>,
	pub provided_id: Vec<u8>,
	pub execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
	pub executions_left: u32,
	pub action: Action<T>,
}

impl<T: Config> Task<T> {
	pub fn create_task(
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
		execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
		action: Action<T>,
	) -> Task<T> {
		let executions_left: u32 = execution_times.len().try_into().unwrap();
		Task::<T> { owner_id, provided_id, execution_times, executions_left, action }
	}

	pub fn create_event_task(
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
		execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
		message: Vec<u8>,
	) -> Task<T> {
		let action = Action::Notify { message };
		Self::create_task(owner_id, provided_id, execution_times, action)
	}

	pub fn create_native_transfer_task(
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
		execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
		recipient_id: AccountOf<T>,
		amount: BalanceOf<T>,
	) -> Task<T> {
		let action =
			Action::NativeTransfer { sender: owner_id.clone(), recipient: recipient_id, amount };
		Self::create_task(owner_id, provided_id, execution_times, action)
	}

	pub fn create_xcmp_task(
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
		execution_times: BoundedVec<UnixTime, T::MaxExecutionTimes>,
		para_id: ParaId,
		call: Vec<u8>,
		weight_at_most: Weight,
	) -> Task<T> {
		let action = Action::XCMP { para_id, call, weight_at_most };
		Self::create_task(owner_id, provided_id, execution_times, action)
	}

	pub fn create_auto_compound_delegated_stake_task(
		owner_id: AccountOf<T>,
		provided_id: Vec<u8>,
		execution_time: UnixTime,
		frequency: Seconds,
		collator_id: AccountOf<T>,
		account_minimum: BalanceOf<T>,
	) -> Task<T> {
		let action = Action::AutoCompoundDelegatedStake {
			delegator: owner_id.clone(),
			collator: collator_id,
			account_minimum,
			frequency,
		};
		Self::create_task(owner_id, provided_id, vec![execution_time].try_into().unwrap(), action)
	}

	pub fn clean_execution_times_vector(v: &mut Vec<UnixTime>) {
		v.sort_unstable();
		v.dedup();
	}

	pub fn execute(self, task_id: T::Hash) -> Weight {
		let action = self.action.clone();
		let (weight, mut task) = action.execute(task_id, self);
		task.decrement_task_and_remove_if_complete(task_id);
		weight
	}

	pub fn decrement_task_and_remove_if_complete(&mut self, task_id: T::Hash) {
		self.executions_left = self.executions_left.saturating_sub(1);
		if self.executions_left <= 0 {
			Tasks::<T>::remove(task_id);
		} else {
			Tasks::<T>::insert(task_id, self);
		}
	}

	/// Cleans the executions times by removing duplicates and putting in ascending order.
	pub fn clean_execution_times(&mut self) -> &Self {
		let mut execution_times = self.execution_times.to_vec();
		Self::clean_execution_times_vector(&mut execution_times);
		self.execution_times = execution_times.try_into().expect("Vec did not grow or change type");
		self
	}
}

/// Needed for assert_eq to compare Tasks in tests due to BoundedVec.
impl<T: Config> PartialEq for Task<T> {
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
