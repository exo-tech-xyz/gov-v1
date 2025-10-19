use anchor_lang::prelude::*;

use crate::{error::ErrorCode, BallotBox, ProgramConfig};

#[derive(Accounts)]
pub struct SetTieBreaker<'info> {
    pub tie_breaker_admin: Signer<'info>,
    #[account(mut)]
    pub ballot_box: Box<Account<'info, BallotBox>>,
    #[account(has_one = tie_breaker_admin)]
    pub program_config: Box<Account<'info, ProgramConfig>>,
}

pub fn handler(ctx: Context<SetTieBreaker>, ballot_index: u8) -> Result<()> {
    let ballot_box = &mut ctx.accounts.ballot_box;
    let clock = Clock::get()?;
    require!(
        ballot_box.has_vote_expired(clock.unix_timestamp),
        ErrorCode::VotingNotExpired
    );
    require!(
        !ballot_box.has_consensus_reached(),
        ErrorCode::ConsensusReached
    );
    require!(
        (ballot_index as usize) < ballot_box.ballot_tallies.len(),
        ErrorCode::InvalidBallotIndex
    );

    ballot_box.slot_consensus_reached = clock.slot;
    ballot_box.winning_ballot = ballot_box.ballot_tallies[ballot_index as usize]
        .ballot
        .clone();

    Ok(())
}
