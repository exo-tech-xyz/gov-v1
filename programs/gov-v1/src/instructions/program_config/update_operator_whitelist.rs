use anchor_lang::prelude::*;

use crate::ProgramConfig;

#[derive(Accounts)]
pub struct UpdateOperatorWhitelist<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority
    )]
    pub program_config: Box<Account<'info, ProgramConfig>>,
}

pub fn handler(
    ctx: Context<UpdateOperatorWhitelist>,
    operators_to_add: Option<Vec<Pubkey>>,
    operators_to_remove: Option<Vec<Pubkey>>,
) -> Result<()> {
    let program_config = &mut ctx.accounts.program_config;
    program_config.remove_operators(operators_to_remove);
    program_config.add_operators(operators_to_add);

    Ok(())
}
