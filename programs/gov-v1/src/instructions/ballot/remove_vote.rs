use anchor_lang::prelude::*;

use crate::{error::ErrorCode, BallotBox, ProgramConfig};

#[derive(Accounts)]
pub struct RemoveVote<'info> {
    pub operator: Signer<'info>,
    #[account(mut)]
    pub ballot_box: Box<Account<'info, BallotBox>>,
    pub program_config: Box<Account<'info, ProgramConfig>>,
}

pub fn handler(ctx: Context<RemoveVote>) -> Result<()> {
    let operator = &ctx.accounts.operator.key();
    let ballot_box = &mut ctx.accounts.ballot_box;
    let program_config = &ctx.accounts.program_config;
    program_config.contains_operator(operator)?;

    require!(
        !ballot_box.has_vote_expired(Clock::get()?.unix_timestamp),
        ErrorCode::VotingExpired
    );
    require!(
        !ballot_box.has_consensus_reached(),
        ErrorCode::ConsensusReached
    );

    let operator_vote_idx = ballot_box
        .operator_votes
        .iter()
        .position(|vote| vote.operator == *operator);

    // Get operator's ballot index and remove operator from OperatorVotes.
    let ballot_index: u8;
    if let Some(idx) = operator_vote_idx {
        ballot_index = ballot_box.operator_votes[idx].ballot_index;
        ballot_box.operator_votes.remove(idx);
    } else {
        return err!(ErrorCode::OperatorHasNotVoted);
    }

    // Decrement tally on BallotTally and remove from vec if new tally is zero.
    let ballot_tally = &mut ballot_box.ballot_tallies[ballot_index as usize];
    ballot_tally.tally = ballot_tally.tally.checked_sub(1).unwrap();
    if ballot_tally.tally == 0 {
        ballot_box.ballot_tallies.remove(ballot_index as usize);
    }

    Ok(())
}
