//use core::u32::MAX as MAX_SUBJECT;

use codec::{Decode, Encode};

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

use crate::identity::{self};
use crate::task_board::{self, Error as BoardError, Task, TaskKind};

//use log::info;

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
        pub fn publish_task(origin, _description: Vec<u8>, min_rep: u32, pay: T::Balance, judge_pay: T::Balance, req_subjects: Vec<u32>) -> DispatchResult {
          task_board::Module::<T>::publish_task(origin, _description, min_rep, pay, judge_pay, req_subjects)
        }

        /// Claim a task.
        pub fn claim_task(origin, task_hash: T::Hash, players: Vec<T::AccountId>) -> DispatchResult {
          <task_board::Module<T>>::claim_task(origin, task_hash, players)
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

    }
}

