use codec::{Decode, Encode};

use frame_support::{
	decl_error,
	decl_event,
	decl_module,
	decl_storage,
	ensure, StorageMap,
	StorageValue,
};
use sp_runtime::{DispatchResult, RuntimeDebug, traits::{Hash, Zero}};
use sp_std::prelude::*;
use system::{self, ensure_signed};

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Default, Eq, PartialEq)]
pub struct ReputationLevel {
	pub score: u32,
}

impl ReputationLevel {
	const FIXED_REP_REWARD: u32 = 5;
	/// Will add reputation strategies and algorithms
	pub fn reduce(&mut self) -> u32 {
		self.score -= Self::FIXED_REP_REWARD;
		Self::FIXED_REP_REWARD
	}
	/// Will add reputation strategies and algorithms
	pub fn increase(&mut self) -> u32 {
		self.score += Self::FIXED_REP_REWARD;
		Self::FIXED_REP_REWARD
	}
}

#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq)]
pub enum ReputationOp {
	FailedReduce,
	CompleteIncrease,
}

decl_event! {
	pub enum Event <T>
	where AccountId = <T as system::Trait>::AccountId,
	{
		CompletionIncrease(AccountId, u32),
		FailedReduce(AccountId, u32),
	}

}

decl_storage! {
	trait Store for Module<T: Trait> as ReputationModule {
		Reputation: map T::AccountId => ReputationLevel;
	}
}

decl_module! {
	pub struct Module < T: Trait > for enum Call where origin: T::Origin {
		fn deposit_event() = default;
	}
}

impl<T: Trait> Module<T> {
	/// View the credit score of the current account
	pub fn get_account_reputation_level(account_id: &T::AccountId) -> ReputationLevel {
		if !<Reputation<T>>::exists(account_id) {
			let rep = ReputationLevel {
				score: 50
			};
			<Reputation<T>>::insert(account_id, &rep);
			rep
		} else {
			<Reputation<T>>::get(account_id)
		}
	}

	/// Change of reputation according to different status
	pub fn reputation_change(account_id: T::AccountId, op: ReputationOp) -> DispatchResult {
		let mut rep = Self::get_account_reputation_level(&account_id);
		match op {
			ReputationOp::CompleteIncrease => {
				Self::deposit_event(RawEvent::FailedReduce(account_id.clone(), rep.reduce()));
			}
			ReputationOp::FailedReduce => {
				Self::deposit_event(RawEvent::CompletionIncrease(account_id.clone(), rep.increase()));
			}
			_ => {}
		}
		<Reputation<T>>::insert(account_id.clone(), rep);
		Ok(())
	}

}
