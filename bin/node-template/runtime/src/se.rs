use core::u32::MAX as MAX_SUBJECT;

use codec::{Decode, Encode};
use log::info;

use frame_support::{decl_event, decl_module, decl_storage, ensure, StorageMap, StorageValue};
//use sp_std::prelude::*;
//use runtime_primitives::traits::{Hash};
//use nicks;
use identity;
use sp_runtime::DispatchResult;
use sp_runtime::traits::Hash;
use sp_std::prelude::*;
use sp_std::prelude::Vec;
use system::ensure_signed;

use crate::task_board::{self, Error as BoardError, Task, TaskKind};

pub trait Trait: system::Trait + timestamp::Trait + balances::Trait /*+ nicks::Trait*/ + task_board::Trait {
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

//#[derive(Encode, Decode, Default, Clone, PartialEq)]
//#[cfg_attr(feature = "std", derive(Debug))]
//pub struct IdentityInfo {
//	/// Additional fields of the identity that are not catered for with the struct's explicit
//	/// fields.
//	pub additional: Vec<u8>,
//
//	/// A reasonable display name for the controller of the account. This should be whatever it is
//	/// that it is typically known as and should not be confusable with other entities, given
//	/// reasonable context.
//	///
//	/// Stored as UTF-8.
//	pub display: Vec<u8>,
//
//	/// The full legal name in the local jurisdiction of the entity. This might be a bit
//	/// long-winded.
//	///
//	/// Stored as UTF-8.
//	pub legal: Vec<u8>,
//
//	/// A representative website held by the controller of the account.
//	///
//	/// NOTE: `https://` is automatically prepended.
//	///
//	/// Stored as UTF-8.
//	pub web: Vec<u8>,
//
//	/// The Riot/Matrix handle held by the controller of the account.
//	///
//	/// Stored as UTF-8.
//	pub riot: Vec<u8>,
//
//	/// The email address of the controller of the account.
//	///
//	/// Stored as UTF-8.
//	pub email: Vec<u8>,
//
//	/// The PGP/GPG public key of the controller of the account.
//	///pub pgp_fingerprint: Option<[u8; 20]>,
//
//	/// A graphic image representing the controller of the account. Should be a company,
//	/// organization or project logo or a headshot in the case of a human.
//	pub image: Vec<u8>,

//	/// The Twitter identity. The leading `@` character may be elided.
//	pub email: Vec<u8>,
//}


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
        pub fn issue(origin, to: T::AccountId, subject: u32) {
            // Check if origin is an issuer.
            // Issue the credential - add to storage.

            let sender = ensure_signed(origin)?;
            let subject_issuer = Self::subjects(subject);
            //ensure!(sender == <CredManager<T>>::get(), "Unauthorized.");
            ensure!(subject_issuer == sender, "Unauthorized.");

            //ensure!(<Credentials<T>>::exists((to.clone(), subject)), "Credential already issued to user.");

            let now = <timestamp::Module<T>>::get();
            let cred = Credential {
              subject: subject,
              when: now,
              by: sender.clone()
            };

            <Credentials<T>>::insert((to.clone(), subject), cred);

            Self::deposit_event(RawEvent::CredentialIssued(to, subject, sender));
        }

        /// Publish a task.
        pub fn publish_task(origin, _description: Vec<u8>, min_rep: u32, pay: T::Balance, req_subjects: Vec<u32>) -> DispatchResult {
          task_board::Module::<T>::do_publish_task(origin, _description, min_rep, pay, req_subjects)
        }

        /// Claim a task.
        pub fn claim_task(origin, task_hash: T::Hash, players: Vec<T::AccountId>) -> DispatchResult {
          Self::do_claim_task(origin, task_hash, players)
        }

        /// Apply for a specialist judge
        ///

        /// Revoke a credential.
        /// Only an issuer can call this function.
        pub fn revoke_credential(origin, to: T::AccountId, subject: u32) {
            // Check if origin is an issuer.
            // Check if credential is issued.
            // Change the bool flag of the stored credential tuple to false.

            let sender = ensure_signed(origin)?;
            let subject_issuer = Self::subjects(subject);
            ensure!(subject_issuer == sender, "Unauthorized.");
            ensure!(<Credentials<T>>::exists((to.clone(), subject)), "Credential not issued yet.");

            <Credentials<T>>::remove((to.clone(), subject));
            Self::deposit_event(RawEvent::CredentialRevoked(to, subject, sender));
        }

        /// Verify a credential.
        pub fn verify_credential(origin, holder: T::AccountId, subject: u32) {
            let _sender = ensure_signed(origin)?;

            // Ensure credential is issued and allowed to be verified.
            ensure!(<Credentials<T>>::exists((holder.clone(), subject)), "Credential not issued yet.");
        }

        /// Create a new subject.
        pub fn create_subject(origin) {
            let sender = ensure_signed(origin)?;
            //ensure!(sender == <CredManager<T>>::get(), "Unauthorized.");
            let subject_count = <SubjectCount>::get();

            //ensure!(subject_count < MAX_SUBJECT, "Max issuance count reached");

            <Subjects<T>>::insert(subject_count, sender.clone());


            // Update the subject nonce.
            <SubjectCount>::put(subject_count + 1);

            // Deposit the event.
            Self::deposit_event(RawEvent::SubjectCreated(sender, subject_count));
        }

        ///update_reputation
        pub fn update_reputation(origin, who: T::AccountId, rep_value: u32) {
            let sender = ensure_signed(origin)?;
        	<Reputation<T>>::insert(who, rep_value);
        }

        ///register User
        pub fn register_user(origin, name: Vec<u8>, email: Vec<u8>, description: Vec<u8>, additional: Vec<u8>) {
        	let sender = ensure_signed(origin)?;
        	let info = IdentityInfo {
        	    name: name,
        		email: email,
        		description: description,
        		additional: additional,
        	};
        	let identity_count = <IdentityCount>::get();
        	<Identities>::insert(identity_count, info.clone());
			<IdentityIndex<T>>::insert(sender.clone(), identity_count);
			identity_count.checked_add(1).ok_or("error to add task")?;
			<IdentityCount>::put(identity_count+1);
			Self::deposit_event(RawEvent::IdentityCreated(sender.clone()));
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

