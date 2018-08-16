

use exonum::crypto::{Hash, PublicKey};

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

	