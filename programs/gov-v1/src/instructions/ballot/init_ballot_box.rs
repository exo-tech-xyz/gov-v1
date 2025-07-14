use anchor_lang::prelude::*;

use crate::{BallotBox, ProgramConfig};

#[derive(Accounts)]
pub struct InitBallotBox<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub operator: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"BallotBox".as_ref(),
            &program_config.next_ballot_id.to_le_bytes()
        ],
        bump,
        payer = payer,
        space = 8 + BallotBox::INIT_SPACE
    )]
    pub ballot_box: Box<Account<'info, BallotBox>>,
    #[account(mut)]
    pub program_config: Box<Account<'info, ProgramConfig>>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitBallotBox>) -> Result<()> {
    let program_config = &mut ctx.accounts.program_config;
    program_config.contains_operator(&ctx.accounts.operator.key())?;

    let ballot_box = &mut ctx.accounts.ballot_box;
    ballot_box.ballot_id = program_config.next_ballot_id;
    ballot_box.bump = ctx.bumps.ballot_box;

    let clock = Clock::get()?;
    ballot_box.epoch = clock.epoch;
    ballot_box.slot_created = clock.slot;
    ballot_box.min_consensus_threshold_bps = program_config.min_consensus_threshold_bps;
    ballot_box.vote_expiry_timestamp = clock
        .unix_timestamp
        .checked_add(program_config.vote_duration)
        .unwrap();

    // Increment for next ballot box
    program_config.next_ballot_id = program_config.next_ballot_id.checked_add(1).unwrap();

    Ok(())
}
