use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Operator not whitelisted")]
    OperatorNotWhitelisted,
    #[msg("Operator has voted")]
    OperatorHasVoted,
    #[msg("Operator has not voted")]
    OperatorHasNotVoted,
    #[msg("Voting has expired")]
    VotingExpired,
    #[msg("Voting not expired")]
    VotingNotExpired,
    #[msg("Consensus has reached")]
    ConsensusReached,
    #[msg("Consensus not reached")]
    ConsensusNotReached,
    #[msg("Invalid ballot")]
    InvalidBallot,
}
