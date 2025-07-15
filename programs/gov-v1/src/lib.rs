pub mod constants;
pub mod error;
pub mod instructions;
pub mod merkle_helper;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("HQrwhDzMa7dEnUi2Nku925yeAAxioFhqpLMpQ4g6Zh5N");

#[program]
pub mod gov_v1 {
    use super::*;

    pub fn init_program_config(ctx: Context<InitProgramConfig>) -> Result<()> {
        init_program_config::handler(ctx)
    }
}
