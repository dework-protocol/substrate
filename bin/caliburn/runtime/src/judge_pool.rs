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
use sp_runtime::{DispatchError, DispatchResult, RuntimeDebug, traits::{Hash, Zero}};
use sp_std::{self, prelude::*};
use sp_std::result::Result;
use system::{self, ensure_root, ensure_signed};

use crate::reputation;

pub trait Trait: system::Trait + balances::Trait + reputation::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
pub struct Judgement<T: Trait> {
	pub sender: T::AccountId,
	pub judges: Vec<(T::AccountId, ResultKind)>,
	pub kind: JudgeKind,
	pub threshold: u32,
	pub pool_size: u32,
	pub pledge_limit: T::Balance,
}

impl<T: Trait> Default for Judgement<T> {
	fn default() -> Self {
		Judgement {
			sender: Default::default(),
			judges: Default::default(),
			kind: JudgeKind::Processing,
			threshold: Default::default(),
			pool_size: Default::default(),
			pledge_limit: Default::default(),
		}
	}
}

impl<T: Trait> Judgement<T> {
	pub fn verify_repeat(&self, judge: T::AccountId) -> bool {
		for j in self.judges.clone() {
			if j.0 == judge {
				return false;
			}
		}
		true
	}

	pub fn add_judge(&mut self, judge: T::AccountId, result: ResultKind) -> DispatchResult {
		ensure!(self.verify_repeat(judge.clone()), Error::<T>::JudgeRepeat);
		self.judges.push((judge, result));
		if (self.judges.len() as u32) == self.pool_size {
			self.kind = JudgeKind::Done;
		}
		Ok(())
	}

	pub fn result(&self) -> ResultKind {
		let mut count = (0, 0);
		self.judges.iter().map(|judge| {
			match judge.1 {
				ResultKind::ResultTrue => { count.0 += 1 }
				ResultKind::ResultFalse => { count.1 += 1 }
			}
		}).count();
		if count.0 > count.1 { ResultKind::ResultTrue } else { ResultKind::ResultFalse }
	}
}

#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq)]
pub enum JudgeKind {
	UnCreated,
	Processing,
	Done,
}

#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq)]
pub enum ResultKind {
	ResultTrue,
	ResultFalse,
}

decl_event! {
	pub enum Event <T>
	where
	AccountId = <T as system::Trait>::AccountId,
	Hash = <T as system::Trait>::Hash,
	{
		BeginJudge(AccountId, Hash),
	}

}

decl_storage! {
	trait Store for Module<T: Trait> as JudgePoolModule {
		Judges: map T::Hash => Judgement<T>;
		JudgeSize get(judge_size) config(): u32;
		Threshold get(threshold) config(): u32;
		PledgePool: map (T::AccountId, T::Hash) => T::Balance;
		JudgeResult: map T::Hash => u8;
	}
}


decl_error! {
	pub enum Error for Module <T: Trait> {
		JudgeKindInvalid,
		JudgeVerifyFaild,
		JudgementNotFound,
		JudgeAlreadyDone,
		JudgeSizeFull,
		JudgeRepeat,
		JudgeProcessing,
	}
}

decl_module! {
	pub struct Module < T: Trait > for enum Call where origin: T::Origin {
		type Error = Error < T >;
		fn deposit_event() = default;

		/// Changing judge_size through governance
		fn change_judge_size(origin, size: u32) {
			let root = ensure_root(origin)?;
			<JudgeSize>::put(size);
		}

		/// Changing threshold through governance
		fn change_judge_threshold(origin, threshold: u32) {
			let root = ensure_root(origin)?;
			<Threshold>::put(threshold);
		}

		/// Execute judge task
		pub fn exec_judgement(origin, hash: T::Hash, result: u32) {
			Self::do_exec_judgement(origin, hash, result)?;
		}

		/// Verify that the task has been completed
		pub fn verify_judgement_Done(origin, hash: T::Hash) {
			Self::do_view_judgement_result(origin, hash)?;
		}
	}
}


impl<T: Trait> Module<T> {
	pub fn do_exec_judgement(origin: T::Origin, hash: T::Hash, result: u32) -> DispatchResult {
		let sender = ensure_signed(origin)?;
		ensure!(Self::verify_judge_for_hash(sender.clone(), &hash), Error::<T>::JudgeVerifyFaild);
		let judgement: Judgement<T> = <Judges<T>>::get(&hash);
		<balances::Module<T> as Currency<_>>::withdraw(&sender, judgement.pledge_limit, WithdrawReasons::all(), ExistenceRequirement::KeepAlive)?;
		<PledgePool<T>>::insert((sender.clone(), hash.clone()), judgement.pledge_limit.clone());
		Self::add_judge(sender.clone(), &hash, result)?;
		Ok(())
	}

	pub fn do_view_judgement_result(origin: T::Origin, hash: T::Hash) -> Result<ResultKind, DispatchError> {
		let _ = ensure_signed(origin)?;
		match Self::view_progress(hash.clone()) {
			JudgeKind::Done => {
				let result = Self::view_result(hash.clone())?;
				if !<JudgeResult<T>>::exists(hash.clone()) {
					<JudgeResult<T>>::insert(hash.clone(), result as u8);
				}
				Ok(Self::handler_result(<JudgeResult<T>>::get(hash.clone()) as u32))
			}
			_ => {
				Err(Error::<T>::JudgeProcessing.into())
			}
		}
	}
}

impl<T: Trait> Module<T> {
	/// The application layer processes the task corresponding to the hash,
	/// obtains the credentials of the success of the task, and makes a decision
	pub fn begin_judgement(hash: T::Hash, sender: T::AccountId, pledge_limit: T::Balance) -> DispatchResult {
		let mut judge = Judgement::default();
		judge.threshold = Self::threshold();
		judge.pool_size = Self::judge_size();
		judge.pledge_limit = pledge_limit;
		judge.sender = sender;
		Self::save_judgement(&hash, &judge)?;
		Ok(())
	}

	/// View judge task progress
	pub fn view_progress(hash: T::Hash) -> JudgeKind {
		if !<Judges<T>>::exists(hash) {
			return JudgeKind::UnCreated;
		}
		let judgement: Judgement<T> = <Judges<T>>::get(hash.clone());
		judgement.kind.clone()
	}

	/// View results of judge's current ruling
	pub fn view_result(hash: T::Hash) -> Result<ResultKind, DispatchError> {
		if !<Judges<T>>::exists(hash) {
			return Err(Error::<T>::JudgementNotFound.into());
		}
		let judgement = <Judges<T>>::get(hash.clone());
		Ok(judgement.result())
	}
}

impl<T: Trait> Module<T> {
	/// Store judge tasks
	pub fn save_judgement(hash: &T::Hash, judgement: &Judgement<T>) -> DispatchResult {
		if !<Judges<T>>::exists(hash) {
			ensure!(judgement.kind == JudgeKind::Processing, Error::<T>::JudgeKindInvalid);
		} else {
			let _judge = <Judges<T>>::get(hash);
			ensure!((_judge.kind as u8) <= (judgement.kind as u8), Error::<T>::JudgeKindInvalid);
			ensure!(_judge.kind != JudgeKind::Done, Error::<T>::JudgeAlreadyDone);
		}
		<Judges<T>>::insert(hash, judgement);
		Ok(())
	}

	/// Increase judge executive
	pub fn add_judge(sender: T::AccountId, hash: &T::Hash, result: u32) -> DispatchResult {
		ensure!(<Judges<T>>::exists(hash), Error::<T>::JudgementNotFound);
		let mut judgement: Judgement<T> = <Judges<T>>::get(hash);
		ensure!((judgement.judges.len() as u32) < (Self::judge_size() as u32), Error::<T>::JudgeSizeFull);
		judgement.add_judge(sender, Self::handler_result(result))?;
		Self::save_judgement(hash, &judgement);
		Ok(())
	}

	pub fn handler_result(result: u32) -> ResultKind {
		if result == 0 { ResultKind::ResultFalse } else { ResultKind::ResultTrue }
	}

	/// Validate judge based on hash value
	pub fn verify_judge_for_hash(judge: T::AccountId, hash: &T::Hash) -> bool {
		let rep = <reputation::Module<T>>::get_account_reputation_level(&judge);
		if !<Judges<T>>::exists(hash) {
			return false;
		}
		let judgement: Judgement<T> = <Judges<T>>::get(hash);
		rep.score >= Self::threshold() && (judgement.judges.len() as u32) < Self::judge_size() && judgement.verify_repeat(judge)
	}
}
