//use core::u32::MAX as MAX_SUBJECT;

use codec::{Decode, Encode};
//use log::info;

use frame_support::{decl_event, decl_module, decl_storage, ensure, StorageMap, StorageValue};
//use sp_std::prelude::*;
//use runtime_primitives::traits::{Hash};
//use nicks;
//use identity;
use sp_runtime::DispatchResult;
//use sp_runtime::traits::Hash;
use sp_std::prelude::*;
use sp_std::prelude::Vec;
use system::ensure_signed;

use crate::task_board::{self, Error as BoardError, Task, TaskKind};
use crate::identity::{self};

pub trait Trait: system::Trait + timestamp::Trait + balances::Trait /*+ nicks::Trait*/ + task_board::Trait + identity::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

//pub type Subject = u32;

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Clone, Default, PartialEq)]
pub struct Credential<Timestamp, AccountId> {
	subject: u32,
	when: Timestamp,
	by: AccountId,
}

decl_storage! {
    trait Store for Module<T: Trait> as Caliburn {
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Hash = <T as system::Trait>::Hash,

    {
        //A new task is published.
        TaskPublished(AccountId),
        //A new task is claimed.
        TaskClaimed(AccountId, Hash),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event() = default;

        /// Issue a credential to an identity.
        /// Only an issuer can call this function.
        pub fn issue(origin, to: T::AccountId, subject: u32) -> DispatchResult{
			identity::Module::<T>::do_issue(origin, to, subject)
        }

        /// Publish a task.
        pub fn publish_task(origin, _description: Vec<u8>, min_rep: u32, pay: T::Balance, req_subjects: Vec<u32>) -> DispatchResult {
          task_board::Module::<T>::do_publish_task(origin, _description, min_rep, pay, req_subjects)
        }

        /// Claim a task.
        pub fn claim_task(origin, task_hash: T::Hash, players: Vec<T::AccountId>) -> DispatchResult {
          Self::do_claim_task(origin, task_hash, players)
        }

        /// Revoke a credential.
        /// Only an issuer can call this function.
        pub fn revoke_credential(origin, to: T::AccountId, subject: u32) -> DispatchResult {
			identity::Module::<T>::do_revoke_credential(origin, to, subject)
        }

        /// Verify a credential.
        pub fn verify_credential(origin, holder: T::AccountId, subject: u32) -> DispatchResult {
			identity::Module::<T>::do_verify_credential(origin, holder, subject)
        }

        /// Create a new subject.
        pub fn create_subject(origin) -> DispatchResult {
			identity::Module::<T>::do_create_subject(origin)
        }

        ///register User
        pub fn register_user(origin, name: Vec<u8>, email: Vec<u8>, description: Vec<u8>, additional: Vec<u8>, kyc_hash: Vec<u8>) -> DispatchResult{
			identity::Module::<T>::do_register_user(origin, name, email, description, additional, kyc_hash)
		}

        ///update_reputation
        pub fn update_reputation(origin, who: T::AccountId, rep_value: u32) -> DispatchResult {
			identity::Module::<T>::do_update_reputation(origin, who, rep_value)
        }
    }
}

impl<T: Trait> Module<T> {
	pub fn do_claim_task(leader: T::Origin, hash: T::Hash, players: Vec<T::AccountId>) -> DispatchResult {
		let sender = ensure_signed(leader)?;
		let mut task = task_board::Module::<T>::query_task_by_hash(hash)?;
		ensure!(task.kind.clone() == TaskKind::Published, "task not waiting for claim.");
		let mut players = players;
		if !players.contains(&sender) {
			players.push(sender.clone())
		}
		let req_subjects = &task.req_subjects;
		for p in &players {
			ensure!(Self::verify_player(&task, p), "player is invalid.");
		}
		task.receivers = players;
		<task_board::Module<T>>::change_task_status(&mut task, TaskKind::InDelivery);
		Self::deposit_event(RawEvent::TaskClaimed(sender, hash));
		Ok(())
	}

	pub fn verify_player(task: &Task<T::Hash, T::AccountId, T::Moment, T::Balance>, player: &T::AccountId) -> bool {
		for sub in &task.req_subjects {
			if !identity::Module::<T>::check_credential(player, sub) {
				return false;
			}
		}
		identity::Module::<T>::get_reputation(player) >= task.min_rep
	}
}