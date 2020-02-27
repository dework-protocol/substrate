use codec::{Decode, Encode};
use log::info;

use frame_support::{
	decl_error,
	decl_event,
	decl_module,
	decl_storage,
	dispatch::fmt::Debug,
	ensure, StorageMap,
	StorageValue,
	traits::{
		Currency,
		ExistenceRequirement,
		WithdrawReason,
		WithdrawReasons,
	},
};
use sp_core::offchain::Timestamp;
use sp_runtime::{DispatchResult, RuntimeDebug, traits::{Hash, Zero}};
use sp_std::prelude::*;
use sp_std::{result::Result, ops::Div};
use system::{self, ensure_signed};

use crate::task_board::FundsExchange::IssuerPay;

pub trait Trait: system::Trait + timestamp::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Default, Eq, PartialEq)]
pub struct Task<Hash, AccountId, Timestamp, Balance> {
	pub hash: Hash,
	pub issuer: AccountId,
	pub receivers: Vec<AccountId>,
	pub description: Vec<u8>,
	/// done condition / overdue treatment
	pub judge: Vec<u8>,
	pub pay: Balance,
	pub min_rep: u32,
	pub kind: TaskKind,
	pub history: Vec<(TaskKind, Timestamp)>,
	pub req_subjects: Vec<u32>,
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
pub enum TaskKind {
	Published,
	InDelivery,
	Arbitration,
	/// final status
	Overdue,
	/// final status
	Done,
}

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, Eq, PartialEq)]
pub enum FundsExchange {
	IssuerPay,
	RecvReward,
	IssuerBack,
}

impl Default for TaskKind {
	fn default() -> Self {
		Self::Published
	}
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct Board<Hash> {
	pub inner_board: Vec<Hash>,
	pub task_kind: TaskKind,
}

decl_event! {
	pub enum Event < T >
	where
		AccountId = <T as system::Trait >::AccountId,
		Hash = < T as system::Trait >::Hash,
		Timestamp = < T as timestamp::Trait >::Moment,
	{
		/// params: task-hash, form_kind, to_kind
		TaskChangeState(Hash, u8, u8, Timestamp),
		TaskPublish(AccountId, Hash, Timestamp),
	}
}

decl_error! {
	pub enum Error for Module < T: Trait > {
		TaskDuplicated,
		TaskCheckAddFail,
		TaskChangeStatusFail,
		TaskNotInBoard,
		TaskNotFoundAtIndex,
		TaskNotFoundAtHash,
		TaskInWrongBoard,
		TaskInvalid,
		TaskKindInvalid,
		TaskRecvEmpty,
		BoardDuplicated,
		FundsRecvRewardWrongTime,
		FundsIssuserBackWrongTime,
	}

}

decl_storage! {
	trait Store for Module < T: Trait > as DeWorkTasks {
		Tasks get(tasks): map u64 => Task< T::Hash, T::AccountId, T::Moment, T::Balance >;
		TaskCount get(task_count): u64;
		TaskIndex: map T::Hash => u64;

		BoardManager get(load_board): map u8 => Board < T::Hash >;
		Nonce: u64;
		IssuerPayPool: map T::AccountId => T::Balance;
	}
}

decl_module! {
	pub struct Module < T: Trait > for enum Call where origin: T::Origin {
		type Error = Error < T >;
		fn deposit_event() = default;

		pub fn publish_task(origin , desc: Vec < u8 >, min_rep: u32, pay: T::Balance, req_subjects: Vec < u32 > ) {
			Self::do_publish_task(origin, desc, min_rep, pay, req_subjects) ?;
		}
	}
}

impl<T: Trait> Module<T> {
	pub fn do_publish_task(origin: T::Origin, desc: Vec<u8>, min_rep: u32, pay: T::Balance, req_subjects: Vec<u32>) -> DispatchResult {
		let sender = ensure_signed(origin)?;
		let mut task = Task::default();
		task.description = desc;
		task.min_rep = min_rep;
		task.pay = pay;
		task.issuer = sender.clone();
		task.req_subjects = req_subjects.clone();
		task.hash = Self::task_hash(&task);
		ensure!( ! ( < TaskIndex < T > >::exists(task.hash.clone())), Error::< T >::TaskDuplicated);
		Self::change_task_status(&mut task, TaskKind::Published)?;
		Ok(())
	}

	pub fn task_hash(task: &Task<T::Hash, T::AccountId, T::Moment, T::Balance>) -> T::Hash {
		let nonce = <Nonce>::get();
		<Nonce>::mutate(|n| *n += 1);
		(task, nonce).using_encoded(<T as system::Trait>::Hashing::hash)
	}

	pub fn save_task(task: &Task<T::Hash, T::AccountId, T::Moment, T::Balance>) -> DispatchResult {
		if !<TaskIndex<T>>::exists(task.hash.clone()) {
			ensure!(task.kind.clone() == TaskKind::Published, Error::< T >::TaskKindInvalid);
			let task_count = <TaskCount>::get();
			task_count.checked_add(1).ok_or(Error::<T>::TaskCheckAddFail)?;
			Self::exchange_of_funds(task, FundsExchange::IssuerPay)?;

			<Tasks<T>>::insert(task_count, task);
			<TaskIndex<T>>::insert(task.clone().hash, task_count);
			<TaskCount>::put(task_count + 1);
		} else {
			let index = <TaskIndex<T>>::get(task.hash.clone());
			match task.kind.clone() {
				TaskKind::Overdue => {
					Self::exchange_of_funds(task, FundsExchange::IssuerBack)?;
				}
				TaskKind::Done => {
					Self::exchange_of_funds(task, FundsExchange::RecvReward)?;
				}
				_ => {}
			}
			<Tasks<T>>::insert(index, task);
		}
		Ok(())
	}

	pub fn query_task_by_hash(hash: T::Hash) -> sp_std::result::Result<Task<T::Hash, T::AccountId, T::Moment, T::Balance>, Error<T>> {
		let index = <TaskIndex<T>>::get(hash);
		if !<Tasks<T>>::exists(index) {
			return Err(Error::<T>::TaskNotFoundAtIndex);
		}
		Ok(<Tasks<T>>::get(index))
	}

	pub fn change_task_status(task: &mut Task<T::Hash, T::AccountId, T::Moment, T::Balance>, to_task_kind: TaskKind) -> DispatchResult {
		task.kind = to_task_kind.clone();
		task.history.push((task.kind.clone(), <timestamp::Module<T>>::get()));

		match to_task_kind {
			TaskKind::Published => {
				ensure!( ! ( < TaskIndex < T >>::exists(task.hash.clone())), Error::< T >::TaskDuplicated);
				Self::save_task(&task)?;
				Self::deposit_event(RawEvent::TaskPublish(task.issuer.clone(), task.hash.clone(), <timestamp::Module<T>>::get()));
			}
			_ => {
				ensure!((task.kind.clone() as u32) < (to_task_kind.clone() as u32), Error::< T >::TaskChangeStatusFail);
				let mut bm_form = <BoardManager<T>>::get(task.kind.clone() as u8);
				let mut bm_to = <BoardManager<T>>::get(to_task_kind.clone() as u8);
				ensure!(bm_form.inner_board.contains( &task.hash), Error::< T >::TaskNotInBoard);
				ensure!( ! bm_to.inner_board.contains( & task.hash), Error::< T >::TaskInWrongBoard);

				Self::save_task(&task)?;
				bm_form.inner_board.remove_item(&task.hash);
				bm_to.inner_board.push(task.hash.clone());

				<BoardManager<T>>::insert(bm_form.task_kind.clone() as u8, bm_form);
				<BoardManager<T>>::insert(bm_to.task_kind.clone() as u8, bm_to);

				Self::deposit_event(RawEvent::TaskChangeState(task.hash.clone(), task.kind.clone() as u8, to_task_kind.clone() as u8, <timestamp::Module<T>>::get()));
			}
		}
		Ok(())
	}

	pub fn exchange_of_funds(task: &Task<T::Hash, T::AccountId, T::Moment, T::Balance>, kind: FundsExchange) -> DispatchResult {
		match kind {
			FundsExchange::IssuerPay => {
				<balances::Module<T> as Currency<_>>::withdraw(&task.issuer, task.pay, WithdrawReasons::all(), ExistenceRequirement::KeepAlive)?;
				<IssuerPayPool<T>>::insert(task.issuer.clone(), task.pay.clone());
			}
			FundsExchange::RecvReward => {
				ensure!(task.kind.clone() == TaskKind::Done, Error::<T>::FundsRecvRewardWrongTime);
				ensure!(task.receivers.len() > 0, Error::<T>::TaskRecvEmpty);
				let mut pay = <IssuerPayPool<T>>::get(task.issuer.clone());
				let each = pay.div(T::Balance::from(task.receivers.len() as u32));
				task.receivers.iter().enumerate().map(|(i, r)| {
					let mut _each = each;
					if i == task.receivers.len() - 1 {
						_each = pay
					}
					<balances::Module<T> as Currency<_>>::deposit_into_existing(r, _each);
					pay -= _each;
				}).count();
				<IssuerPayPool<T>>::insert(task.issuer.clone(), T::Balance::from(0_u32));
			}
			FundsExchange::IssuerBack => {
				ensure!(task.kind.clone() == TaskKind::Overdue, Error::<T>::FundsIssuserBackWrongTime);
				let pay = <IssuerPayPool<T>>::get(&task.issuer);
				<balances::Module<T> as Currency<_>>::deposit_into_existing(&task.issuer, pay);
				<IssuerPayPool<T>>::insert(task.issuer.clone(), T::Balance::from(0_u32));
			}
			_ => {}
		}
		Ok(())
	}
}
