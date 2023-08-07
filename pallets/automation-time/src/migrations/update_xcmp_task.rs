use core::marker::PhantomData;

use crate::{
	weights::WeightInfo, AccountOf, ActionOf, AssetPayment, Config, InstructionSequence, TaskOf,
};

use frame_support::{
	traits::{Get, OnRuntimeUpgrade},
	weights::{RuntimeDbWeight, Weight},
	Twox64Concat,
};
use sp_runtime::traits::Convert;
use sp_std::{boxed::Box, vec, vec::Vec};
use xcm::latest::prelude::*;

use crate::migrations::utils::{
	deprecate::{generate_old_task_id, old_taskid_to_idv2},
	OldAccountTaskId, OldAction, OldTask, OldTaskId, TEST_TASKID1,
};

use frame_support::{dispatch::GetDispatchInfo, pallet_prelude::DispatchError};

//#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
use scale_info::TypeInfo;

const EXECUTION_FEE_AMOUNT: u128 = 4_000_000_000;
const INSTRUCTION_WEIGHT_REF_TIME: u64 = 150_000_000;

impl<T: Config> From<OldAction<T>> for ActionOf<T> {
	fn from(action: OldAction<T>) -> Self {
		use codec::{Decode, Encode};
		use primitives::TransferCallCreator;
		match action {
			OldAction::AutoCompoundDelegatedStake { delegator, collator, account_minimum } =>
				Self::AutoCompoundDelegatedStake { delegator, collator, account_minimum },
			OldAction::Notify { message } => {
				let call: <T as frame_system::Config>::RuntimeCall =
					frame_system::Call::<T>::remark_with_event { remark: message }.into();
				Self::DynamicDispatch { encoded_call: call.encode() }
			},
			OldAction::NativeTransfer { recipient, amount, .. } => {
				let call: <T as frame_system::Config>::RuntimeCall =
					T::TransferCallCreator::create_transfer_call(
						sp_runtime::MultiAddress::Id(recipient),
						amount,
					);
				Self::DynamicDispatch { encoded_call: call.encode() }
			},
			OldAction::XCMP {
				para_id,
				currency_id,
				encoded_call,
				encoded_call_weight,
				schedule_as,
				..
			} => {
				let schedule_fee =
					T::CurrencyIdConvert::convert(currency_id).expect("IncoveribleCurrencyId");
				Self::XCMP {
					destination: MultiLocation::new(1, X1(Parachain(para_id.into()))),
					schedule_fee,
					execution_fee: AssetPayment {
						asset_location: MultiLocation::new(0, Here).into(),
						amount: EXECUTION_FEE_AMOUNT,
					},
					encoded_call,
					encoded_call_weight: encoded_call_weight.clone(),
					overall_weight: encoded_call_weight.saturating_add(
						Weight::from_ref_time(INSTRUCTION_WEIGHT_REF_TIME).saturating_mul(6),
					),
					schedule_as,
					instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
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
	OldTaskId<T>,
	OldTask<T>,
>;

pub struct UpdateXcmpTask<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for UpdateXcmpTask<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "automation-time", "UpdateXcmpTask migration");

		let mut migrated_tasks = 0u64;

		let mut tasks: Vec<(AccountOf<T>, TaskOf<T>)> = vec![];
		AccountTasks::<T>::drain().for_each(|(account_id, _task_id, task)| {
			let migrated_task: TaskOf<T> = task.into();
			tasks.push((account_id, migrated_task));
			migrated_tasks += 1;
		});

		tasks.iter().for_each(|(account_id, task)| {
			crate::AccountTasks::<T>::insert(account_id, task.task_id.clone(), task)
		});

		log::info!(
			target: "automation-time",
			"migration: UpdateXcmpTask succesful! Migrated {} tasks.",
			migrated_tasks
		);

		let db_weight: RuntimeDbWeight = T::DbWeight::get();
		db_weight.reads_writes(migrated_tasks + 1, migrated_tasks + 1)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		let prev_count = crate::AccountTasks::<T>::iter().count() as u32;
		Ok(prev_count.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(prev_count: Vec<u8>) -> Result<(), &'static str> {
		let prev_count: u32 = Decode::decode(&mut prev_count.as_slice())
			.expect("the state parameter should be something that was generated by pre_upgrade");
		let post_count = crate::AccountTasks::<T>::iter().count() as u32;
		assert!(post_count == prev_count);

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{mock::*, ActionOf, AssetPayment, InstructionSequence, Schedule, TaskOf};
	use cumulus_primitives_core::ParaId;
	use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
	use sp_runtime::AccountId32;
	use xcm::latest::prelude::*;

	#[test]
	fn on_runtime_upgrade() {
		new_test_ext(0).execute_with(|| {
			let para_id: ParaId = 1000.into();
			let account_id = AccountId32::new(ALICE);
			let schedule_as = Some(AccountId32::new(BOB));
			let encoded_call_weight = Weight::from_ref_time(10);

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
					xcm_asset_location: MultiLocation::new(1, X1(Parachain(para_id.into()))).into(),
					encoded_call: vec![0u8],
					encoded_call_weight: encoded_call_weight.clone(),
					schedule_as: schedule_as.clone(),
				},
			};

			let old_task_id =
				generate_old_task_id::<Test>(account_id.clone(), task.provided_id.clone());
			super::AccountTasks::<Test>::insert(account_id.clone(), old_task_id.clone(), task);

			UpdateXcmpTask::<Test>::on_runtime_upgrade();

			assert_eq!(crate::AccountTasks::<Test>::iter().count(), 1);

			let task_id1 = TEST_TASKID1.as_bytes().to_vec();
			assert_eq!(
				crate::AccountTasks::<Test>::get(account_id.clone(), task_id1.clone()).unwrap(),
				TaskOf::<Test> {
					owner_id: account_id.clone(),
					task_id: task_id1,
					schedule: Schedule::Fixed {
						execution_times: vec![0, 1].try_into().unwrap(),
						executions_left: 2
					},
					action: ActionOf::<Test>::XCMP {
						destination: MultiLocation::new(1, X1(Parachain(para_id.into()))),
						schedule_fee: MultiLocation::default(),
						execution_fee: AssetPayment {
							asset_location: MultiLocation::new(0, Here).into(),
							amount: EXECUTION_FEE_AMOUNT,
						},
						encoded_call: vec![0u8],
						encoded_call_weight: encoded_call_weight.clone(),
						overall_weight: encoded_call_weight.saturating_add(
							Weight::from_ref_time(INSTRUCTION_WEIGHT_REF_TIME).saturating_mul(6)
						),
						schedule_as,
						instruction_sequence: InstructionSequence::PayThroughSovereignAccount,
					},
					abort_errors: vec![],
				}
			);
		})
	}

	#[test]
	fn on_runtime_upgrade_notify() {
		new_test_ext(0).execute_with(|| {
			let para_id: ParaId = 1000.into();
			let account_id = AccountId32::new(ALICE);
			let encoded_call_weight = Weight::from_ref_time(10);

			let task = OldTask::<Test> {
				owner_id: account_id.clone(),
				provided_id: vec![1],
				schedule: Schedule::Fixed {
					execution_times: vec![0, 1].try_into().unwrap(),
					executions_left: 2,
				},
				action: OldAction::<Test>::Notify { message: vec![1, 2, 3] },
			};

			let old_task_id =
				generate_old_task_id::<Test>(account_id.clone(), task.provided_id.clone());
			super::AccountTasks::<Test>::insert(account_id.clone(), old_task_id.clone(), task);

			UpdateXcmpTask::<Test>::on_runtime_upgrade();

			assert_eq!(crate::AccountTasks::<Test>::iter().count(), 1);

			let task_id1 = TEST_TASKID1.as_bytes().to_vec();
			assert_eq!(
				crate::AccountTasks::<Test>::get(account_id.clone(), task_id1.clone()).unwrap(),
				TaskOf::<Test> {
					owner_id: account_id.clone(),
					task_id: task_id1,
					schedule: Schedule::Fixed {
						execution_times: vec![0, 1].try_into().unwrap(),
						executions_left: 2
					},
					action: ActionOf::<Test>::DynamicDispatch { encoded_call: vec![0, 7, 12, 1, 2, 3] },
					abort_errors: vec![],
				}
			);
		})
	}

	#[test]
	fn on_runtime_upgrade_native_transfer() {
		new_test_ext(0).execute_with(|| {
			let para_id: ParaId = 1000.into();
			let account_id = AccountId32::new(ALICE);
			let recipient = AccountId32::new(BOB);
			let encoded_call_weight = Weight::from_ref_time(10);

			let task = OldTask::<Test> {
				owner_id: account_id.clone(),
				provided_id: vec![1],
				schedule: Schedule::Fixed {
					execution_times: vec![0, 1].try_into().unwrap(),
					executions_left: 2,
				},
				action: OldAction::<Test>::NativeTransfer {
					sender: account_id.clone(),
					recipient,
					amount: 100,
				},
			};

			let old_task_id =
				generate_old_task_id::<Test>(account_id.clone(), task.provided_id.clone());
			super::AccountTasks::<Test>::insert(account_id.clone(), old_task_id.clone(), task);

			UpdateXcmpTask::<Test>::on_runtime_upgrade();

			assert_eq!(crate::AccountTasks::<Test>::iter().count(), 1);

			let task_id1 = TEST_TASKID1.as_bytes().to_vec();
			assert_eq!(
				crate::AccountTasks::<Test>::get(account_id.clone(), task_id1.clone()).unwrap(),
				TaskOf::<Test> {
					owner_id: account_id.clone(),
					task_id: task_id1,
					schedule: Schedule::Fixed {
						execution_times: vec![0, 1].try_into().unwrap(),
						executions_left: 2
					},
					action: ActionOf::<Test>::DynamicDispatch {
						encoded_call: vec![
                            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 145, 1
						],
					},
					abort_errors: vec![],
				}
			);
		})
	}
}
