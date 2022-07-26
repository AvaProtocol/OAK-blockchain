use crate::{
	pallet::{AccountOf, BalanceOf, Config, Error, Event, Pallet, Seconds, UnixTime},
	weights::WeightInfo,
	Decode, Encode, Junction, ParaId, Task, Transact, TypeInfo, Xcm,
};
use pallet_automation_time_rpc_runtime_api::AutomationAction;

use frame_support::{
	dispatch::DispatchErrorWithPostInfo,
	traits::{Currency, ExistenceRequirement, Get},
	weights::Weight,
};
use sp_runtime::traits::{CheckedSub, SaturatedConversion, Saturating};
use sp_std::{vec, vec::Vec};
use xcm::{latest::prelude::OriginKind, v2::SendXcm};

use pallet_parachain_staking::DelegatorActions;

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
		call: Vec<u8>,
		weight_at_most: Weight,
	},
	AutoCompoundDelegatedStake {
		delegator: AccountOf<T>,
		collator: AccountOf<T>,
		account_minimum: BalanceOf<T>,
		frequency: Seconds,
	},
}

impl<T: Config> Action<T> {
	pub fn execute(&self, task_id: T::Hash, mut task: Task<T>) -> (Weight, Task<T>) {
		let weight = match self.clone() {
			Action::Notify { message } => Self::run_notify_task(message),
			Action::NativeTransfer { sender, recipient, amount } =>
				Self::run_native_transfer_task(sender, recipient, amount, task_id),
			Action::XCMP { para_id, call, weight_at_most } =>
				Self::run_xcmp_task(para_id, call, weight_at_most, task_id),
			Action::AutoCompoundDelegatedStake {
				delegator,
				collator,
				account_minimum,
				frequency,
			} => {
				let (weight, mut_task) = Self::run_auto_compound_delegated_stake_task(
					delegator,
					collator,
					account_minimum,
					frequency,
					task_id,
					task,
				);
				task = mut_task;
				weight
			},
		};
		(weight, task)
	}

	/// Fire the notify event with the custom message.
	pub fn run_notify_task(message: Vec<u8>) -> Weight {
		Pallet::<T>::deposit_event(Event::Notify { message });
		<T as Config>::WeightInfo::run_notify_task()
	}

	pub fn run_native_transfer_task(
		sender: T::AccountId,
		recipient: T::AccountId,
		amount: BalanceOf<T>,
		task_id: T::Hash,
	) -> Weight {
		match T::Currency::transfer(&sender, &recipient, amount, ExistenceRequirement::KeepAlive) {
			Ok(_number) =>
				Pallet::<T>::deposit_event(Event::SuccessfullyTransferredFunds { task_id }),
			Err(e) => Pallet::<T>::deposit_event(Event::TransferFailed { task_id, error: e }),
		};

		<T as Config>::WeightInfo::run_native_transfer_task()
	}

	pub fn run_xcmp_task(
		para_id: ParaId,
		call: Vec<u8>,
		weight_at_most: Weight,
		task_id: T::Hash,
	) -> Weight {
		let destination = (1, Junction::Parachain(para_id.into()));
		let message = Xcm(vec![Transact {
			origin_type: OriginKind::Native,
			require_weight_at_most: weight_at_most,
			call: call.into(),
		}]);
		match T::XcmSender::send_xcm(destination, message) {
			Ok(()) => {
				Pallet::<T>::deposit_event(Event::SuccessfullySentXCMP { task_id, para_id });
			},
			Err(e) => {
				Pallet::<T>::deposit_event(Event::FailedToSendXCMP { task_id, para_id, error: e });
			},
		}
		// Adding 1 DB write that doesn't get accounted for in the benchmarks to run an xcmp task
		T::DbWeight::get()
			.writes(1)
			.saturating_add(<T as Config>::WeightInfo::run_xcmp_task())
	}

	/// Executes auto compounding delegation and reschedules task on success
	pub fn run_auto_compound_delegated_stake_task(
		delegator: T::AccountId,
		collator: T::AccountId,
		account_minimum: BalanceOf<T>,
		frequency: Seconds,
		task_id: T::Hash,
		mut task: Task<T>,
	) -> (Weight, Task<T>) {
		match Self::compound_delegator_stake(
			delegator.clone(),
			collator.clone(),
			account_minimum,
			task.action.calculate_execution_fee(1),
		) {
			Ok(delegation) =>
				Pallet::<T>::deposit_event(Event::SuccesfullyAutoCompoundedDelegatorStake {
					task_id,
					amount: delegation,
				}),
			Err(e) => {
				Pallet::<T>::deposit_event(Event::AutoCompoundDelegatorStakeFailed {
					task_id,
					error_message: Into::<&str>::into(e).as_bytes().to_vec(),
					error: e,
				});
				return (
					// TODO: benchmark and return a smaller weight here to account for the early exit
					<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
					task,
				)
			},
		}

		let new_execution_times: Vec<UnixTime> =
			task.execution_times.iter().map(|when| when.saturating_add(frequency)).collect();
		let _ = task.reschedule(task_id, new_execution_times).map_err(|e| {
			let err: DispatchErrorWithPostInfo = e.into();
			Pallet::<T>::deposit_event(Event::AutoCompoundDelegatorStakeFailed {
				task_id,
				error_message: Into::<&str>::into(err).as_bytes().to_vec(),
				error: err,
			});
		});

		(<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(), task)
	}

	fn compound_delegator_stake(
		delegator: T::AccountId,
		collator: T::AccountId,
		account_minimum: BalanceOf<T>,
		execution_fee: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchErrorWithPostInfo> {
		// TODO: Handle edge case where user has enough funds to run task but not reschedule
		let reserved_funds = account_minimum.saturating_add(execution_fee);
		T::Currency::free_balance(&delegator)
			.checked_sub(&reserved_funds)
			.ok_or(Error::<T>::InsufficientBalance.into())
			.and_then(|delegation| {
				T::DelegatorActions::delegator_bond_more(&delegator, &collator, delegation)
					.and(Ok(delegation))
			})
	}

	/// Calculates the execution fee for a given action based on weight and num of executions
	///
	/// Fee saturates at Weight/BalanceOf when there are an unreasonable num of executions
	/// In practice, executions is bounded by T::MaxExecutionTimes and unlikely to saturate
	pub fn calculate_execution_fee(&self, executions: u32) -> BalanceOf<T> {
		let action_weight = match self {
			Action::Notify { .. } => <T as Config>::WeightInfo::run_notify_task(),
			Action::NativeTransfer { .. } => <T as Config>::WeightInfo::run_native_transfer_task(),
			// Adding 1 DB write that doesn't get accounted for in the benchmarks to run an xcmp task
			Action::XCMP { .. } => T::DbWeight::get()
				.writes(1)
				.saturating_add(<T as Config>::WeightInfo::run_xcmp_task()),
			Action::AutoCompoundDelegatedStake { .. } =>
				<T as Config>::WeightInfo::run_auto_compound_delegated_stake_task(),
		};

		let total_weight = action_weight.saturating_mul(executions.into());
		let weight_as_balance = <BalanceOf<T>>::saturated_from(total_weight);

		T::ExecutionWeightFee::get().saturating_mul(weight_as_balance)
	}
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
				call: vec![0],
				weight_at_most: 0,
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
