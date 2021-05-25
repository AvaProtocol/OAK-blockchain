//! Benchmarking setup for pallet-template

use super::*;

use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, whitelisted_caller, impl_benchmark_test_suite};
#[allow(unused)]
use crate::Pallet as OpenGrant;
use sp_runtime::traits::Bounded;

benchmarks! {
	fund {
		let s in 1 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
        let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
	}: _(RawOrigin::Signed(caller), s.into())

	create_project {
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])
}

impl_benchmark_test_suite!(
	OpenGrant,
	crate::mock::new_test_ext(),
	crate::mock::Test,
);
