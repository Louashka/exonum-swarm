
use exonum::{
    api::{self, ServiceApiBuilder, ServiceApiState},
    blockchain::{self, BlockProof, Transaction, TransactionSet}, crypto::{Hash, PublicKey},
    helpers::Height, node::TransactionSend, storage::{ListProof, MapProof},
};

use transactions::VotingTransactions;
use voting::Voting;
use {SwarmSchema, VOTING_SERVICE_ID};


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


#[derive(Debug, Clone, Copy)]
pub struct VotingApi;

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
            general_schema.get_proof_to_service_table(VOTING_SERVICE_ID, 0);

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
