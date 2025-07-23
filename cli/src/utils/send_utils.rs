use anchor_client::{
    anchor_lang::system_program,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signature},
        signer::Signer,
    },
    ClientError, Program,
};
use gov_v1::{accounts, instruction, Ballot, MetaMerkleLeaf, ProgramConfig, StakeMerkleLeaf};
use solana_sdk::instruction::Instruction;
use solana_sdk::{compute_budget::ComputeBudgetInstruction, transaction::Transaction};

pub struct TxSender<'a> {
    pub program: &'a Program<&'a Keypair>,
    pub micro_lamports: Option<u64>,
    pub payer: &'a Keypair,
    pub authority: &'a Keypair,
}

impl<'a> TxSender<'a> {
    pub fn send(&self, ixs: Vec<Instruction>) -> Result<Signature, ClientError> {
        send_with_anchor(
            ixs,
            self.micro_lamports,
            &[self.payer, self.authority],
            self.program,
        )
    }

    pub fn send_with_signers(
        &self,
        ixs: Vec<Instruction>,
        signers: &[&Keypair],
    ) -> Result<Signature, ClientError> {
        send_with_anchor(ixs, self.micro_lamports, signers, self.program)
    }
}

/// Sends an Anchor request manually, ensuring proper setup and signing.
fn send_with_anchor(
    mut ixs: Vec<Instruction>,
    micro_lamports: Option<u64>,
    signers: &[&Keypair],
    program: &Program<&Keypair>,
) -> Result<Signature, ClientError> {
    let payer = program.payer();
    let blockhash = program.rpc().get_latest_blockhash()?;

    if let Some(lamports) = micro_lamports {
        ixs.insert(
            0,
            ComputeBudgetInstruction::set_compute_unit_price(lamports),
        );
    }

    let tx = Transaction::new_signed_with_payer(&ixs, Some(&payer), signers, blockhash);
    program
        .rpc()
        .send_and_confirm_transaction(&tx)
        .map_err(ClientError::SolanaClientError)
}

pub fn send_init_program_config(tx_sender: &TxSender) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::InitProgramConfig {
            payer: tx_sender.program.payer(),
            authority: tx_sender.authority.pubkey(),
            program_config: ProgramConfig::pda().0,
            system_program: system_program::ID,
        })
        .args(instruction::InitProgramConfig {})
        .instructions()?;

    tx_sender.send(ixs)
}

pub fn send_update_operator_whitelist(
    tx_sender: &TxSender,
    operators_to_add: Option<Vec<Pubkey>>,
    operators_to_remove: Option<Vec<Pubkey>>,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::UpdateOperatorWhitelist {
            authority: tx_sender.authority.pubkey(),
            program_config: ProgramConfig::pda().0,
        })
        .args(instruction::UpdateOperatorWhitelist {
            operators_to_add,
            operators_to_remove,
        })
        .instructions()?;

    tx_sender.send(ixs)
}

pub fn send_update_program_config(
    tx_sender: &TxSender,
    new_authority: Option<&Keypair>,
    min_consensus_threshold_bps: Option<u16>,
    tie_breaker_admin: Option<Pubkey>,
    vote_duration: Option<i64>,
) -> Result<Signature, ClientError> {
    let mut signers = vec![tx_sender.payer, tx_sender.authority];
    let mut accounts = accounts::UpdateProgramConfig {
        authority: tx_sender.authority.pubkey(),
        program_config: ProgramConfig::pda().0,
        new_authority: None,
    };

    if let Some(kp) = new_authority {
        accounts.new_authority = Some(kp.pubkey());
        signers.push(&kp);
    }

    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts)
        .args(instruction::UpdateProgramConfig {
            min_consensus_threshold_bps,
            tie_breaker_admin,
            vote_duration,
        })
        .instructions()?;

    tx_sender.send_with_signers(ixs, &signers)
}

pub fn send_cast_vote(
    tx_sender: &TxSender,
    ballot_box: Pubkey,
    ballot: Ballot,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::CastVote {
            operator: tx_sender.authority.pubkey(),
            ballot_box,
            program_config: ProgramConfig::pda().0,
        })
        .args(instruction::CastVote { ballot })
        .instructions()?;

    tx_sender.send(ixs)
}

pub fn send_init_ballot_box(
    tx_sender: &TxSender,
    ballot_box: Pubkey,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::InitBallotBox {
            payer: tx_sender.payer.pubkey(),
            operator: tx_sender.authority.pubkey(),
            ballot_box,
            program_config: ProgramConfig::pda().0,
            system_program: system_program::ID,
        })
        .args(instruction::InitBallotBox {})
        .instructions()?;

    tx_sender.send(ixs)
}

pub fn send_remove_vote(
    tx_sender: &TxSender,
    ballot_box: Pubkey,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::RemoveVote {
            operator: tx_sender.authority.pubkey(),
            ballot_box,
            program_config: ProgramConfig::pda().0,
        })
        .args(instruction::RemoveVote {})
        .instructions()?;

    tx_sender.send(ixs)
}

pub fn send_finalize_ballot(
    tx_sender: &TxSender,
    ballot_box: Pubkey,
    consensus_result: Pubkey,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::FinalizeBallot {
            payer: tx_sender.payer.pubkey(),
            ballot_box,
            consensus_result,
            system_program: system_program::ID,
        })
        .args(instruction::FinalizeBallot {})
        .instructions()?;

    tx_sender.send_with_signers(ixs, &[tx_sender.payer])
}

pub fn send_set_tie_breaker(
    tx_sender: &TxSender,
    ballot_box: Pubkey,
    ballot_index: u8,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::SetTieBreaker {
            tie_breaker_admin: tx_sender.authority.pubkey(),
            ballot_box,
            program_config: ProgramConfig::pda().0,
        })
        .args(instruction::SetTieBreaker { ballot_index })
        .instructions()?;

    tx_sender.send(ixs)
}

pub fn send_init_meta_merkle_proof(
    tx_sender: &TxSender,
    meta_merkle_proof_pda: Pubkey,
    consensus_result: Pubkey,
    meta_merkle_leaf: MetaMerkleLeaf,
    meta_merkle_proof: Vec<[u8; 32]>,
    close_timestamp: i64,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::InitMetaMerkleProof {
            payer: tx_sender.payer.pubkey(),
            merkle_proof: meta_merkle_proof_pda,
            consensus_result,
            system_program: system_program::ID,
        })
        .args(instruction::InitMetaMerkleProof {
            meta_merkle_leaf,
            meta_merkle_proof,
            close_timestamp,
        })
        .instructions()?;

    tx_sender.send(ixs)
}

pub fn send_verify_merkle_proof(
    tx_sender: &TxSender,
    consensus_result: Pubkey,
    meta_merkle_proof: Pubkey,
    stake_merkle_proof: Option<Vec<[u8; 32]>>,
    stake_merkle_leaf: Option<StakeMerkleLeaf>,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::VerifyMerkleProof {
            consensus_result,
            meta_merkle_proof,
        })
        .args(instruction::VerifyMerkleProof {
            stake_merkle_proof,
            stake_merkle_leaf,
        })
        .instructions()?;

    tx_sender.send(ixs)
}

pub fn send_close_meta_merkle_proof(
    tx_sender: &TxSender,
    meta_merkle_proof: Pubkey,
) -> Result<Signature, ClientError> {
    let ixs = tx_sender
        .program
        .request()
        .accounts(accounts::CloseMetaMerkleProof {
            payer: tx_sender.payer.pubkey(),
            meta_merkle_proof,
            system_program: system_program::ID,
        })
        .args(instruction::CloseMetaMerkleProof {})
        .instructions()?;

    tx_sender.send(ixs)
}
