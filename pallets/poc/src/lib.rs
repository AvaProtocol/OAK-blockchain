#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::Decode;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo,
		pallet_prelude::*,
		traits::IsSubType,
		weights::{GetDispatchInfo, Weight},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{Convert, Dispatchable};
	use sp_std::prelude::*;
	use cumulus_primitives_core::ParaId;
	use xcm::latest::prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The overarching call type.
		type Call: Parameter
			+ Dispatchable<Origin = Self::Origin>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::Call>;

		//The paraId of this chain.
		type SelfParaId: Get<ParaId>;

		type AccountIdToMultiLocation: Convert<Self::AccountId, MultiLocation>;

		type XcmSender: SendXcm;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T> {
		/// The weight of the encoded call.
		CallWeight { weight: Weight },
		/// The dispatch result.
		DispatchResult { result: DispatchResult },
		CallSent,
		ErrorSendingCall { error: SendError},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not a valid encoded call.
		BadEncodedCall,
	}

	#[derive(Debug, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct TaskTypeOne<T: Config> {
		owner: T::AccountId,
		call: Box<<T as Config>::Call>,
	}

	#[derive(Debug, Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct TaskTypeTwo<T: Config> {
		owner: T::AccountId,
		encoded_call: Vec<u8>,
	}

	#[pallet::storage]
	#[pallet::getter(fn get_task_type_one)]
	pub type RunnerOne<T: Config> = StorageValue<_, TaskTypeOne<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_task_type_two)]
	pub type RunnerTwo<T: Config> = StorageValue<_, TaskTypeTwo<T>>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: T::BlockNumber) -> Weight {
			if let Some(task_one) = Self::get_task_type_one() {
				let signed_who: T::Origin = frame_system::RawOrigin::Signed(task_one.owner).into();
				let e = task_one.call.dispatch(signed_who);
				Self::deposit_event(Event::DispatchResult {
					result: e.map(|_| ()).map_err(|e| e.error),
				});
				RunnerOne::<T>::kill();
			};

			if let Some(task_two) = Self::get_task_type_two() {
				let signed_who: T::Origin = frame_system::RawOrigin::Signed(task_two.owner).into();
				let call: <T as Config>::Call =
					Decode::decode(&mut &*task_two.encoded_call).unwrap();
				let e = call.dispatch(signed_who);
				Self::deposit_event(Event::DispatchResult {
					result: e.map(|_| ()).map_err(|e| e.error),
				});
				RunnerTwo::<T>::kill();
			};

			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		///Test accepting a call
		#[pallet::weight(100_000)]
		pub fn call_now(
			origin: OriginFor<T>,
			call: Box<<T as Config>::Call>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let dispatch_weight = call.get_dispatch_info().weight;
			Self::deposit_event(Event::CallWeight { weight: dispatch_weight });

			let signed_who: T::Origin = frame_system::RawOrigin::Signed(who).into();
			let e = call.dispatch(signed_who);
			Self::deposit_event(Event::DispatchResult {
				result: e.map(|_| ()).map_err(|e| e.error),
			});

			Ok(().into())
		}

		///Test accepting a call and triggering in the next block.
		#[pallet::weight(100_000)]
		pub fn call_later(
			origin: OriginFor<T>,
			call: Box<<T as Config>::Call>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let dispatch_weight = call.get_dispatch_info().weight;
			Self::deposit_event(Event::CallWeight { weight: dispatch_weight });

			let task = TaskTypeOne::<T> { owner: who, call };
			RunnerOne::<T>::put(task);

			Ok(().into())
		}

		/// Test accepting an encoded call
		#[pallet::weight(100_000)]
		pub fn encoded_now(
			origin: OriginFor<T>,
			encoded_call: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let call: <T as Config>::Call =
				Decode::decode(&mut &*encoded_call).map_err(|_| Error::<T>::BadEncodedCall)?;
			let dispatch_weight = call.get_dispatch_info().weight;
			Self::deposit_event(Event::CallWeight { weight: dispatch_weight });

			let signed_who: T::Origin = frame_system::RawOrigin::Signed(who).into();
			let call: <T as Config>::Call = Decode::decode(&mut &*encoded_call).unwrap();
			let e = call.dispatch(signed_who);
			Self::deposit_event(Event::DispatchResult {
				result: e.map(|_| ()).map_err(|e| e.error),
			});

			Ok(().into())
		}

		/// Test accepting an encoded call and triggering in the next block.
		#[pallet::weight(100_000)]
		pub fn encoded_later(
			origin: OriginFor<T>,
			encoded_call: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let call: <T as Config>::Call =
				Decode::decode(&mut &*encoded_call).map_err(|_| Error::<T>::BadEncodedCall)?;
			let dispatch_weight = call.get_dispatch_info().weight;
			Self::deposit_event(Event::CallWeight { weight: dispatch_weight });

			let task = TaskTypeTwo::<T> { owner: who, encoded_call };
			RunnerTwo::<T>::put(task);

			Ok(().into())
		}

		#[pallet::weight(100_000)]
		pub fn xcm_test(origin: OriginFor<T>, encoded_call: Vec<u8>, target_chain: u32) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let instruction_set = Self::create_xcm_instruction_set(who, encoded_call);

			match T::XcmSender::send_xcm(
				(1, Junction::Parachain(target_chain)),
				instruction_set,
			) {
				Ok(()) => {
					Self::deposit_event(Event::CallSent);
				},
				Err(e) => {
					Self::deposit_event(Event::ErrorSendingCall{ error: e});
				},
			};

			Ok(().into())
		}
	} 

	impl<T: Config> Pallet<T> {

		pub fn create_xcm_instruction_set(caller:T::AccountId, encoded_call: Vec<u8>) -> xcm::v2::Xcm<()> {
			let local_asset = MultiAsset {
				id: Concrete(MultiLocation::new(1, X1(Parachain(T::SelfParaId::get().into())))),
				fun: Fungibility::Fungible(5_000_000_000_000), //500 TUR
			};

			let descend_location: Junctions = T::AccountIdToMultiLocation::convert(caller).try_into().unwrap();

			let withdraw = WithdrawAsset::<()>(vec![local_asset.clone()].into());
			let buy_execution = BuyExecution::<()>{ fees: local_asset, weight_limit: Unlimited };
			let descend = DescendOrigin(descend_location);
			let transact =  Transact::<()> { origin_type: OriginKind::SovereignAccount , require_weight_at_most: 3_000_000_000, call:  encoded_call.encode().into()};

			Xcm(vec![
				withdraw,
				buy_execution,
				descend,
				transact,
			])
		}
	}
}
