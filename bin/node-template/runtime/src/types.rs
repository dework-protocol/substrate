#[derive(Encode, Decode, Default, Clone, PartialEq)]
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
enum TaskKind<Timestamp> {
	Published(Timestamp),
	InDelivery(Timestamp, Timestamp),
	Arbitration(Timestamp),
	// final
	Overdue,
	// final
	Done(Timestamp),
}

type Result = std::result::Result<T, Box<Error>>;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
trait Board<Hash> {
	type BoardTask;
	fn load_board() -> Self;
	fn exist(&self, task_id: Hash) -> bool;
	fn put(&self, task: Self::Task) -> Result;
	fn get(&self, task_id: Hash) -> Self::Task;
}

enum BoardKind {
	Req,
	Delivery,
	Arbitration,
	Final,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BoardManager<T> {
	pub board: Vec<Task>,
	pub kind: T,
}

// use macro to impl Req, Delivery, Arbitration, Final
impl<Hash> Board<Hash> for BoardManager<BoardKind::Req> {
	type BoardTask = Task;

	fn load_board() -> Self {
		unimplemented!()
	}

	fn exist(&self, task_id: Hash) -> bool {
		unimplemented!()
	}

	fn put(&self, task: _) -> _ {
		unimplemented!()
	}

	fn get(&self, task_id: Hash) -> _ {
		unimplemented!()
	}
}

pub trait Participant {
	type Hash;
	type TaskHash;
	type OrdMatchHash;
	type RepHash;
	type AccountId;
}

pub struct Requester<Hash, AccountId> {}

impl<Hash, AccountId> Participant for Requester<Hash, AccountId> {
	type Hash = Hash;
	type TaskHash = Hash;
	type OrdMatchHash = Hash;
	type RepHash = Hash;
	type AccountId = AccountId;
}

pub struct Executor<T, A> {}

impl<Hash, AccountId> Participant for Executor<Hash, AccountId> {
	type Hash = Hash;
	type TaskHash = Hash;
	type OrdMatchHash = Hash;
	type RepHash = Hash;
	type AccountId = AccountId;
}

pub struct OrderMatch<Hash> {
	req: Hash,
	exe: Hash,
	task: Hash,
}

pub struct Reputation {
	individual: Vec<u8>,
	team: Vec<u8>,
}




