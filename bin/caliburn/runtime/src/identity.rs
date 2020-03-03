use codec::{Decode, Encode};
//use log::info;

use frame_support::{
	decl_error,
	decl_event,
	decl_module,
	decl_storage,
	ensure, StorageMap,
	StorageValue,
};
use sp_runtime::{DispatchResult};
use sp_std::prelude::*;
use system::{self, ensure_signed};


pub trait Trait: system::Trait + timestamp::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct IdentityInfo {
	//pub issuer: AccountId,
	pub name: Vec<u8>,
	pub email: Vec<u8>,
	pub description: Vec<u8>,
	pub kyc_hash:Vec<u8>,
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

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Clone, Default, PartialEq)]
pub struct Credential<Timestamp, AccountId> {
	subject: u32,
	when: Timestamp,
	by: AccountId,
}



decl_event! {
	pub enum Event < T >
	where
		AccountId = <T as system::Trait >::AccountId,
		//Hash = < T as system::Trait >::Hash,
		//Timestamp = < T as timestamp::Trait >::Moment,
	{
        CredentialIssued(AccountId, u32, AccountId),
        // A credential is revoked - holder, subj, issuer.
        CredentialRevoked(AccountId, u32, AccountId),
        // A new subject is created.
        SubjectCreated(AccountId, u32),
        //A new identity is created.
        IdentityCreated(AccountId),
	}
}

decl_error! {
	pub enum Error for Module < T: Trait > {
		TaskDuplicated,
		TaskCheckAddFail,
		TaskChangeStatusFail,
		TaskNotInBoard,
		TaskNotFoundAtIndex,
		TaskNotFoundAtHash,
		TaskInWrongBoard,
		TaskInvalid,
		TaskKindInvalid,
		TaskRecvEmpty,
		BoardDuplicated,
		FundsRecvRewardWrongTime,
		FundsIssuserBackWrongTime,
	}

}

decl_storage! {
	trait Store for Module < T: Trait > as Identity {
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

        Nonce: u64;
	}
}

decl_module! {
	pub struct Module < T: Trait > for enum Call where origin: T::Origin {
		type Error = Error < T >;
		fn deposit_event() = default;
	}
}


impl<T: Trait> Module<T> {
	pub fn do_issue(origin: T::Origin, to: T::AccountId, subject: u32) -> DispatchResult {
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
		Ok(())
	}

	pub fn do_revoke_credential(origin: T::Origin, to: T::AccountId, subject: u32) -> DispatchResult {
		// Check if origin is an issuer.
		// Check if credential is issued.
		// Change the bool flag of the stored credential tuple to false.

		let sender = ensure_signed(origin)?;
		let subject_issuer = Self::subjects(subject);
		ensure!(subject_issuer == sender, "Unauthorized.");
		ensure!(<Credentials<T>>::exists((to.clone(), subject)), "Credential not issued yet.");

		<Credentials<T>>::remove((to.clone(), subject));
		Self::deposit_event(RawEvent::CredentialRevoked(to, subject, sender));
		Ok(())
	}

	/// Create a new subject.
	pub fn do_create_subject(origin: T::Origin) -> DispatchResult  {
		let sender = ensure_signed(origin)?;
		//ensure!(sender == <CredManager<T>>::get(), "Unauthorized.");
		let subject_count = <SubjectCount>::get();

		//ensure!(subject_count < MAX_SUBJECT, "Max issuance count reached");

		<Subjects<T>>::insert(subject_count, sender.clone());


		// Update the subject nonce.
		<SubjectCount>::put(subject_count + 1);

		// Deposit the event.
		Self::deposit_event(RawEvent::SubjectCreated(sender,  subject_count));
		Ok(())
	}

	pub fn do_verify_credential(origin : T::Origin, holder: T::AccountId, subject: u32) -> DispatchResult {
		let _sender = ensure_signed(origin)?;

		// Ensure credential is issued and allowed to be verified.
		ensure!(<Credentials<T>>::exists((holder.clone(), subject)), "Credential not issued yet.");
		Ok(())
	}

	///register User
	pub fn do_register_user(origin: T::Origin, name: Vec<u8>, email: Vec<u8>, description: Vec<u8>, additional: Vec<u8>, kyc_hash: Vec<u8>) -> DispatchResult {
		let sender = ensure_signed(origin)?;
		let mut info = IdentityInfo::default();
		info.name = name;
		info.email = email;
		info.description = description;
		info.additional = additional;
		info.kyc_hash = kyc_hash;

		let identity_count = <IdentityCount>::get();
		<Identities>::insert(identity_count, info.clone());
		<IdentityIndex<T>>::insert(sender.clone(), identity_count);
		identity_count.checked_add(1).ok_or("error to add task")?;
		<IdentityCount>::put(identity_count+1);
		Self::deposit_event(RawEvent::IdentityCreated(sender.clone()));
		Ok(())
	}

	pub fn check_credential(player: &T::AccountId, sub: &u32) -> bool {
		<Credentials<T>>::exists((player, sub))
	}

}
