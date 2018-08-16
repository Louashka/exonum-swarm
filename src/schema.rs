
use exonum::{
    crypto::{Hash, PublicKey}, storage::{Fork, ProofListIndex, ProofMapIndex, Snapshot},
};

use voting::{Voting, VotingAction};
use INITIAL_AMOUNT;


#[derive(Debug)]
pub struct SwarmSchema<T> {
    view: T,
}

impl<T> AsMut<T> for SwarmSchema<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.view
    }
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

