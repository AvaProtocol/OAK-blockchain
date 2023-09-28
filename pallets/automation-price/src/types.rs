use crate::{weights::WeightInfo, Config, InstructionSequence};

use frame_support::pallet_prelude::*;

use sp_std::prelude::*;

use xcm::{latest::prelude::*, VersionedMultiLocation};

/// The struct that stores execution payment for a task.
#[derive(Debug, Encode, Eq, PartialEq, Decode, TypeInfo, Clone)]
pub struct AssetPayment {
	pub asset_location: VersionedMultiLocation,
	pub amount: u128,
}

/// The enum that stores all action specific data.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum Action<AccountId> {
	XCMP {
		destination: MultiLocation,
		schedule_fee: MultiLocation,
		execution_fee: AssetPayment,
		encoded_call: Vec<u8>,
		encoded_call_weight: Weight,
		overall_weight: Weight,
		schedule_as: Option<AccountId>,
		instruction_sequence: InstructionSequence,
	},
}

impl<AccountId> Action<AccountId> {
	pub fn execution_weight<T: Config>(&self) -> Result<u64, DispatchError> {
		let weight = match self {
			Action::XCMP { .. } => <T as Config>::WeightInfo::run_xcmp_task(),
		};
		Ok(weight.ref_time())
	}

	pub fn schedule_fee_location<T: Config>(&self) -> MultiLocation {
		match self {
			Action::XCMP { schedule_fee, .. } => *schedule_fee,
			_ => MultiLocation::default(),
		}
	}
}
