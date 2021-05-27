//! Benchmarking setup for pallet-template

use super::*;

use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, whitelisted_caller, impl_benchmark_test_suite};
#[allow(unused)]
use crate::Pallet as QuadraticFunding;
use sp_runtime::traits::Bounded;

benchmarks! {
	fund {
		let caller: T::AccountId = whitelisted_caller();
        let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
	}: _(RawOrigin::Signed(caller), BalanceOf::<T>::max_value())

	create_project {
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])

	schedule_round {
		let s in 1 .. 60;
		let caller: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller));
		QuadraticFunding::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		let mut project_indexes: Vec<u32> = Vec::new();
		for i in 0 .. s {
			QuadraticFunding::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
			project_indexes.push(i);
		}
	}: _(RawOrigin::Root, 100u32.into(), 200u32.into(), 10u32.into(), project_indexes)

	cancel_round {
		let caller: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller));
		QuadraticFunding::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		QuadraticFunding::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
		QuadraticFunding::<T>::schedule_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 100u32.into(), 200u32.into(), 10u32.into(), vec![0])?;
	}: _(RawOrigin::Root, 0)

	cancel {
		let caller: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller));
		QuadraticFunding::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		QuadraticFunding::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
		QuadraticFunding::<T>::schedule_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 100u32.into(), 200u32.into(), 10u32.into(), vec![0])?;
	}: _(RawOrigin::Root, 0, 0)

	set_withdrawal_expiration {}: _(RawOrigin::Root, T::MaxWithdrawalExpiration::get())

	set_max_grant_count_per_round {
		let s in 1 .. 200;
	}: _(RawOrigin::Root, s)

	set_is_identity_required {}: _(RawOrigin::Root, true)
	
	contribute {
		let caller2: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller2, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller2.clone()));
		QuadraticFunding::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		QuadraticFunding::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
		QuadraticFunding::<T>::schedule_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 100u32.into(), 200u32.into(), 10u32.into(), vec![0])?;
		frame_system::Pallet::<T>::set_block_number(150u32.into());
	}: _(RawOrigin::Signed(caller2), 0, 100u32.into())

	finalize_round {
		let caller2: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller2, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller2.clone()));
		QuadraticFunding::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		QuadraticFunding::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
		QuadraticFunding::<T>::schedule_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 100u32.into(), 200u32.into(), 10u32.into(), vec![0])?;
		frame_system::Pallet::<T>::set_block_number(150u32.into());
		QuadraticFunding::<T>::contribute(caller_origin.clone(), 0, 100u32.into())?;
		frame_system::Pallet::<T>::set_block_number(210u32.into());
	}: _(RawOrigin::Root, 0)

	approve {
		let caller2: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller2, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller2.clone()));
		QuadraticFunding::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		QuadraticFunding::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
		QuadraticFunding::<T>::schedule_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 100u32.into(), 200u32.into(), 10u32.into(), vec![0])?;
		frame_system::Pallet::<T>::set_block_number(150u32.into());
		QuadraticFunding::<T>::contribute(caller_origin.clone(), 0, 100u32.into())?;
		frame_system::Pallet::<T>::set_block_number(210u32.into());
		QuadraticFunding::<T>::finalize_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 0)?;
	}: _(RawOrigin::Root, 0, 0)

	withdraw {
		let caller2: T::AccountId = whitelisted_caller();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&caller2, BalanceOf::<T>::max_value());
		let caller_origin = <T as frame_system::Config>::Origin::from(RawOrigin::Signed(caller2.clone()));
		QuadraticFunding::<T>::fund(caller_origin.clone(), 1000000u32.into())?;
		QuadraticFunding::<T>::create_project(caller_origin.clone(), vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256], vec![b'X'; 256])?;
		QuadraticFunding::<T>::schedule_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 100u32.into(), 200u32.into(), 10u32.into(), vec![0])?;
		frame_system::Pallet::<T>::set_block_number(150u32.into());
		QuadraticFunding::<T>::contribute(caller_origin.clone(), 0, 100u32.into())?;
		frame_system::Pallet::<T>::set_block_number(210u32.into());
		QuadraticFunding::<T>::finalize_round(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 0)?;
		QuadraticFunding::<T>::approve(<T as frame_system::Config>::Origin::from(RawOrigin::Root), 0, 0)?;
	}: _(RawOrigin::Signed(caller2), 0, 0)
}

impl_benchmark_test_suite!(
	QuadraticFunding,
	crate::mock::new_test_ext(),
	crate::mock::Test,
);
