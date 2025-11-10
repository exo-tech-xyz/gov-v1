use anchor_lang::prelude::*;

use crate::{error::ErrorCode, BallotBox, ProgramConfig};

#[derive(Accounts)]
#[instruction(snapshot_slot: u64)]
pub struct InitBallotBox<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    // TODO: Enforce check that signer is a PDA of gov contract program
    pub govcontract: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"BallotBox".as_ref(),
            &snapshot_slot.to_le_bytes()
        ],
        bump,
        payer = payer,
        space = 8 + BallotBox::INIT_SPACE
    )]
    pub ballot_box: Box<Account<'info, BallotBox>>,
    pub program_config: Box<Account<'info, ProgramConfig>>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitBallotBox>, snapshot_slot: u64) -> Result<()> {
    let clock = Clock::get()?;

    // Check that snapshot slot is greater than current slot to 
    // allow sufficient lead time for snapshot.
    require!(snapshot_slot > clock.slot, ErrorCode::InvalidSnapshotSlot);

    let program_config = &ctx.accounts.program_config;
    let ballot_box = &mut ctx.accounts.ballot_box;

    ballot_box.bump = ctx.bumps.ballot_box;
    ballot_box.epoch = clock.epoch;
    ballot_box.slot_created = clock.slot;
    ballot_box.snapshot_slot = snapshot_slot;
    ballot_box.min_consensus_threshold_bps = program_config.min_consensus_threshold_bps;
    ballot_box.vote_expiry_timestamp = clock
        .unix_timestamp
        .checked_add(program_config.vote_duration)
        .unwrap();
    ballot_box.voter_list = program_config.whitelisted_operators.clone();
    ballot_box.tie_breaker_consensus = false;

    Ok(())
}
