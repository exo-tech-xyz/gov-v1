use anchor_lang::prelude::*;

use crate::Ballot;

#[account]
#[derive(InitSpace, Debug)]
pub struct ConsensusResult {
    /// Ballot ID
    pub ballot_id: u64,
    /// Ballot
    pub ballot: Ballot,
}

impl ConsensusResult {
    pub fn pda(ballot_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"ConsensusResult", &ballot_id.to_le_bytes()], &crate::ID)
    }
}
