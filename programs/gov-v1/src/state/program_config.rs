use crate::error::ErrorCode;
use std::collections::HashSet;

use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct ProgramConfig {
    /// Authority allowed to update the config.
    pub authority: Pubkey,
    /// Operators whitelisted to participate in voting.
    #[max_len(64)]
    pub whitelisted_operators: Vec<Pubkey>,
    /// Min. percentage of votes required to finalize a ballot. Used during BallotBox creation.
    pub min_consensus_threshold_bps: u16,
    /// Admin allowed to decide the winning ballot if vote expires before consensus.
    pub tie_breaker_admin: Pubkey,
    /// ID for next BallotBox
    pub next_ballot_id: u64,
    /// Duration for which ballot box will be opened for voting.
    pub vote_duration: i64,
}

impl ProgramConfig {
    pub fn remove_operators(&mut self, operators_to_remove: Option<Vec<Pubkey>>) {
        if let Some(operators) = operators_to_remove {
            let remove_set: HashSet<Pubkey> = operators.into_iter().collect();
            self.whitelisted_operators
                .retain(|op| !remove_set.contains(op));
        }
    }

    pub fn add_operators(&mut self, operators_to_add: Option<Vec<Pubkey>>) {
        if let Some(operators) = operators_to_add {
            let existing: HashSet<Pubkey> = self.whitelisted_operators.iter().cloned().collect();
            for op in operators {
                if !existing.contains(&op) {
                    self.whitelisted_operators.push(op);
                }
            }
        }
    }

    pub fn contains_operator(&self, operator: &Pubkey) -> Result<()> {
        require!(
            self.whitelisted_operators.contains(operator),
            ErrorCode::OperatorNotWhitelisted
        );
        Ok(())
    }
}
