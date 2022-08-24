use crate::{
	AccountTaskId, Config, Decode, Encode, Error, Get, Pallet, Task, TaskId, TypeInfo, Weight,
};

#[derive(Debug, Decode, Eq, Encode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ScheduledTasks<T: Config> {
	pub tasks: Vec<AccountTaskId<T>>,
	pub weight: Weight,
}

impl<T: Config> Default for ScheduledTasks<T> {
	fn default() -> Self {
		ScheduledTasks { tasks: vec![], weight: 0 }
	}
}

impl<T: Config> ScheduledTasks<T> {
	pub fn try_push(&mut self, task_id: TaskId<T>, task: &Task<T>) -> Result<&mut Self, Error<T>> {
		let weight = self
			.weight
			.checked_add(Pallet::<T>::execution_weight(&task.action))
			.ok_or(Error::<T>::TimeSlotFull)?;
		if weight > T::MaxWeightPerSlot::get() {
			Err(Error::<T>::TimeSlotFull)?
		}

		self.weight = weight;
		self.tasks.push((task.owner_id.clone(), task_id));
		Ok(self)
	}
}
