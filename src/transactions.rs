#![allow(bare_trait_objects)]

use exonum::{
    blockchain::{ExecutionError, ExecutionResult, Transaction}, crypto::{CryptoHash, PublicKey},
    messages::Message, storage::Fork,
};

use voting::VotingAction;
use schema::SwarmSchema;
use VOTING_SERVICE_ID;

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

transactions! {
	pub VotingTransactions {
		const SERVICE_ID = VOTING_SERVICE_ID;

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
