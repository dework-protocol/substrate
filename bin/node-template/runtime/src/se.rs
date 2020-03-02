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

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct IdentityInfo {
	//pub issuer: AccountId,
	pub name: Vec<u8>,
	pub email: Vec<u8>,
	pub description: Vec<u8>,
	pub additional: Vec<u8>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct SkillSubjectDetails<AccountId> {
	pub issuer: AccountId,
	pub name: Vec<u8>,
	pub tags: Vec<u8>,
	pub description: Vec<u8>,
}


decl_storage! {
    trait Store for Module<T: Trait> as SkillExchange {

    	//// Part 1. ID

        // Global nonce for subject count.
        SubjectCount get(subject_count) config(): u32;
		// Issuers can issue credentials to others.
        // Issuer to Subject mapping.
        Subjects get(subjects) config(): map u32 => T::AccountId;

        // Credentials store.
        // Mapping (holder, subject) to Credential.
        Credentials get(credentials): map (T::AccountId, u32) => Credential<T::Moment, T::AccountId>;
        //credential manager
        CredManager get(cred_manager) config(): T::AccountId;

        //Reputation: default is 50.
        Reputation get(rep) config(): map T::AccountId => u32;

        //User map.
        Identities get(identities): map u64 => IdentityInfo;
        IdentityCount get(identity_count) : u64;
        IdentityIndex: map T::AccountId => u64;

        //Order map.
//        Orders get(orders): map u64 => Order<T::Hash, T::AccountId>;
//        OrderCount get(order_count) : u64;
//        OrderIndex: map T::Hash => u64;

        Nonce: u64;
    }
    //extra_genesis_skip_phantom_data_field;
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Hash = <T as system::Trait>::Hash,

    {
        // A credential is issued - holder, subj, issuer.
        CredentialIssued(AccountId, u32, AccountId),
        // A credential is revoked - holder, subj, issuer.
        CredentialRevoked(AccountId, u32, AccountId),
        // A new subject is created.
        SubjectCreated(AccountId, u32),
        //A new task is published.
        TaskPublished(AccountId),
        //A new task is claimed.
        TaskClaimed(AccountId, Hash),
        //A new identity is created.
        IdentityCreated(AccountId),
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
        pub fn update_reputation(origin, who: T::AccountId, rep_value: u32) {
            let sender = ensure_signed(origin)?;
        	<Reputation<T>>::insert(who, rep_value);
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
			if !<Credentials<T>>::exists((player, sub)) {
				return false;
			}
		}
		<Reputation<T>>::get(player) >= task.min_rep
	}
}

