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

	schedule_round {
		let s in 1 .. 60;
		let caller: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller));
		OpenGrant::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		let mut project_indexes: Vec<u32> = Vec::new();
		for i in 0 .. s {
			OpenGrant::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
			project_indexes.push(i);
		}
	}: _(RawOrigin::Root, 100u32.into(), 200u32.into(), 10u32.into(), project_indexes)

	cancel_round {
		let caller: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller));
		OpenGrant::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		OpenGrant::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
		OpenGrant::<T>::schedule_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 100u32.into(), 200u32.into(), 10u32.into(), vec![0])?;
	}: _(RawOrigin::Root, 0)

	cancel {
		let caller: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller));
		OpenGrant::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		OpenGrant::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
		OpenGrant::<T>::schedule_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 100u32.into(), 200u32.into(), 10u32.into(), vec![0])?;
	}: _(RawOrigin::Root, 0, 0)

	set_withdrawal_expiration {
		let s in 1 .. 100000000;
	}: _(RawOrigin::Root, s.into())

	set_max_grant_count_per_round {
		let s in 1 .. 200;
	}: _(RawOrigin::Root, s)

	set_is_identity_required {}: _(RawOrigin::Root, true)
	
}

impl_benchmark_test_suite!(
	OpenGrant,
	crate::mock::new_test_ext(),
	crate::mock::Test,
);
