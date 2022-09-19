use frame_support::{pallet_prelude::PhantomData, traits::Get};
use sp_runtime::{traits::CheckedConversion, ArithmeticError, DispatchError};

use crate::{Config, Error, UnixTime};

pub struct Base<T>(PhantomData<T>);
impl<T: Config> Base<T> {
	/// Based on the block time, return the time slot.
	///
	/// In order to do this we:
	/// * Get the most recent timestamp from the block.
	/// * Convert the ms unix timestamp to seconds.
	/// * Bring the timestamp down to the last whole hour.
	pub fn get_current_time_slot() -> Result<UnixTime, DispatchError> {
		let now = <pallet_timestamp::Pallet<T>>::get()
			.checked_into::<UnixTime>()
			.ok_or(ArithmeticError::Overflow)?;

		if now == 0 {
			Err(Error::<T>::BlockTimeNotSet)?
		}

		let now = now.checked_div(1000).ok_or(ArithmeticError::Overflow)?;
		let diff_to_hour = now.checked_rem(3600).ok_or(ArithmeticError::Overflow)?;
		Ok(now.checked_sub(diff_to_hour).ok_or(ArithmeticError::Overflow)?)
	}

	/// Checks to see if the scheduled time is valid.
	///
	/// In order for a time to be valid it must
	/// - End in a whole hour
	/// - Be in the future
	/// - Not be more than MaxScheduleSeconds out
	pub fn is_valid_time(scheduled_time: UnixTime) -> Result<(), DispatchError> {
		#[cfg(feature = "dev-queue")]
		if scheduled_time == 0 {
			return Ok(())
		}

		let remainder = scheduled_time.checked_rem(3600).ok_or(ArithmeticError::Overflow)?;
		if remainder != 0 {
			Err(<Error<T>>::InvalidTime)?;
		}

		let current_time_slot = Base::<T>::get_current_time_slot()?;
		if scheduled_time <= current_time_slot {
			Err(<Error<T>>::PastTime)?;
		}

		let max_schedule_time = current_time_slot
			.checked_add(T::MaxScheduleSeconds::get())
			.ok_or(ArithmeticError::Overflow)?;

		if scheduled_time > max_schedule_time {
			Err(Error::<T>::TimeTooFarOut)?;
		}

		Ok(())
	}
}

pub struct Utils;
impl Utils {
	/// Cleans the executions times by removing duplicates and putting in ascending order.
	pub fn clean_execution_times_vector(execution_times: &mut Vec<UnixTime>) {
		execution_times.sort_unstable();
		execution_times.dedup();
	}
}
