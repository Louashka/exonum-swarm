#![deny(missing_debug_implementations, unsafe_code, bare_trait_objects)]

#[macro_use]
extern crate exonum;
#[macro_use]
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub use schema::SwarmSchema;

pub mod api;
pub mod schema;
pub mod transactions;
pub mod voting;

use exonum::{
    api::ServiceApiBuilder, blockchain::{Service, Transaction, TransactionSet}, crypto::Hash,
    encoding::Error as EncodingError, helpers::fabric::{self, Context}, messages::RawTransaction,
    storage::Snapshot,
};

use transactions::VotingTransactions;

const VOTING_SERVICE_ID: u16 = 128;
pub const SERVICE_NAME: &str = "voting";
const INITIAL_AMOUNT: u8 = 0;


#[derive(Default, Debug)]
pub struct VotingService;

impl Service for VotingService {
    fn service_name(&self) -> &str {
        SERVICE_NAME
    }

    fn service_id(&self) -> u16 {
        VOTING_SERVICE_ID
    }

    fn state_hash(&self, view: &dyn Snapshot) -> Vec<Hash> {
        let schema = SwarmSchema::new(view);
        schema.state_hash()
    }

    fn tx_from_raw(&self, raw: RawTransaction) -> Result<Box<dyn Transaction>, EncodingError> {
        VotingTransactions::tx_from_raw(raw).map(Into::into)
    }

    fn wire_api(&self, builder: &mut ServiceApiBuilder) {
        api::VotingApi::wire(builder);
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