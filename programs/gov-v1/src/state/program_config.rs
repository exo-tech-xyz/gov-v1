use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ProgramConfig {
    /// Authority allowed to update the config.
    authority: Pubkey,
    /// Operators whitelisted to participate in voting.
    whitelisted_operators: [Pubkey; 64],
    /// Min. percentage of votes required to finalize a ballot.
    min_consensus_threshold_bps: u16,
    /// Admin allowed to decide the winning ballot if vote expires before consensus.
    tie_breaker_admin: Pubkey,
    /// ID for next BallotBox
    next_ballot_id: u64,
}
