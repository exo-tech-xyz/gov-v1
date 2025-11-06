use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke_signed},
};

use crate::{error::ErrorCode, BallotBox, ConsensusResult};

const GOVCONTRACT_PROGRAM_ID: Pubkey = pubkey!("3GBS7ZjQV5cKfsazbA2CSGm8kVQjjT6ow9XxZtSxRH3G");

#[derive(Accounts)]
pub struct FinalizeBallot<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub ballot_box: Box<Account<'info, BallotBox>>,
    #[account(
        init,
        seeds = [
            b"ConsensusResult".as_ref(),
            &ballot_box.ballot_id.to_le_bytes()
        ],
        bump,
        payer = payer,
        space = 8 + ConsensusResult::INIT_SPACE
    )]
    pub consensus_result: Box<Account<'info, ConsensusResult>>,

    #[account(mut)]
    pub proposal: UncheckedAccount<'info>,

    #[account(address = GOVCONTRACT_PROGRAM_ID)]
    pub govcontract_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<FinalizeBallot>) -> Result<()> {
    let ballot_box = &ctx.accounts.ballot_box;
    require!(
        ballot_box.has_consensus_reached(),
        ErrorCode::ConsensusNotReached
    );

    let consensus_result = &mut ctx.accounts.consensus_result;
    consensus_result.ballot_id = ballot_box.ballot_id;
    consensus_result.ballot = ballot_box.winning_ballot.clone();

    // CPI to add merkle tree
    let cpi_accounts = vec![
        ctx.accounts.consensus_result.to_account_info(),
        ctx.accounts.proposal.to_account_info(),
    ];
    let seeds: &[&[u8]] = &[
        b"ConsensusResult".as_ref(),
        &ballot_box.ballot_id.to_le_bytes(),
        &[ctx.bumps.consensus_result],
    ];
    let signer = &[&seeds[..]];

    let mut data: Vec<u8> = vec![235, 31, 120, 49, 53, 9, 197, 147];
    data.extend_from_slice(&ballot_box.ballot_id.to_le_bytes()[..]);

    data.extend_from_slice(&ballot_box.winning_ballot.clone().meta_merkle_root[..]);

    let instruction = Instruction {
        program_id: ctx.accounts.govcontract_program.to_account_info().key(),
        data,
        accounts: vec![
            AccountMeta::new(ctx.accounts.consensus_result.key(), true),
            AccountMeta::new(ctx.accounts.proposal.to_account_info().key(), false),
        ],
    };
    invoke_signed(&instruction, &cpi_accounts, signer)?;
    Ok(())
}
