use crate::{Config, PriceData, Task};

use sp_std::ops::{
	Bound,
	Bound::{Excluded, Included},
};

pub const TRIGGER_FUNC_GT: &[u8] = "gt".as_bytes();
pub const TRIGGER_FUNC_LT: &[u8] = "lt".as_bytes();

pub trait PriceConditionMatch {
	fn is_price_condition_match(&self, price: &PriceData) -> bool;
}

impl<T: Config> PriceConditionMatch for Task<T> {
	/// check that the task has its condition match the target price of asset
	///
	/// # Argument
	///
	/// * `price` - the desire price of the asset to check on
	fn is_price_condition_match(&self, price: &PriceData) -> bool {
		// trigger when target price > current price of the asset
		// Example:
		//  - current price: 100, the task is has target price: 50  -> runable
		//  - current price: 100, the task is has target price: 150 -> not runable
		//

		if self.trigger_function == TRIGGER_FUNC_GT.to_vec() {
			price.value > self.trigger_params[0]
		} else {
			price.value < self.trigger_params[0]
		}
	}
}

/// Given a condition, and a target price, generate a range that match the condition
pub fn range_by_trigger_func(
	trigger_func: &[u8],
	current_price: &PriceData,
) -> (Bound<u128>, Bound<u128>) {
	//Eg sell order, sell when price >
	if trigger_func == TRIGGER_FUNC_GT {
		(Excluded(u128::MIN), Excluded(current_price.value))
	} else {
		// Eg buy order, buy when price < target
		(Included(current_price.value), Excluded(u128::MAX))
	}
}
