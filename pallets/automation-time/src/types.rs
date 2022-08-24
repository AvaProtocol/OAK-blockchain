use crate::{
	AutomationAction, Config, Currency, Decode, Encode, Error, Get, ParaId, Task, TypeInfo, Weight,
	WeightInfo,
};

pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type UnixTime = u64;
pub type Seconds = u64;
pub type TaskId<T> = <T as frame_system::Config>::Hash;
pub type AccountTaskId<T> = (<T as frame_system::Config>::AccountId, TaskId<T>);

/// The enum that stores all action specific data.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum Action<T: Config> {
	Notify {
		message: Vec<u8>,
	},
	NativeTransfer {
		sender: AccountOf<T>,
		recipient: AccountOf<T>,
		amount: BalanceOf<T>,
	},
	XCMP {
		para_id: ParaId,
		currency_id: T::CurrencyId,
		encoded_call: Vec<u8>,
		encoded_call_weight: Weight,
	},
	AutoCompoundDelegatedStake {
		delegator: AccountOf<T>,
		collator: AccountOf<T>,
		account_minimum: BalanceOf<T>,
		frequency: Seconds,
	},
}
impl<T: Config> From<AutomationAction> for Action<T> {
	fn from(a: AutomationAction) -> Self {
		let default_account =
			T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
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
				currency_id: T::GetNativeCurrencyId::get(),
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
impl<T: Config> Action<T> {
	pub fn execution_weight(&self) -> Weight {
		match self {
			Action::Notify { .. } => <T as Config>::WeightInfo::run_notify_task(),
			Action::NativeTransfer { .. } => <T as Config>::WeightInfo::run_native_transfer_task(),
			// Adding 1 DB write that doesn't get accounted for in the benchmarks to run an xcmp task
			Action::XCMP { .. } => T::DbWeight::get()
				.writes(1)
				.saturating_add(<T as Config>::WeightInfo::run_xcmp_task()),
			Action::AutoCompoundDelegatedStake { .. } =>
				<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
		}
	}
}

#[derive(Debug, Decode, Eq, Encode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ScheduledTasks<T: Config> {
	pub tasks: Vec<AccountTaskId<T>>,
	pub weight: Weight,
}
impl<T: Config> Default for ScheduledTasks<T> {
	fn default() -> Self {
		ScheduledTasks { tasks: vec![], weight: 0 }
	}
}
impl<T: Config> ScheduledTasks<T> {
	pub fn try_push(&mut self, task_id: TaskId<T>, task: &Task<T>) -> Result<&mut Self, Error<T>> {
		let weight = self
			.weight
			.checked_add(task.action.execution_weight())
			.ok_or(Error::<T>::TimeSlotFull)?;
		if weight > T::MaxWeightPerSlot::get() {
			Err(Error::<T>::TimeSlotFull)?
		}

		self.weight = weight;
		self.tasks.push((task.owner_id.clone(), task_id));
		Ok(self)
	}
}
