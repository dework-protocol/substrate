use frame_support::{decl_event, decl_module, decl_storage, StorageMap, StorageValue, ensure};
use system::ensure_signed;
use sp_std::prelude::Vec;
use sp_runtime::{DispatchResult};
use sp_std::prelude::*;
use codec::{Decode, Encode};
use core::u32::MAX as MAX_SUBJECT;
use sp_runtime::traits::{Hash};
//use runtime_primitives::traits::{Hash};
//use nicks;


pub trait Trait: system::Trait + timestamp::Trait +  balances::Trait /*+ nicks::Trait*/{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

//pub type Subject = u32;

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Clone, Default, PartialEq)]
pub struct Credential<Timestamp, AccountId> {
   subject: u32,
   when: Timestamp,
   by: AccountId
}


#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Task<Hash, AccountId, Timestamp, Balance> {
    pub hash: Hash,
    pub issuer: AccountId,
    pub description: Vec<u8>,
    pub when: Timestamp,
    pub pay: Balance,
    pub min_rep: u32,
    pub status: u32, /* 0: published, 1: claimed*/
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct UserInfo<> {

    pub rep: u32,
}


#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct IdSubjectDetails<AccountId> {
    pub issuer: AccountId,
    pub name: Vec<u8>,
    pub tags: Vec<u8>,
    pub description: Vec<u8>,
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
        // Global nonce for subject count.
        SubjectCount get(subject_count) config(): u32;
        //Reputation: default is 50.
        Reputation get(rep) config(): map T::AccountId => u32;
        //Task store.
        //Mapping issuer to task.
        Tasks get(tasks): map T::Hash => Task<T::Hash, T::AccountId, T::Moment, T::Balance>;

        //NewSubjectCount get(new_subject_count) config(): u32;
        // Issuers can issue credentials to others.
        // Issuer to Subject mapping.
        //Subjects get(subjects) config(): map u32 => T::AccountId;
        Subjects get(subjects) config(): map u32 => T::AccountId;

        // Credentials store.
        // Mapping (holder, subject) to Credential.
        Credentials get(credentials): map (T::AccountId, u32) => Credential<T::Moment, T::AccountId>;
        Nonce: u64;

        //credential manager
        CredManager get(cred_manager) config(): T::AccountId;

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

            <Credentials<T>>::insert((to.clone(), 1), cred);

            Self::deposit_event(RawEvent::CredentialIssued(to, 1, sender));
        }

        /// Publish a task.
        pub fn publish_task(origin, _description: Vec<u8>, min_rep: u32, pay: T::Balance) -> DispatchResult {
          Self::do_publish_task(origin, _description, min_rep, pay)
        }

        /// Claim a task.
        pub fn claim_task(origin, task_hash: T::Hash) -> DispatchResult {
          Self::do_claim_task(origin, task_hash)
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
    }
}


impl<T: Trait> Module<T> {

    pub fn do_publish_task(origin: T::Origin, _description: Vec<u8>, min_rep: u32, pay: T::Balance) -> DispatchResult {
        let sender = ensure_signed(origin)?;
        let _nonce = <Nonce>::get();

//        let hash = (<system::Module<T>>::random_seed(), sender.clone(), nonce)
//            .using_encoded(<T as system::Trait>::Hashing::hash);
		//TODO:
		let hash = sender.clone().using_encoded(<T as system::Trait>::Hashing::hash);

        frame_support::print(hash.encode().as_slice());

        let now = <timestamp::Module<T>>::get();

        let task = Task {
            hash: hash,
            issuer: sender.clone(),
            description: _description,
            pay: pay,
            when: now,
            min_rep: min_rep,
            status: 0,
        };

        <Tasks<T>>::insert(hash, task.clone());
        //let t = <Tasks<T>>::get(hash.clone()).hash;
        //frame_support::print(t.encode().as_slice());

        Self::deposit_event(RawEvent::TaskPublished(sender));
        Ok(())
  }

    pub fn do_claim_task(origin: T::Origin, hash: T::Hash) -> DispatchResult {
        let sender = ensure_signed(origin)?;
        ensure!(<Tasks<T>>::exists(hash.clone()), "no task found according to the given hash.");
        let mut task = <Tasks<T>>::get(hash.clone());
        ensure!(task.status == 0, "task not waiting for claim.");
        let min_rep = task.min_rep;
        ensure!(<Reputation<T>>::exists(sender.clone()), "no valid user reputation.");
        let rep = <Reputation<T>>::get(sender.clone());
        ensure!(rep >= min_rep, "reputation not matched.");
        task.status = 1;
        <Tasks<T>>::insert(hash, task.clone());
        Self::deposit_event(RawEvent::TaskClaimed(sender, hash));


        Ok(())
    }
}


#[cfg(test)]
mod tests {
  use super::*;

  use primitives::{Blake2Hasher, H256};
  use frame_support::with_externalities;
  use runtime_primitives::{
    testing::{Digest, DigestItem, Header},
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
  };
  use support::{assert_noop, assert_ok, impl_outer_origin};

  impl_outer_origin! {
    pub enum Origin for Test {}
  }

  // For testing the module, we construct a mock runtime. This means
  // first constructing a configuration type (`Test`) which implements each of the
  // configuration traits of modules we use.
  #[derive(Clone, Eq, PartialEq)]
  pub struct Test;
  impl system::Trait for Test {
    type Origin = Origin;
    type Index = u32;
    type BlockNumber = u32;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u32;
    type Lookup = IdentityLookup<u32>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
  }
  impl timestamp::Trait for Test {
    type Moment = u32;
    type OnTimestampSet = ();
  }
  impl Trait for Test {
    type Event = ();
  }
  type SkillExchange = Module<Test>;

  // builds the genesis config store and sets mock values
  fn new_test_ext() -> frame_support::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
      .build_storage()
      .unwrap()
      .0;
    t.extend(
      GenesisConfig::<Test> {
        subjects: vec![(1, 1), (2, 2)],
        subject_count: 3,
      }
      .build_storage()
      .unwrap()
      .0,
    );
    t.into()
  }

  #[test]
  fn should_fail_issue() {
    with_externalities(&mut new_test_ext(), || {
        assert_noop!(
            SkillExchange::issue_credential(Origin::signed(1), 3, 2),
            "Unauthorized.");
    });
  }

  #[test]
  fn should_issue() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(
            SkillExchange::issue_credential(Origin::signed(1), 3, 1));
    });
  }

  #[test]
  fn should_revoke() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(
            SkillExchange::issue_credential(Origin::signed(1), 3, 1));
        assert_ok!(
            SkillExchange::revoke_credential(Origin::signed(1), 3, 1));
    });
  }

  #[test]
  fn should_add_subject() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(
            SkillExchange::create_subject(Origin::signed(3)));
        assert_eq!(
            SkillExchange::subjects(3), 3);
    });
  }
}
