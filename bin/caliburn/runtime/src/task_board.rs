use codec::{Decode, Encode};

use frame_support::{
	decl_error,
	decl_event,
	decl_module,
	decl_storage,
	ensure, StorageMap,
	StorageValue,
	traits::{
		Currency,
		ExistenceRequirement,
		WithdrawReason,
		WithdrawReasons,
	},
};
use sp_runtime::{DispatchResult, RuntimeDebug, traits::Hash};
use sp_std::{ops::Div};
use sp_std::prelude::*;
use system::{self, ensure_signed};

use crate::{identity, judge_pool, reputation};

pub trait Trait: system::Trait + timestamp::Trait + balances::Trait + reputation::Trait + identity::Trait + judge_pool::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
pub struct Task<T: Trait> {
	pub hash: T::Hash,
	pub issuer: T::AccountId,
	pub receivers: Vec<T::AccountId>,
	pub description: Vec<u8>,
	/// done condition / Failure treatment
	pub judge_pay: T::Balance,
	pub pay: T::Balance,
	pub min_rep: u32,
	pub kind: TaskKind,
	pub history: Vec<(TaskKind, T::Moment)>,
	pub req_subjects: Vec<u32>,
	pub delivery_certificate: T::Hash,
}

impl<T: Trait> Default for Task<T> {
	fn default() -> Self {
		Task {
			hash: Default::default(),
			issuer: Default::default(),
			receivers: Default::default(),
			description: Default::default(),
			judge_pay: Default::default(),
			pay: Default::default(),
			min_rep: Default::default(),
			kind: Default::default(),
			history: Default::default(),
			req_subjects: Default::default(),
			delivery_certificate: Default::default(),
		}
	}
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
pub enum TaskKind {
	Published,
	InDelivery,
	Deliveryed,
	Arbitration,
	/// final status
	Failure,
	/// final status
	Done,
}

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, Eq, PartialEq)]
pub enum FundsExchange {
	IssuerPay,
	RecvStaking,
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
		TaskClaimed(AccountId, Hash),
	}
}

decl_error! {
	pub enum Error for Module < T: Trait > {
		TaskDuplicated,
		TaskCheckAddFail,
		TaskChangeStatusFail,
		TaskNotWaitForRecv,
		TaskNotInBoard,
		TaskTeamLeaderRepeatSetting,
		TaskNotFoundAtIndex,
		TaskNotFoundAtHash,
		TaskPlayerIsInvalid,
		TaskInWrongBoard,
		TaskInvalid,
		TaskKindInvalid,
		TaskRecvEmpty,
		TaskParticipantInvalid,
		TaskProcessing,
		BoardDuplicated,
		FundsRecvRewardWrongTime,
		FundsIssuserBackWrongTime,
		PermissionError,
	}

}

decl_storage! {
	trait Store for Module < T: Trait > as DeWorkTasks {
		Tasks get(tasks): map u64 => Task<T>;
		TaskCount get(task_count): u64;
		TaskIndex: map T::Hash => u64;

		BoardManager get(load_board): map u8 => Board < T::Hash >;
		Nonce: u64;
		IssuerPayPool: map (T::AccountId, T::Hash) => T::Balance;
		StakingPayPool: map (T::AccountId, T::Hash) => T::Balance;
	}
}

decl_module! {
	pub struct Module < T: Trait > for enum Call where origin: T::Origin {
		type Error = Error < T >;
		fn deposit_event() = default;

		/// Publish tasks on bulletin boards
		pub fn publish_task(origin , desc: Vec < u8 >, min_rep: u32, pay: T::Balance, judge_pay: T::Balance, req_subjects: Vec < u32 > ) {
			Self::do_publish_task(origin, desc, min_rep, pay, judge_pay, req_subjects)?;
		}

		/// Claim accept task
		pub fn claim_task(origin, hash: T::Hash, players: Vec<T::AccountId>) {
			Self::do_claim_task(origin, hash, players)?;
		}

		/// Deliver tasks when they are completed
		pub fn claim_deliver_task(origin, hash: T::Hash, delivery_certificate: T::Hash) {
			Self::do_claim_deliver_task(origin, hash, delivery_certificate)?;
		}

		/// Apply for arbitration
		pub fn request_for_judge(origin, hash: T::Hash) {
			Self::do_request_for_judge(origin, hash)?;
		}

		/// Push the mission to its final state
		pub fn task_to_final(origin, hash: T::Hash) {
			Self::do_task_to_final(origin, hash)?;
		}
	}
}

impl<T: Trait> Module<T> {
	/// Publish tasks on bulletin boards
	pub fn do_publish_task(origin: T::Origin, desc: Vec<u8>, min_rep: u32, pay: T::Balance, judge_pay: T::Balance, req_subjects: Vec<u32>) -> DispatchResult {
		let sender = ensure_signed(origin)?;
		let mut task = Task::default();
		task.description = desc;
		task.min_rep = min_rep;
		task.pay = pay;
		task.judge_pay = judge_pay;
		task.issuer = sender.clone();
		task.req_subjects = req_subjects.clone();
		task.hash = Self::task_hash(&task);
		ensure!( ! ( < TaskIndex < T > >::exists(task.hash.clone())), Error::< T >::TaskDuplicated);
		Self::change_task_status(&mut task, TaskKind::Published)?;
		Ok(())
	}

	/// Claim accept task
	pub fn do_claim_task(leader: T::Origin, hash: T::Hash, players: Vec<T::AccountId>) -> DispatchResult {
		let sender = ensure_signed(leader)?;
		let mut task = Self::query_task_by_hash(hash)?;
		ensure!(task.kind.clone() == TaskKind::Published, <Error<T>>::TaskNotWaitForRecv);
		let mut players = players;
		ensure!(!players.contains(&sender), <Error<T>>::TaskTeamLeaderRepeatSetting);
		players.push(sender.clone());
		let req_subjects = &task.req_subjects;
		for p in players.clone() {
			ensure!(Self::verify_claim(&task, p), <Error<T>>::TaskPlayerIsInvalid);
		}
		task.receivers = players;
		Self::change_task_status(&mut task, TaskKind::InDelivery);
		Self::deposit_event(RawEvent::TaskClaimed(sender, hash));
		Ok(())
	}

	/// Team leader to deliver tasks
	pub fn do_claim_deliver_task(leader: T::Origin, hash: T::Hash, delivery_certificate: T::Hash) -> DispatchResult {
		let sender = ensure_signed(leader)?;
		let mut task = Self::query_task_by_hash(hash)?;
		ensure!(task.kind.clone() == TaskKind::InDelivery, Error::<T>::TaskKindInvalid);
		ensure!(task.receivers.len() > 0, <Error<T>>::TaskRecvEmpty);
		let mut recv = task.receivers.clone();
		ensure!(recv.pop() == Some(sender), Error::<T>::PermissionError);
		task.delivery_certificate = delivery_certificate;
		Self::change_task_status(&mut task, TaskKind::Deliveryed)?;
		Ok(())
	}

	/// Apply for arbitration
	pub fn do_request_for_judge(origin: T::Origin, hash: T::Hash) -> DispatchResult {
		let sender = ensure_signed(origin)?;
		let mut task: Task<T> = Self::query_task_by_hash(hash.clone())?;
		ensure!(Self::is_task_participant(&task, sender.clone()), Error::<T>::TaskParticipantInvalid);
		ensure!(task.kind.clone() == TaskKind::Deliveryed, Error::<T>::TaskKindInvalid);
		<judge_pool::Module<T>>::begin_judgement(hash.clone(), sender.clone(), task.judge_pay)?;
		Self::change_task_status(&mut task, TaskKind::Arbitration);
		Ok(())
	}

	/// Push the mission to its final state
	pub fn do_task_to_final(origin: T::Origin, hash: T::Hash) -> DispatchResult {
		let sender = ensure_signed(origin.clone())?;
		let mut task: Task<T> = Self::query_task_by_hash(hash.clone())?;
		ensure!(Self::is_task_participant(&task, sender.clone()), Error::<T>::TaskParticipantInvalid);
		match task.kind.clone() {
			TaskKind::Arbitration => {
				let result = judge_pool::Module::<T>::do_view_judgement_result(origin, hash.clone())?;
				match result {
					judge_pool::ResultKind::ResultTrue => {
						Self::change_task_status(&mut task, TaskKind::Done)?;
					}
					judge_pool::ResultKind::ResultFalse => {
						Self::change_task_status(&mut task, TaskKind::Failure)?;
					}
				}
				Ok(())
			}
			TaskKind::Failure => {
				Ok(())
			}
			TaskKind::Done => {
				Ok(())
			}
			_ => {
				Err(Error::<T>::TaskProcessing.into())
			}
		}
	}
}

impl<T: Trait> Module<T> {
	pub fn verify_claim(task: &Task<T>, player: T::AccountId) -> bool {
		for sub in &task.req_subjects {
			if !identity::Module::<T>::check_credential(&player, sub) {
				return false;
			}
		}
		reputation::Module::<T>::get_account_reputation_level(&player).score >= task.min_rep && task.issuer.clone() != player.clone()
	}

	pub fn is_task_participant(task: &Task<T>, player: T::AccountId) -> bool {
		task.issuer.clone() == player || task.receivers.contains(&player)
	}

	/// Get task hash
	pub fn task_hash(task: &Task<T>) -> T::Hash {
		let nonce = <Nonce>::get();
		<Nonce>::mutate(|n| *n += 1);
		(task, nonce).using_encoded(<T as system::Trait>::Hashing::hash)
	}

	/// Create if not exist, modify if exist
	pub fn save_task(task: &Task<T>) -> DispatchResult {
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
				TaskKind::InDelivery => {
					Self::exchange_of_funds(task, FundsExchange::RecvStaking)?;
				}
				TaskKind::Failure => {
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

	/// Task query
	pub fn query_task_by_hash(hash: T::Hash) -> sp_std::result::Result<Task<T>, Error<T>> {
		let index = <TaskIndex<T>>::get(hash);
		if !<Tasks<T>>::exists(index) {
			return Err(Error::<T>::TaskNotFoundAtIndex);
		}
		Ok(<Tasks<T>>::get(index))
	}

	/// Task state flow
	pub fn change_task_status(task: &mut Task<T>, to_task_kind: TaskKind) -> DispatchResult {
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

	/// Asset circulation
	pub fn exchange_of_funds(task: &Task<T>, kind: FundsExchange) -> DispatchResult {
		match kind {
			FundsExchange::IssuerPay => {
				Self::issuer_pay(task)?;
			}
			FundsExchange::RecvStaking => {
				Self::recv_staking(task)?;
			}
			FundsExchange::RecvReward => {
				ensure!(task.kind.clone() == TaskKind::Done, Error::<T>::FundsRecvRewardWrongTime);
				ensure!(task.receivers.len() > 0, Error::<T>::TaskRecvEmpty);
				let mut pay = <IssuerPayPool<T>>::get((task.issuer.clone(), task.hash.clone()));
				let each = pay.div(T::Balance::from(task.receivers.len() as u32));
				task.receivers.iter().enumerate().map(|(i, r)| {
					let mut _each = each;
					if i == task.receivers.len() - 1 {
						_each = pay
					}
					<balances::Module<T> as Currency<_>>::deposit_into_existing(r, _each);
					pay -= _each;
				}).count();
				<IssuerPayPool<T>>::insert((task.issuer.clone(), task.hash.clone()), T::Balance::from(0_u32));
			}
			FundsExchange::IssuerBack => {
				ensure!(task.kind.clone() == TaskKind::Failure, Error::<T>::FundsIssuserBackWrongTime);
				Self::issuer_back_pay(task)?;
				if task.history.iter().filter(|t| {
					t.0.clone() == TaskKind::Arbitration
				}).count() == 0 {
					Self::issuer_back_judge(task)?;
				}
			}
			_ => {}
		}
		Ok(())
	}
}

// pay
impl<T: Trait> Module<T> {
	pub fn issuer_pay(task: &Task<T>) -> DispatchResult {
		<balances::Module<T> as Currency<_>>::withdraw(&task.issuer, task.pay, WithdrawReasons::all(), ExistenceRequirement::KeepAlive)?;
		<IssuerPayPool<T>>::insert((task.issuer.clone(), task.hash.clone()), task.pay.clone());
		<balances::Module<T> as Currency<_>>::withdraw(&task.issuer, task.judge_pay, WithdrawReasons::all(), ExistenceRequirement::KeepAlive)?;
		<StakingPayPool<T>>::insert((task.issuer.clone(), task.hash.clone()), task.judge_pay.clone());
		Ok(())
	}

	pub fn recv_staking(task: &Task<T>) -> DispatchResult {
		for r in &task.receivers{
			if !<StakingPayPool<T>>::exists((r, task.hash.clone())) {
				<balances::Module<T> as Currency<_>>::withdraw(r, task.judge_pay, WithdrawReasons::all(), ExistenceRequirement::KeepAlive)?;
				<IssuerPayPool<T>>::insert((r, task.hash.clone()), task.judge_pay.clone());
			}
		}
		Ok(())
	}


	pub fn issuer_back_pay(task: &Task<T>) -> DispatchResult {
		let pay = <IssuerPayPool<T>>::get((&task.issuer, task.hash.clone()));
		<balances::Module<T> as Currency<_>>::deposit_into_existing(&task.issuer, pay)?;
		<IssuerPayPool<T>>::insert((task.issuer.clone(), task.hash.clone()), T::Balance::from(0_u32));
		Ok(())
	}

	pub fn issuer_back_judge(task: &Task<T>) -> DispatchResult {
		let judge_pay = <StakingPayPool<T>>::get((&task.issuer, task.hash.clone()));
		<balances::Module<T> as Currency<_>>::deposit_into_existing(&task.issuer, judge_pay)?;
		<StakingPayPool<T>>::insert((task.issuer.clone(), task.hash.clone()), T::Balance::from(0_u32));
		Ok(())
	}
}
