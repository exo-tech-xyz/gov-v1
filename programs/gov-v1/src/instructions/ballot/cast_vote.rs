use anchor_lang::prelude::*;

use crate::{error::ErrorCode, Ballot, BallotBox, BallotTally, OperatorVote, ProgramConfig};

#[derive(Accounts)]
pub struct CastVote<'info> {
    pub operator: Signer<'info>,
    #[account(mut)]
    pub ballot_box: Box<Account<'info, BallotBox>>,
    pub program_config: Box<Account<'info, ProgramConfig>>,
}

pub fn handler(ctx: Context<CastVote>, ballot: Ballot) -> Result<()> {
    let operator = &ctx.accounts.operator.key();
    let ballot_box = &mut ctx.accounts.ballot_box;
    let program_config = &ctx.accounts.program_config;
    program_config.contains_operator(operator)?;

    let clock = Clock::get()?;
    require!(
        !ballot_box.has_vote_expired(clock.unix_timestamp),
        ErrorCode::VotingExpired
    );
    require!(ballot.meta_merkle_root != [0; 32], ErrorCode::InvalidBallot);

    let operator_vote = ballot_box
        .operator_votes
        .iter()
        .find(|vote| vote.operator == *operator);
    require!(operator_vote.is_none(), ErrorCode::OperatorHasVoted);

    // Look for ballot within ballot_tallies first. If ballot already exists,
    // increment vote on ballot.
    let mut ballot_index = 0;
    let mut found = false;
    let mut tally = 0;
    for ballot_tally in &mut ballot_box.ballot_tallies {
        if ballot_tally.ballot == ballot {
            ballot_tally.tally = ballot_tally.tally.checked_add(1).unwrap();
            ballot_index = ballot_tally.index;
            tally = ballot_tally.tally;
            found = true;
            break;
        }
    }

    // If ballot is new, create a new BallotTally.
    if !found {
        let new_ballot_tally = BallotTally {
            index: ballot_box.ballot_tallies.len().try_into().unwrap(),
            ballot: ballot.clone(),
            tally: 1,
        };
        tally = 1;
        ballot_index = new_ballot_tally.index;
        ballot_box.ballot_tallies.push(new_ballot_tally);
    }

    // Create a new operator vote for the ballot tally.
    let new_operator_vote = OperatorVote {
        operator: ctx.accounts.operator.key(),
        slot_voted: clock.slot,
        ballot_index,
    };
    ballot_box.operator_votes.push(new_operator_vote);

    // Set winning ballot if consensus threshold is reached (for first time).
    if !ballot_box.has_consensus_reached() {
        let tally_bps =
            u64::from(tally) * 10000 / (program_config.whitelisted_operators.len() as u64);
        if tally_bps >= ballot_box.min_consensus_threshold_bps.into() {
            ballot_box.slot_consensus_reached = clock.slot;
            ballot_box.winning_ballot = ballot;
        }
    }

    Ok(())
}
