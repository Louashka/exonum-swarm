#![deny(missing_debug_implementations, unsafe_code, bare_trait_objects)]

#[macro_use]
extern crate exonum;
#[macro_use]
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod schema {
	use exonum::{
		crypto::PublicKey, storage::{Fork, MapIndex, ListIndex, Snapshot},
	};

	encoding_struct! {
		struct Voting {
			voting_id: u64,
			drone: &PublicKey,
			amount: u8,
		}
	}

	encoding_struct! {
		struct VoteAction {
			action_id: u64,
			validator: &PublicKey,
			voting_status: bool,
		}
	}

	encoding_struct! {
		struct VoteCollection {
			voting_id: u64,
			vector: Vec<VoteAction>,
		}
	}

	impl Voting {
		pub fn vote(self) -> Self {
			let amount = self.amount() + 1;
			Self::new(self.voting_id(), self.drone(), amount)
		}
	}

	impl VoteCollection {
		pub fn add_action(self, action: VoteAction) -> Self {
			let mut vector = self.vector();
			vector.push(action);
			Self::new(self.voting_id(), vector)
		}
	}

	#[derive(Debug)]
	pub struct SwarmSchema<T> {
	    view: T,
	}

	impl<T: AsRef<dyn Snapshot>> SwarmSchema<T> {
	    pub fn new(view: T) -> Self {
	        SwarmSchema { view }
	    }

	    pub fn votings(&self) -> MapIndex<&dyn Snapshot, u64, Voting> {
	        MapIndex::new("swarm.votings", self.view.as_ref())
	    }

	    pub fn voting(&self, voting_id: &u64) -> Option<Voting> {
	        self.votings().get(voting_id)
	    }

	    pub fn votings_with_actions(&self) -> MapIndex<&dyn Snapshot, u64, VoteCollection> {
	        MapIndex::new("swarm.votings_with_actions", self.view.as_ref())
	    }

	    pub fn voting_with_actions(&self, voting_id: &u64) -> Option<VoteCollection> {
	        self.votings_with_actions().get(voting_id)
	    }

	}

	impl<'a> SwarmSchema<&'a mut Fork> {
		pub fn votings_mut(&mut self) -> MapIndex<&mut Fork, u64, Voting> {
			MapIndex::new("swarm.votings", &mut self.view)
		}

		pub fn vote_collections_mut(&mut self) -> MapIndex<&mut Fork, u64, VoteCollection> {
			MapIndex::new("swarm.votings_with_actions", &mut self.view)
		}
	}
}

pub mod transactions {
	use exonum::crypto::PublicKey;

    use service::SERVICE_ID;

    transactions! {
		pub VoteTransactions {
			const SERVICE_ID = SERVICE_ID;

			struct TxCreateVoting {
				voting_id: u64,
				drone: &PublicKey,
			}

			struct TxVote {
				action_id: u64,
				voting_id: u64,
				validator: &PublicKey,
				voting_status: bool,
				seed: u64,
			}
		}
	}
}

pub mod errors {
	#![allow(bare_trait_objects)]

    use exonum::blockchain::ExecutionError;

    #[derive(Debug, Fail)]
	#[repr(u8)]
	pub enum Error {
		#[fail(display = "Voting already exists")]
	    VotingAlreadyExists = 0,

	    #[fail(display = "Voting doesn't exist")]
	    VotingNotFound = 1,

	    #[fail(display = "Validator doesn't exist")]
	    ValidatorNotFound = 2,

	    #[fail(display = "Validator already voted")]
	    ValidatorAlreadyVoted = 3,
	}

	impl From<Error> for ExecutionError {
	    fn from(value: Error) -> ExecutionError {
	        let description = format!("{}", value);
	        ExecutionError::with_description(value as u8, description)
	    }
	}
}

pub mod contracts {
	use exonum::{
        blockchain::{ExecutionResult, Transaction}, messages::Message, storage::Fork,
    };

    use errors::Error;
    use schema::{SwarmSchema, Voting, VoteAction, VoteCollection};
    use transactions::{TxCreateVoting, TxVote};

    const INIT_AMOUNT: u8 = 0;

    impl Transaction for TxCreateVoting {
		fn verify(&self) -> bool {
			//self.verify_signature(self.drone())
			true
		}

		fn execute(&self, view: &mut Fork) -> ExecutionResult {
			let mut schema = SwarmSchema::new(view);
			if schema.voting(&self.voting_id()).is_none() {
				let voting = Voting::new(self.voting_id(), self.drone(), INIT_AMOUNT);
				println!("Create the voting: {:?}", voting);
				schema.votings_mut().put(&self.voting_id(), voting);
				Ok(())
			} else {
				Err(Error::VotingAlreadyExists)?
			}
		}
	}

	impl Transaction for TxVote {
		fn verify(&self) -> bool {
			true
			//self.verify_signature(self.validator()) //??? what signature?
		}

		fn execute(&self, view: &mut Fork) -> ExecutionResult {
			let mut schema = SwarmSchema::new(view);

			let voting = match schema.voting(&self.voting_id()) {
				Some(val) => val,
				None => Err(Error::VotingNotFound)?,
			};

			//????????? check if validator exists

			//?????? Check if item is already voted
			let vote_collection = match schema.voting_with_actions(&self.voting_id()) {
				Some(val) => val,
				None => VoteCollection::new(self.voting_id(), Vec::new()),
			};

			for i in &vote_collection.vector() {
				if i.validator() == self.validator() {
					Err(Error::ValidatorAlreadyVoted)?;
				}
			}

			let vote_action = VoteAction::new(self.action_id(), self.validator(), self.voting_status());
			let vote_collection = vote_collection.add_action(vote_action);
			let mut vote_collections = schema.vote_collections_mut();
			vote_collections.put(&self.voting_id(), vote_collection);

			/*
			if self.voting_status() == true {
				let voting = voting.vote();
				println!("Validator voted for field spraying");
				let mut votings = schema.votings_mut();
				votings.put(&self.voting(), voting);
			} else {
				println!("Validator voted that filed is ok");
			} */
			Ok(())

		}
	}
}

pub mod api {
    use exonum::{
        api::{self, ServiceApiBuilder, ServiceApiState}, blockchain::Transaction,
        crypto::{Hash, PublicKey}, node::TransactionSend,
    };

    use schema::{SwarmSchema, Voting, VoteCollection};
    use transactions::VoteTransactions;

    #[derive(Debug, Clone)]
	pub struct VotingApi;

	#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
    pub struct VotingQuery {
        /// Public key of the queried wallet.
        pub voting_id: u64,
    }

    /// The structure returned by the REST API.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TransactionResponse {
        /// Hash of the transaction.
        pub tx_hash: Hash,
    }

    impl VotingApi {
        /// Endpoint for getting a single wallet.
        pub fn get_voting(state: &ServiceApiState, query: VotingQuery) -> api::Result<Voting> {
            let snapshot = state.snapshot();
            let schema = SwarmSchema::new(snapshot);
            schema
                .voting(&query.voting_id)
                .ok_or_else(|| api::Error::NotFound("\"Voting not found\"".to_owned()))
        }

        /// Endpoint for dumping all wallets from the storage.
        pub fn get_votings(state: &ServiceApiState, _query: ()) -> api::Result<Vec<Voting>> {
            let snapshot = state.snapshot();
            let schema = SwarmSchema::new(snapshot);
            let idx = schema.votings();
            let votings = idx.values().collect();
            Ok(votings)
        }

        pub fn get_vote_collections(state: &ServiceApiState, _query: ()) -> api::Result<Vec<VoteCollection>> {
            let snapshot = state.snapshot();
            let schema = SwarmSchema::new(snapshot);
            let idx = schema.votings_with_actions();
            let votings_with_actions = idx.values().collect();
            Ok(votings_with_actions)
        }

        /// Common processing for transaction-accepting endpoints.
        pub fn post_transaction(
            state: &ServiceApiState,
            query: VoteTransactions,
        ) -> api::Result<TransactionResponse> {
            let transaction: Box<dyn Transaction> = query.into();
            let tx_hash = transaction.hash();
            state.sender().send(transaction)?;
            Ok(TransactionResponse { tx_hash })
        }

        /// 'ServiceApiBuilder' facilitates conversion between transactions/read requests and REST
        /// endpoints; for example, it parses `POST`ed JSON into the binary transaction
        /// representation used in Exonum internally.
        pub fn wire(builder: &mut ServiceApiBuilder) {
            // Binds handlers to specific routes.
            builder
                .public_scope()
                .endpoint("v1/voting", Self::get_voting)
                .endpoint("v1/votings", Self::get_votings)
                .endpoint("v1/vote_collections", Self::get_vote_collections)
                .endpoint_mut("v1/votings", Self::post_transaction)
                .endpoint_mut("v1/votings/vote", Self::post_transaction);
        }
    }
}

pub mod service {
    use exonum::{
    api::ServiceApiBuilder, blockchain::{Service, Transaction, TransactionSet}, crypto::Hash,
    encoding, messages::RawTransaction, storage::Snapshot,
    };

    use api::VotingApi;
    use transactions::VoteTransactions;

    /// Service ID for the `Service` trait.
    pub const SERVICE_ID: u16 = 1;

    #[derive(Debug)]
    pub struct VotingService;

    impl Service for VotingService {
        fn service_name(&self) -> &'static str {
            "voting"
        }

        fn service_id(&self) -> u16 {
            SERVICE_ID
        }

        // Implement a method to deserialize transactions coming to the node.
        fn tx_from_raw(
            &self,
            raw: RawTransaction,
        ) -> Result<Box<dyn Transaction>, encoding::Error> {
            let tx = VoteTransactions::tx_from_raw(raw)?;
            Ok(tx.into())
        }

        // Hashes for the service tables that will be included into the state hash.
        // To simplify things, we don't have [Merkelized tables][merkle] in the service storage
        // for now, so we return an empty vector.
        //
        // [merkle]: https://exonum.com/doc/architecture/storage/#merklized-indices
        fn state_hash(&self, _: &dyn Snapshot) -> Vec<Hash> {
            vec![]
        }

        // Links the service api implementation to the Exonum.
        fn wire_api(&self, builder: &mut ServiceApiBuilder) {
            VotingApi::wire(builder);
        }
    }
}