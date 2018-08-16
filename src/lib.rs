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
		 crypto::{Hash, PublicKey}, storage::{Fork, ProofListIndex, ProofMapIndex, Snapshot},
	};

	const INITIAL_AMOUNT: u8 = 0;

	encoding_struct! {
		struct VotingAction {
	        action_id:          u64,
	        validator:          &PublicKey,
	        voting_status:      bool,
	    }
	}

	encoding_struct! {
		struct Voting {
	        pub_key:          	&PublicKey,
	        drone:              &PublicKey,
	        actions:            Vec<VotingAction>,
	        amount:             u8,
	        history_len:        u64,
	        history_hash:       &Hash,
	    }
	}

	impl Voting {
	    pub fn set_amount(self, actions: Vec<VotingAction>, amount: u8, history_hash: &Hash) -> Self {
	        Self::new(
	            self.pub_key(),
	            self.drone(),
	            actions,
	            amount,
	            self.history_len() + 1,
	            history_hash,
	        )
	    }
	}

	#[derive(Debug)]
	pub struct SwarmSchema<T> {
	    view: T,
	}

	impl<T> SwarmSchema<T>
	where
	    T: AsRef<dyn Snapshot>,
	{
	    pub fn new(view: T) -> Self {
	        SwarmSchema { view }
	    }

	    pub fn votings(&self) -> ProofMapIndex<&T, PublicKey, Voting> {
	        ProofMapIndex::new("swarm.votings", &self.view)
	    }

	    pub fn voting_history(&self, pub_key: &PublicKey) -> ProofListIndex<&T, Hash> {
	        ProofListIndex::new_in_family("swarm.voting_history", pub_key, &self.view)
	    }

	    pub fn voting(&self, pub_key: &PublicKey) -> Option<Voting> {
	        self.votings().get(pub_key)
	    }

	    pub fn state_hash(&self) -> Vec<Hash> {
	        vec![self.votings().merkle_root()]
	    }
	}

	impl<'a> SwarmSchema<&'a mut Fork> {

	    pub fn votings_mut(&mut self) -> ProofMapIndex<&mut Fork, PublicKey, Voting> {
	        ProofMapIndex::new("swarm.votings", &mut self.view)
	    }

	    pub fn voting_history_mut(
	        &mut self,
	        pub_key: &PublicKey,
	    ) -> ProofListIndex<&mut Fork, Hash> {
	        ProofListIndex::new_in_family("swarm.voting_history", pub_key, &mut self.view)
	    }

	    pub fn create_voting(&mut self, key: &PublicKey, drone: &PublicKey, transaction: &Hash) {
	        let voting = {
	            let mut history = self.voting_history_mut(key);
	            history.push(*transaction);
	            let history_hash = history.merkle_root();
	            Voting::new(key, drone, Vec::new(), INITIAL_AMOUNT, history.len(), &history_hash)
	        };
	        self.votings_mut().put(key, voting);
	    }

	    pub fn vote(&mut self, voting: Voting, action: VotingAction, transaction: &Hash){
	        let voting = {
	            let mut history = self.voting_history_mut(voting.pub_key());
	            history.push(*transaction);
	            let history_hash = history.merkle_root();
	            
	            let mut amount = voting.amount();
	            if action.voting_status() {
	            	amount = amount + 1;
	            }

	            let mut actions = voting.actions();
	            actions.push(action);
	            voting.set_amount(actions, amount, &history_hash)
	        };
	        self.votings_mut().put(voting.pub_key(), voting.clone());
	    }

	}
}

pub mod transactions {
	use exonum::crypto::PublicKey;

    use service::SERVICE_ID;

    transactions! {
		pub VotingTransactions {
			const SERVICE_ID = SERVICE_ID;

			struct CreateVoting {
	            pub_key:  		&PublicKey,
	            drone:      	&PublicKey,
	        }

	        struct Vote {
	            action_id: 		u64,
	            pub_key: 		&PublicKey,
	            validator: 		&PublicKey,
	            voting_status: 	bool,
	            seed: 			u64,
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
        crypto::{CryptoHash},
    };

    use errors::Error;
    use schema::{SwarmSchema, VotingAction};
    use transactions::{CreateVoting, Vote};

    impl Transaction for CreateVoting {
	    fn verify(&self) -> bool {
	    	true
	        //self.verify_signature(self.drone())
	    }

	    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
	        let mut schema = SwarmSchema::new(fork);
	        let pub_key = self.pub_key();
	        let hash = self.hash();

	        if schema.voting(pub_key).is_none() {
	            let drone = self.drone();
	            schema.create_voting(pub_key, drone, &hash);
	            Ok(())
	        } else {
	            Err(Error::VotingAlreadyExists)?
	        }
	    }
	}

	impl Transaction for Vote {
	    fn verify(&self) -> bool {
	    	true
	        //self.verify_signature(self.validator())
	    }

	    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
	        let mut schema = SwarmSchema::new(fork);

	        let action_id = self.action_id();
	        let pub_key = self.pub_key();
	        let validator = self.validator();
	        let voting_status = self.voting_status();
	        let hash = self.hash();

	        let voting = schema.voting(pub_key).ok_or(Error::VotingNotFound)?;
	        let voting_actions = voting.actions();

	        for i in &voting_actions {
	            if i.validator() == validator {
	                Err(Error::ValidatorAlreadyVoted)?
	            }
	        }

	        let voting_action = VotingAction::new(action_id, validator, voting_status);

	        schema.vote(voting, voting_action, &hash);

	        Ok(())
	    }
	}
}

pub mod api {
    use exonum::{
        api::{self, ServiceApiBuilder, ServiceApiState},
	    blockchain::{self, BlockProof, Transaction, TransactionSet}, crypto::{Hash, PublicKey},
	    helpers::Height, node::TransactionSend, storage::{ListProof, MapProof},
    };

    use schema::{SwarmSchema, Voting};
    use transactions::VotingTransactions;
    use service::SERVICE_ID;

    #[derive(Debug, Clone)]
	pub struct VotingApi;

	#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
    pub struct VotingQuery {
        pub pub_key: PublicKey,
    }

    /// The structure returned by the REST API.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TransactionResponse {
        /// Hash of the transaction.
        pub tx_hash: Hash,
    }

    #[derive(Debug, Serialize, Deserialize)]
	pub struct VotingProof {
	    /// Proof to the whole database table.
	    pub to_table: MapProof<Hash, Hash>,
	    pub to_voting: MapProof<PublicKey, Voting>,
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct VotingHistory {
	    pub proof: ListProof<Hash>,
	    pub transactions: Vec<VotingTransactions>,
	}

	/// Wallet information.
	#[derive(Debug, Serialize, Deserialize)]
	pub struct VotingInfo {
	    pub block_proof: BlockProof,
	    pub voting_proof: VotingProof,
	    pub voting_history: Option<VotingHistory>,
	}

    impl VotingApi {

        pub fn get_voting(state: &ServiceApiState, query: VotingQuery) -> api::Result<Voting> {
            let snapshot = state.snapshot();
            let schema = SwarmSchema::new(snapshot);
            schema
                .voting(&query.pub_key)
                .ok_or_else(|| api::Error::NotFound("\"Voting not found\"".to_owned()))
        }


        pub fn get_votings(state: &ServiceApiState, _query: ()) -> api::Result<Vec<Voting>> {
            let snapshot = state.snapshot();
            let schema = SwarmSchema::new(snapshot);
            let idx = schema.votings();
            let votings = idx.values().collect();
            Ok(votings)
        }

        pub fn voting_info(state: &ServiceApiState, query: VotingQuery) -> api::Result<VotingInfo> {
	        let snapshot = state.snapshot();
	        let general_schema = blockchain::Schema::new(&snapshot);
	        let swarm_schema = SwarmSchema::new(&snapshot);

	        let max_height = general_schema.block_hashes_by_height().len() - 1;

	        let block_proof = general_schema
	            .block_and_precommits(Height(max_height))
	            .unwrap();

	        let to_table: MapProof<Hash, Hash> =
	            general_schema.get_proof_to_service_table(SERVICE_ID, 0);

	        let to_voting: MapProof<PublicKey, Voting> =
	            swarm_schema.votings().get_proof(query.pub_key);

	        let voting_proof = VotingProof {
	            to_table,
	            to_voting,
	        };

	        let voting = swarm_schema.voting(&query.pub_key);

	        let voting_history = voting.map(|_| {
	            let history = swarm_schema.voting_history(&query.pub_key);
	            let proof = history.get_range_proof(0, history.len());

	            let transactions: Vec<VotingTransactions> = history
	                .iter()
	                .map(|record| general_schema.transactions().get(&record).unwrap())
	                .map(|raw| VotingTransactions::tx_from_raw(raw).unwrap())
	                .collect::<Vec<_>>();

	            VotingHistory {
	                proof,
	                transactions,
	            }
	        });

	        Ok(VotingInfo {
	            block_proof,
	            voting_proof,
	            voting_history,
	        })
	    }

        /// Common processing for transaction-accepting endpoints.
        pub fn post_transaction(
            state: &ServiceApiState,
            query: VotingTransactions,
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
                .endpoint("v1/votings/info", Self::voting_info)
                .endpoint_mut("v1/votings", Self::post_transaction)
                .endpoint_mut("v1/votings/vote", Self::post_transaction);
        }
    }
}

pub mod service {
    use exonum::{
    api::ServiceApiBuilder, blockchain::{Service, Transaction, TransactionSet}, crypto::Hash,
    encoding::Error as EncodingError, helpers::fabric::{self, Context}, messages::RawTransaction,
    storage::Snapshot,
    };

    use api::VotingApi;
    use transactions::VotingTransactions;
    use schema::SwarmSchema;

    /// Service ID for the `Service` trait.
    pub const SERVICE_ID: u16 = 128;
    pub const SERVICE_NAME: &str = "voting";

    #[derive(Debug)]
    pub struct VotingService;

    impl Service for VotingService {
        fn service_name(&self) -> &str {
            SERVICE_NAME
        }

        fn service_id(&self) -> u16 {
            SERVICE_ID
        }

        fn state_hash(&self, view: &dyn Snapshot) -> Vec<Hash> {
	        let schema = SwarmSchema::new(view);
	        schema.state_hash()
	    }

	    fn tx_from_raw(&self, raw: RawTransaction) -> Result<Box<dyn Transaction>, EncodingError> {
	        VotingTransactions::tx_from_raw(raw).map(Into::into)
	    }

	    fn wire_api(&self, builder: &mut ServiceApiBuilder) {
	        VotingApi::wire(builder);
	    }
    }

    #[derive(Debug)]
	pub struct ServiceFactory;

	impl fabric::ServiceFactory for ServiceFactory {
	    fn service_name(&self) -> &str {
	        SERVICE_NAME
	    }

	    fn make_service(&mut self, _: &Context) -> Box<dyn Service> {
	        Box::new(VotingService)
	    }
	}
}