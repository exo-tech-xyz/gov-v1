use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct BallotBox {
    /// ID
    ballot_id: u64,
    /// The epoch this ballot box is for
    epoch: u64,
    /// Bump seed for the PDA
    bump: u8,
    /// Slot when this ballot box was created
    slot_created: u64,
    /// Slot when consensus was reached
    slot_consensus_reached: u64,
    /// Reserved space
    reserved: [u8; 128],
    /// Number of operators that have voted
    operators_voted: u8,
    /// Number of unique ballots
    unique_ballots: u8,
    /// The ballot that got at least min_consensus_threshold of votes
    winning_ballot: Ballot,
    /// Operator votes
    operator_votes: [OperatorVote; 64],
    /// Mapping of ballots votes to stake weight
    ballot_tallies: [BallotTally; 64],
    /// Timestamp when voting ends. Tie breaker admin will decide the results
    /// if no consensus is reached by then.
    vote_expiry_timestamp: i64,
}

/// Inner struct of BallotBox
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct Ballot {
    /// The merkle root of the meta merkle tree
    meta_merkle_root: [u8; 32],
    /// SHA256 hash of JSON snapshot
    snapshot_hash: [u8; 32],
    /// Whether the ballot is valid
    is_valid: bool,
    /// Reserved space
    reserved: [u8; 63],
}

/// Inner struct of BallotBox
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct OperatorVote {
    /// The operator that cast the vote
    operator: Pubkey,
    /// The slot the operator voted
    slot_voted: u64,
    /// The index of the ballot in the ballot_tallies
    ballot_index: u8,
    /// Reserved space
    reserved: [u8; 64],
}

/// Inner struct of BallotBox
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct BallotTally {
    /// Index of the tally within the ballot_tallies
    index: u16,
    /// The ballot being tallied
    ballot: Ballot,
    /// The number of votes for this ballot. Each vote is equally weighted.
    tally: u8,
    reserved: [u8; 64],
}
