use crate::{weights::WeightInfo, Config, Error, Pallet};

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
			Action::XCMP { .. } => <T as Config>::WeightInfo::run_xcmp_task(),
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
			},
		}
	}
}

#[derive(Debug, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(MaxExecutionTimes))]
pub enum Schedule<MaxExecutionTimes: Get<u32>> {
	Fixed { execution_times: BoundedVec<UnixTime, MaxExecutionTimes>, executions_left: u32 },
	Recurring { next_execution_time: UnixTime, frequency: Seconds },
}

impl<B: Get<u32>> PartialEq for Schedule<B> {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(
				Schedule::Fixed { execution_times: t1, executions_left: l1 },
				Schedule::Fixed { execution_times: t2, executions_left: l2 },
			) => t1.to_vec() == t2.to_vec() && l1 == l2,
			(
				Schedule::Recurring { next_execution_time: t1, frequency: f1 },
				Schedule::Recurring { next_execution_time: t2, frequency: f2 },
			) => t1 == t2 && f1 == f2,
			_ => false,
		}
	}
}

impl<MaxExecutionTimes: Get<u32>> Schedule<MaxExecutionTimes> {
	pub fn new_fixed_schedule<T: Config>(
		mut execution_times: Vec<UnixTime>,
	) -> Result<Self, DispatchError> {
		Pallet::<T>::clean_execution_times_vector(&mut execution_times);
		let executions_left = execution_times.len() as u32;
		let execution_times: BoundedVec<UnixTime, MaxExecutionTimes> =
			execution_times.try_into().map_err(|_| Error::<T>::TooManyExecutionsTimes)?;
		let schedule = Self::Fixed { execution_times, executions_left };
		schedule.valid::<T>()?;
		Ok(schedule)
	}

	pub fn new_recurring_schedule<T: Config>(
		next_execution_time: UnixTime,
		frequency: Seconds,
	) -> Result<Self, DispatchError> {
		let schedule = Self::Recurring { next_execution_time, frequency };
		schedule.valid::<T>()?;
		Ok(schedule)
	}

	pub fn number_of_known_executions(&self) -> u32 {
		match self {
			Self::Fixed { executions_left, .. } => *executions_left,
			Self::Recurring { .. } => 1,
		}
	}

	fn valid<T: Config>(&self) -> DispatchResult {
		match self {
			Self::Fixed { execution_times, .. } =>
				for time in execution_times.iter() {
					Pallet::<T>::is_valid_time(*time)?;
				},
			Self::Recurring { next_execution_time, frequency } => {
				Pallet::<T>::is_valid_time(*next_execution_time)?;
				// Validate frequency by ensuring that the next proposed execution is at a valid time
				let next_recurrence =
					next_execution_time.checked_add(*frequency).ok_or(Error::<T>::TimeTooFarOut)?;
				if *next_execution_time == next_recurrence {
					Err(Error::<T>::InvalidTime)?;
				}
				Pallet::<T>::is_valid_time(next_recurrence)?;
			},
		}
		Ok(())
	}
}

/// The struct that stores all information needed for a task.
#[derive(Debug, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(MaxExecutionTimes))]
pub struct Task<AccountId, Balance, CurrencyId, MaxExecutionTimes: Get<u32>> {
	pub owner_id: AccountId,
	pub provided_id: Vec<u8>,
	pub schedule: Schedule<MaxExecutionTimes>,
	pub action: Action<AccountId, Balance, CurrencyId>,
}

impl<AccountId: Ord, Balance: Ord, CurrencyId: Ord, MaxExecutionTimes: Get<u32>> PartialEq
	for Task<AccountId, Balance, CurrencyId, MaxExecutionTimes>
{
	fn eq(&self, other: &Self) -> bool {
		self.owner_id == other.owner_id &&
			self.provided_id == other.provided_id &&
			self.action == other.action &&
			self.schedule == other.schedule
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
		schedule: Schedule<MaxExecutionTimes>,
		action: Action<AccountId, Balance, CurrencyId>,
	) -> Self {
		Self { owner_id, provided_id, schedule, action }
	}

	pub fn create_event_task<T: Config>(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		execution_times: Vec<UnixTime>,
		message: Vec<u8>,
	) -> Result<Self, DispatchError> {
		let action = Action::Notify { message };
		let schedule = Schedule::<MaxExecutionTimes>::new_fixed_schedule::<T>(execution_times)?;
		Ok(Self::new(owner_id, provided_id, schedule, action))
	}

	pub fn create_native_transfer_task<T: Config>(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		execution_times: Vec<UnixTime>,
		recipient_id: AccountId,
		amount: Balance,
	) -> Result<Self, DispatchError> {
		let action =
			Action::NativeTransfer { sender: owner_id.clone(), recipient: recipient_id, amount };
		let schedule = Schedule::<MaxExecutionTimes>::new_fixed_schedule::<T>(execution_times)?;
		Ok(Self::new(owner_id, provided_id, schedule, action))
	}

	pub fn create_xcmp_task<T: Config>(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		execution_times: Vec<UnixTime>,
		para_id: ParaId,
		currency_id: CurrencyId,
		encoded_call: Vec<u8>,
		encoded_call_weight: Weight,
	) -> Result<Self, DispatchError> {
		let action = Action::XCMP { para_id, currency_id, encoded_call, encoded_call_weight };
		let schedule = Schedule::<MaxExecutionTimes>::new_fixed_schedule::<T>(execution_times)?;
		Ok(Self::new(owner_id, provided_id, schedule, action))
	}

	pub fn create_auto_compound_delegated_stake_task<T: Config>(
		owner_id: AccountId,
		provided_id: Vec<u8>,
		next_execution_time: UnixTime,
		frequency: Seconds,
		collator_id: AccountId,
		account_minimum: Balance,
	) -> Result<Self, DispatchError> {
		let action = Action::AutoCompoundDelegatedStake {
			delegator: owner_id.clone(),
			collator: collator_id,
			account_minimum,
		};
		let schedule = Schedule::<MaxExecutionTimes>::new_recurring_schedule::<T>(
			next_execution_time,
			frequency,
		)?;
		Ok(Self::new(owner_id, provided_id, schedule, action))
	}

	pub fn execution_times(&self, mut l: Vec<UnixTime>) -> Vec<UnixTime> {
		match &self.schedule {
			Schedule::Fixed { execution_times, .. } => execution_times.to_vec(),
			Schedule::Recurring { next_execution_time, .. } => {
				l.push(*next_execution_time);
				l
			},
		}
	}

	pub fn executions_left(&self) -> u32 {
		*&self.schedule.number_of_known_executions()
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
	use crate::{
		mock::*,
		tests::{SCHEDULED_TIME, START_BLOCK_TIME},
	};

	mod scheduled_tasks {
		use super::*;
		use crate::{AccountTaskId, BalanceOf, ScheduledTasksOf, TaskId, TaskOf};
		use frame_support::assert_err;
		use sp_runtime::AccountId32;

		#[test]
		fn try_push_errors_when_slot_is_full_by_weight() {
			new_test_ext(START_BLOCK_TIME).execute_with(|| {
				let task = TaskOf::<Test>::create_event_task::<Test>(
					AccountId32::new(ALICE),
					vec![0],
					vec![SCHEDULED_TIME],
					vec![0],
				)
				.unwrap();
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
			new_test_ext(START_BLOCK_TIME).execute_with(|| {
				let alice = AccountId32::new(ALICE);
				let id = (alice.clone(), TaskId::<Test>::default());
				let task = TaskOf::<Test>::create_event_task::<Test>(
					alice.clone(),
					vec![0],
					vec![SCHEDULED_TIME],
					vec![0],
				)
				.unwrap();
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
			new_test_ext(START_BLOCK_TIME).execute_with(|| {
				let task = TaskOf::<Test>::create_event_task::<Test>(
					AccountId32::new(ALICE),
					vec![0],
					vec![SCHEDULED_TIME],
					vec![0],
				)
				.unwrap();
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

	mod schedule {
		use super::*;
		use frame_support::{assert_err, assert_ok};

		#[test]
		fn partial_eq() {
			new_test_ext(0).execute_with(|| {
				assert_eq!(
					Schedule::<MaxExecutionTimes>::Fixed {
						execution_times: vec![0].try_into().unwrap(),
						executions_left: 1
					},
					Schedule::<MaxExecutionTimes>::Fixed {
						execution_times: vec![0].try_into().unwrap(),
						executions_left: 1
					}
				);
				assert_ne!(
					Schedule::<MaxExecutionTimes>::Fixed {
						execution_times: vec![1].try_into().unwrap(),
						executions_left: 1
					},
					Schedule::<MaxExecutionTimes>::Fixed {
						execution_times: vec![0].try_into().unwrap(),
						executions_left: 1
					}
				);
				assert_ne!(
					Schedule::<MaxExecutionTimes>::Fixed {
						execution_times: vec![0, 1].try_into().unwrap(),
						executions_left: 1
					},
					Schedule::<MaxExecutionTimes>::Fixed {
						execution_times: vec![0, 1].try_into().unwrap(),
						executions_left: 2
					}
				);
				assert_eq!(
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: 0,
						frequency: 3600
					},
					Schedule::Recurring { next_execution_time: 0, frequency: 3600 }
				);
				assert_ne!(
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: 1,
						frequency: 3600
					},
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: 0,
						frequency: 3600
					}
				);
				assert_ne!(
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: 0,
						frequency: 7200
					},
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: 0,
						frequency: 3600
					}
				);
				assert_ne!(
					Schedule::<MaxExecutionTimes>::Fixed {
						execution_times: vec![0].try_into().unwrap(),
						executions_left: 1
					},
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: 0,
						frequency: 3600
					}
				);
			})
		}

		#[test]
		fn new_fixed_schedule_sets_executions_left() {
			new_test_ext(START_BLOCK_TIME).execute_with(|| {
				let t1 = SCHEDULED_TIME + 3600;
				let t2 = SCHEDULED_TIME + 3600 * 2;
				let t3 = SCHEDULED_TIME + 3600 * 3;
				let s = Schedule::<MaxExecutionTimes>::new_fixed_schedule::<Test>(vec![t1, t2, t3])
					.unwrap();
				if let Schedule::Fixed { executions_left, .. } = s {
					assert_eq!(executions_left, 3);
				} else {
					panic!("Exepected Schedule::Fixed");
				}
			})
		}

		#[test]
		fn new_fixed_schedule_errors_with_too_many_executions() {
			new_test_ext(0).execute_with(|| {
				let s = Schedule::<MaxExecutionTimes>::new_fixed_schedule::<Test>(
					(0u64..=MaxExecutionTimes::get() as u64).collect(),
				);
				assert_err!(s, Error::<Test>::TooManyExecutionsTimes);
			})
		}

		#[test]
		fn new_fixed_schedule_cleans_execution_times() {
			new_test_ext(START_BLOCK_TIME).execute_with(|| {
				let t1 = SCHEDULED_TIME + 3600;
				let t2 = SCHEDULED_TIME + 3600 * 2;
				let t3 = SCHEDULED_TIME + 3600 * 3;
				let s = Schedule::<MaxExecutionTimes>::new_fixed_schedule::<Test>(vec![
					t1, t3, t2, t3, t3,
				]);
				if let Schedule::Fixed { execution_times, .. } = s.unwrap() {
					assert_eq!(execution_times, vec![t1, t2, t3]);
				} else {
					panic!("Exepected Schedule::Fixed");
				}
			})
		}

		#[test]
		fn checks_for_fixed_schedule_validity() {
			new_test_ext(START_BLOCK_TIME).execute_with(|| {
				assert_ok!(Schedule::<MaxExecutionTimes>::new_fixed_schedule::<Test>(vec![
					SCHEDULED_TIME + 3600
				]));
				assert_err!(
					Schedule::<MaxExecutionTimes>::new_fixed_schedule::<Test>(vec![
						SCHEDULED_TIME + 3600,
						SCHEDULED_TIME + 3650
					]),
					Error::<Test>::InvalidTime
				);
			})
		}

		#[test]
		fn checks_for_recurring_schedule_validity() {
			let start_time = 1_663_225_200;
			new_test_ext(start_time * 1_000).execute_with(|| {
				assert_ok!(Schedule::<MaxExecutionTimes>::Recurring {
					next_execution_time: start_time + 3600,
					frequency: 3600
				}
				.valid::<Test>());
				assert_err!(
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: start_time + 3650,
						frequency: 3600
					}
					.valid::<Test>(),
					Error::<Test>::InvalidTime
				);
				assert_err!(
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: start_time + 3600,
						frequency: 0
					}
					.valid::<Test>(),
					Error::<Test>::InvalidTime
				);
				assert_err!(
					Schedule::<MaxExecutionTimes>::Recurring {
						next_execution_time: start_time + 3600,
						frequency: start_time + 3600
					}
					.valid::<Test>(),
					Error::<Test>::TimeTooFarOut
				);
			})
		}

		#[test]
		fn number_of_known_executions_for_fixed() {
			new_test_ext(0).execute_with(|| {
				let s = Schedule::<MaxExecutionTimes>::Fixed {
					execution_times: vec![].try_into().unwrap(),
					executions_left: 5,
				};
				assert_eq!(s.number_of_known_executions(), 5);
			})
		}

		#[test]
		fn number_of_known_executions_for_recurring() {
			new_test_ext(0).execute_with(|| {
				let s = Schedule::<MaxExecutionTimes>::Recurring {
					next_execution_time: 0,
					frequency: 0,
				};
				assert_eq!(s.number_of_known_executions(), 1);
			})
		}
	}
}
