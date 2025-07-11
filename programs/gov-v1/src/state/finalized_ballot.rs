use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct FinalizedBallot {
    /// Ballot ID
    ballot_id: u64,
    /// The merkle root of the meta merkle tree
    meta_merkle_root: [u8; 32],
    /// SHA256 hash of JSON snapshot
    snapshot_hash: [u8; 32],
}
