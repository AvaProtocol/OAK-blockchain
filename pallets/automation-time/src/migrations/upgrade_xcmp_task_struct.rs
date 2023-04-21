use core::marker::PhantomData;

use crate::{
	weights::WeightInfo, AccountOf, ActionOf, BalanceOf, Config, Schedule, TaskId, TaskOf,
};
use codec::{Decode, Encode};
use cumulus_primitives_core::ParaId;
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight, Twox64Concat};
use scale_info::TypeInfo;
use sp_std::vec::Vec;
use xcm::latest::prelude::*;

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
		TaskOf::<T> {
			owner_id: task.owner_id,
			provided_id: task.provided_id,
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
		encoded_call: Vec<u8>,
		encoded_call_weight: u64,
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

impl<T: Config> From<OldAction<T>> for ActionOf<T> {
	fn from(action: OldAction<T>) -> Self {
		match action {
			OldAction::AutoCompoundDelegatedStake { delegator, collator, account_minimum } =>
				Self::AutoCompoundDelegatedStake { delegator, collator, account_minimum },
			OldAction::Notify { message } => Self::Notify { message },
			OldAction::NativeTransfer { sender, recipient, amount } =>
				Self::NativeTransfer { sender, recipient, amount },
			OldAction::XCMP { para_id, currency_id, encoded_call, encoded_call_weight } => {
				let xcm_asset_location =
					MultiLocation::new(1, X1(Parachain(para_id.into()))).into();
				Self::XCMP {
					para_id,
					currency_id,
					xcm_asset_location,
					encoded_call,
					encoded_call_weight,
					schedule_as: None,
				}
			},
			OldAction::DynamicDispatch { encoded_call } => Self::DynamicDispatch { encoded_call },
		}
	}
}

#[frame_support::storage_alias]
pub type AccountTasks<T: Config> = StorageDoubleMap<
	AutomationTime,
	Twox64Concat,
	AccountOf<T>,
	Twox64Concat,
	TaskId<T>,
	OldTask<T>,
>;

pub struct UpgradeXcmpTaskStruct<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for UpgradeXcmpTaskStruct<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "automation-time", "UpgradeXcmpTaskStruct migration");

		let mut migrated_tasks = 0u32;
		AccountTasks::<T>::iter().for_each(|(account_id, task_id, task)| {
			let migrated_task: TaskOf<T> = task.into();
			crate::AccountTasks::<T>::insert(account_id, task_id, migrated_task);

			migrated_tasks += 1;
		});

		log::info!(
			target: "automation-time",
			"migration: UpgradeXcmpTaskStruct succesful! Migrated {} tasks.",
			migrated_tasks
		);

		<T as Config>::WeightInfo::migration_upgrade_xcmp_task_struct(migrated_tasks)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::OnRuntimeUpgradeHelpersExt;

		let task_count = AccountTasks::<T>::iter().count();
		Self::set_temp_storage::<u32>(task_count as u32, "pre_migration_task_count");

		log::info!(
			target: "automation-time",
			"migration: UpgradeXcmpTaskStruct PRE migration checks succesful!"
		);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::OnRuntimeUpgradeHelpersExt;

		let post_task_count = crate::AccountTasks::<T>::iter().count() as u32;
		let pre_task_count = Self::get_temp_storage::<u32>("pre_migration_task_count").unwrap();
		assert_eq!(post_task_count, pre_task_count);

		log::info!(
			target: "automation-time",
			"migration: UpgradeXcmpTaskStruct POST migration checks succesful! Migrated {} tasks.",
			post_task_count
		);

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::{OldAction, OldTask, UpgradeXcmpTaskStruct};
	use crate::{mock::*, ActionOf, Pallet, ParaId, Schedule, TaskOf};
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_runtime::AccountId32;
	use xcm::latest::prelude::*;

	#[test]
	fn on_runtime_upgrade() {
		new_test_ext(0).execute_with(|| {
			let para_id: ParaId = 1000.into();
			let account_id = AccountId32::new(ALICE);
			let task_id = Pallet::<Test>::generate_task_id(account_id.clone(), vec![1]);

			let task = OldTask::<Test> {
				owner_id: account_id.clone(),
				provided_id: vec![1],
				schedule: Schedule::Fixed {
					execution_times: vec![0, 1].try_into().unwrap(),
					executions_left: 2,
				},
				action: OldAction::<Test>::XCMP {
					para_id,
					currency_id: 0u32.into(),
					encoded_call: vec![0u8],
					encoded_call_weight: 10,
				},
			};

			super::AccountTasks::<Test>::insert(account_id.clone(), task_id, task);

			UpgradeXcmpTaskStruct::<Test>::on_runtime_upgrade();

			assert_eq!(crate::AccountTasks::<Test>::iter().count(), 1);
			assert_eq!(
				crate::AccountTasks::<Test>::get(account_id.clone(), task_id).unwrap(),
				TaskOf::<Test> {
					owner_id: account_id.clone(),
					provided_id: vec![1],
					schedule: Schedule::Fixed {
						execution_times: vec![0, 1].try_into().unwrap(),
						executions_left: 2
					},
					action: ActionOf::<Test>::XCMP {
						para_id,
						currency_id: 0u32.into(),
						xcm_asset_location: MultiLocation::new(1, X1(Parachain(para_id.into())))
							.into(),
						encoded_call: vec![0u8],
						encoded_call_weight: 10,
						schedule_as: None,
					},
				}
			);
		})
	}
}
