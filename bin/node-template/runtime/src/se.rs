use frame_support::{decl_event, decl_module, decl_storage, StorageMap, StorageValue, ensure};
use system::ensure_signed;
use sp_std::prelude::Vec;
use sp_runtime::{DispatchResult};
use sp_std::prelude::*;
use codec::{Decode, Encode};
use core::u32::MAX as MAX_SUBJECT;
use sp_runtime::traits::{Hash};
use log::{info};

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
	//pub receivers: Vec<AccountId>,
	pub description: Vec<u8>,
	// done condition / overdue treatment
	//pub judge: Vec<u8>,
	pub when: Timestamp,
    pub pay: Balance,
    pub min_rep: u32,
    //pub status: TaskStatus<Timestamp>, /* 0: published, 1: claimed*/
	pub status: u32, /* 0: published, 1: claimed*/
	pub req_subjects: Vec<u32>,

	//pub kind: TaskKind<Timestamp>,
	//pub history: Vec<TaskKind<Timestamp>>,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Order<Hash, AccountId> {
	hash: Hash,
	task_hash: Hash,
	claimer: AccountId,
	status: u32,
}

#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
enum TaskStatus<Timestamp> {
	Published(Timestamp),
	InDelivery(Timestamp, Timestamp),
	Arbitration(Timestamp),
	// final
	Overdue,
	// final
	Done(Timestamp),
}

#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum OrderStatus {
	WaitReq,
	InProcess,
	Arbitration,
	Final,
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
        Tasks get(tasks): map u64 => Task<T::Hash, T::AccountId, T::Moment, T::Balance>;
		TaskCount get(task_count) : u64;
		TaskIndex: map T::Hash => u64;
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

        //Order map.
        Orders get(orders): map u64 => Order<T::Hash, T::AccountId>;
        OrderCount get(order_count) : u64;
        OrderIndex: map T::Hash => u64;



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
        pub fn publish_task(origin, _description: Vec<u8>, min_rep: u32, pay: T::Balance, req_subjects: Vec<u32>) -> DispatchResult {
          Self::do_publish_task(origin, _description, min_rep, pay, req_subjects)
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

    pub fn do_publish_task(origin: T::Origin, _description: Vec<u8>, min_rep: u32, pay: T::Balance, req_subjects: Vec<u32>) -> DispatchResult {
        //generate task.
		let sender = ensure_signed(origin)?;
        let nonce = <Nonce>::get();
		let hash = (sender.clone(), nonce).using_encoded(<T as system::Trait>::Hashing::hash);
        let now = <timestamp::Module<T>>::get();

        let task = Task {
            hash: hash,
            issuer: sender.clone(),
            description: _description,
            pay: pay,
            when: now,
            min_rep: min_rep,
            status: 0,
			req_subjects: req_subjects,
        };

		let task_count = <TaskCount>::get();
        <Tasks<T>>::insert(task_count, task.clone());
		<TaskIndex<T>>::insert(hash, task_count);
		task_count.checked_add(1).ok_or("error to add task")?;
		<TaskCount>::put(task_count+1);

		info!("{:?}", hash);
		<Nonce>::mutate(|n| *n += 1);

		Self::deposit_event(RawEvent::TaskPublished(sender));
        Ok(())
  }

    pub fn do_claim_task(origin: T::Origin, hash: T::Hash) -> DispatchResult {
        let sender = ensure_signed(origin)?;

		//check task qualification.
		let task_index = <TaskIndex<T>>::get(hash);
        ensure!(<Tasks<T>>::exists(task_index.clone()), "no task found according to the given hash.");

		let mut task = <Tasks<T>>::get(task_index);
        ensure!(task.status == 0, "task not waiting for claim.");
		let req_subjects = &task.req_subjects;
		for sub in req_subjects {
			ensure!(<Credentials<T>>::exists((sender.clone(), sub)), "subject not qualified.");
		}

        //ensure!(<Reputation<T>>::exists(sender.clone()), "no valid user reputation.");

		let min_rep = task.min_rep;
		let rep = <Reputation<T>>::get(sender.clone());
        ensure!(rep >= min_rep, "reputation not matched.");



		//modify task status.
        task.status = 1;
		<Tasks<T>>::insert(task_index, task.clone());

		// generate order.
		let nonce = <Nonce>::get();
		let order_hash = (/*<system::Module<T>>::random_seed(),*/ sender.clone(), nonce)
			.using_encoded(<T as system::Trait>::Hashing::hash);

		let order = Order {
			hash: order_hash,
			task_hash: hash,
			claimer: sender.clone(),
			status: 1,
		};
		let order_count = <OrderCount>::get();

		<Orders<T>>::insert(order_count, order.clone());
		<OrderIndex<T>>::insert(order_hash, order_count);

		order_count.checked_add(1).ok_or("error to add order")?;
		<OrderCount>::put(order_count+1);

		info!("{:?}", order_hash);

		<Nonce>::mutate(|n| *n += 1);
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
