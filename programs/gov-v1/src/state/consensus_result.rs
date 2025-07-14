use anchor_lang::prelude::*;

use crate::Ballot;

#[account]
#[derive(InitSpace)]
pub struct ConsensusResult {
    /// Ballot ID
    pub ballot_id: u64,
    /// Ballot
    pub ballot: Ballot,
}
