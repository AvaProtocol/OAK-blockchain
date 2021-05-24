//! Benchmarking setup for pallet-template

use super::*;

use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, whitelisted_caller, impl_benchmark_test_suite};
#[allow(unused)]
use crate::Pallet as Template;

benchmarks! {
	fund {
		let s in 1 .. 100;
		let caller: T::AccountId = whitelisted_caller();
        // let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
	}: _(RawOrigin::Signed(caller), s.into())
}

impl_benchmark_test_suite!(
	OpenGrant,
	crate::mock::new_test_ext(),
	crate::mock::Test,
);
