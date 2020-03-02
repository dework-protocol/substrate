use codec::{Decode, Encode};
use log::info;

use frame_support::dispatch::fmt::Debug;
use sp_runtime::DispatchResult;
use sp_std::prelude::*;
use system;

//use crate::se;

#[derive(Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Task<Hash, AccountId, Timestamp, Balance> {
	pub hash: Hash,
	pub issuer: AccountId,
	pub receivers: Vec<AccountId>,
	pub description: Vec<u8>,
	// done condition / overdue treatment
	pub judge: Vec<u8>,
	pub pay: Balance,
	pub min_rep: u32,
	pub kind: TaskKind<Timestamp>,
	pub history: Vec<TaskKind<Timestamp>>,
}


#[derive(Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum TaskKind<Timestamp> {
	Published(Timestamp),
	InDelivery(Timestamp, Timestamp),
	Arbitration(Timestamp),
	// final
	Overdue,
	// final
	Done(Timestamp),
}

//impl<Timestamp: timestamp::Trait> Default for TaskKind<Timestamp> {
//	fn default() -> Self {
//		let tt = timestamp::Module::<dyn Timestamp>::get();
//		TaskKind::Published(tt)
//	}
//}


trait Board<Hash, AccountId, Timestamp, Balance> {
	fn load_board(kind: BoardKind) -> Self;
	fn exist(&self, task_id: Hash) -> bool;
	fn put(&self, task: Task<Hash, AccountId, Timestamp, Balance>) -> DispatchResult;
	fn get(&self, task_id: Hash) -> Task<Hash, AccountId, Timestamp, Balance>;
}

#[derive(Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum BoardKind {
	Req,
	Delivery,
	Arbitration,
	Final,
}

impl Default for BoardKind {
	fn default() -> Self {
		BoardKind::Req
	}
}

#[derive(Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BoardManager<Hash, AccountId, Timestamp, Balance> {
	pub board: Vec<Task<Hash, AccountId, Timestamp, Balance>>,
	pub kind: BoardKind,
}

// use macro to impl Req, Delivery, Arbitration, Final
impl<
	Hash: Encode + Decode + Copy + Clone + Debug + Eq + PartialEq,
	AccountId: Encode + Decode + Copy + Clone + Debug + Eq + PartialEq,
	Timestamp: timestamp::Trait,
	Balance: Encode + Decode + Copy + Clone + Debug + Eq + PartialEq,
> Board<Hash, AccountId, Timestamp, Balance> for BoardManager<Hash, AccountId, Timestamp, Balance> {
	fn load_board(kind: BoardKind) -> Self {
		unimplemented!()
	}

	fn exist(&self, task_id: Hash) -> bool {
		unimplemented!()
	}

	fn put(&self, task: Task<Hash, AccountId, Timestamp, Balance>) -> DispatchResult {
		unimplemented!()
	}

	fn get(&self, task_id: Hash) -> Task<Hash, AccountId, Timestamp, Balance> {
		unimplemented!()
	}
}

pub trait Participant {
	type TaskHash;
	type OrdMatchHash;
	type RepHash;
	type AccountId;
}

#[derive(Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Requester<AccountId, Hash> {
	account_id: AccountId,
	hash: Hash,
}

impl<
	Hash: Encode + Decode + Copy + Clone + Debug + Eq + PartialEq,
	AccountId: Encode + Decode + Copy + Clone + Debug + Eq + PartialEq
> Participant for Requester<AccountId, Hash> {
	type TaskHash = Hash;
	type OrdMatchHash = Hash;
	type RepHash = Hash;
	type AccountId = AccountId;
}

#[derive(Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Executor<AccountId, Hash> {
	account_id: AccountId,
	hash: Hash,
}

impl<
	Hash: Encode + Decode + Copy + Clone + Debug + Eq + PartialEq,
	AccountId: Encode + Decode + Copy + Clone + Debug + Eq + PartialEq
> Participant for Executor<AccountId, Hash> {
	type TaskHash = Hash;
	type OrdMatchHash = Hash;
	type RepHash = Hash;
	type AccountId = AccountId;
}

#[derive(Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct OrderMatch<Hash, AccountId> {
	hash: Hash,
	req: AccountId,
	exe: AccountId,
	task: Hash,
}

#[derive(Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Reputation {
	individual: Vec<u8>,
	team: Vec<u8>,
}




