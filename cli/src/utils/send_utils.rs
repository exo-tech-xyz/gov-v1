use anchor_client::{
    anchor_lang::system_program,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signature},
        signer::Signer,
    },
    ClientError, Program,
};
use gov_v1::{accounts, instruction, Ballot, MetaMerkleLeaf, StakeMerkleLeaf};

pub fn send_init_program_config(
    program: &Program<&Keypair>,
    authority: &Keypair,
    program_config: Pubkey,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::InitProgramConfig {
            payer: program.payer(),
            authority: authority.pubkey(),
            program_config,
            system_program: system_program::ID,
        })
        .args(instruction::InitProgramConfig {})
        .signer(authority)
        .send()
}

pub fn send_update_operator_whitelist(
    program: &Program<&Keypair>,
    authority: &Keypair,
    program_config: Pubkey,
    operators_to_add: Option<Vec<Pubkey>>,
    operators_to_remove: Option<Vec<Pubkey>>,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::UpdateOperatorWhitelist {
            authority: authority.pubkey(),
            program_config,
        })
        .args(instruction::UpdateOperatorWhitelist {
            operators_to_add,
            operators_to_remove,
        })
        .signer(authority)
        .send()
}

pub fn send_update_program_config(
    program: &Program<&Keypair>,
    authority: &Keypair,
    program_config: Pubkey,
    new_authority: Option<Keypair>,
    min_consensus_threshold_bps: Option<u16>,
    tie_breaker_admin: Option<Pubkey>,
    vote_duration: Option<i64>,
) -> Result<Signature, ClientError> {
    let mut accounts = accounts::UpdateProgramConfig {
        authority: authority.pubkey(),
        program_config,
        new_authority: None,
    };
    let mut builder = program.request().signer(authority);

    if let Some(new_authority) = new_authority {
        accounts.new_authority = Some(new_authority.pubkey());
        builder = builder.signer(new_authority);
    }

    builder
        .accounts(accounts)
        .args(instruction::UpdateProgramConfig {
            min_consensus_threshold_bps,
            tie_breaker_admin,
            vote_duration,
        })
        .send()
}

pub fn send_cast_vote(
    program: &Program<&Keypair>,
    operator: &Keypair,
    program_config: Pubkey,
    ballot_box: Pubkey,
    ballot: Ballot,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::CastVote {
            operator: operator.pubkey(),
            ballot_box,
            program_config,
        })
        .args(instruction::CastVote { ballot })
        .signer(operator)
        .send()
}

pub fn send_init_ballot_box(
    program: &Program<&Keypair>,
    operator: &Keypair,
    program_config: Pubkey,
    ballot_box: Pubkey,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::InitBallotBox {
            payer: program.payer(),
            operator: operator.pubkey(),
            ballot_box,
            program_config,
            system_program: system_program::ID,
        })
        .args(instruction::InitBallotBox {})
        .signer(operator)
        .send()
}

pub fn send_remove_vote(
    program: &Program<&Keypair>,
    operator: &Keypair,
    program_config: Pubkey,
    ballot_box: Pubkey,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::RemoveVote {
            operator: operator.pubkey(),
            ballot_box,
            program_config,
        })
        .args(instruction::RemoveVote {})
        .signer(operator)
        .send()
}

pub fn send_finalize_ballot(
    program: &Program<&Keypair>,
    ballot_box: Pubkey,
    consensus_result: Pubkey,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::FinalizeBallot {
            payer: program.payer(),
            ballot_box,
            consensus_result,
            system_program: system_program::ID,
        })
        .args(instruction::FinalizeBallot {})
        .send()
}

pub fn send_set_tie_breaker(
    program: &Program<&Keypair>,
    tie_breaker_admin: &Keypair,
    ballot_box: Pubkey,
    program_config: Pubkey,
    ballot_index: u8,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::SetTieBreaker {
            tie_breaker_admin: tie_breaker_admin.pubkey(),
            ballot_box,
            program_config,
        })
        .args(instruction::SetTieBreaker { ballot_index })
        .signer(tie_breaker_admin)
        .send()
}

pub fn send_init_meta_merkle_proof(
    program: &Program<&Keypair>,
    meta_merkle_proof_pda: Pubkey,
    consensus_result: Pubkey,
    meta_merkle_leaf: MetaMerkleLeaf,
    meta_merkle_proof: Vec<[u8; 32]>,
    close_timestamp: i64,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::InitMetaMerkleProof {
            payer: program.payer(),
            merkle_proof: meta_merkle_proof_pda,
            consensus_result,
            system_program: system_program::ID,
        })
        .args(instruction::InitMetaMerkleProof {
            meta_merkle_leaf,
            meta_merkle_proof,
            close_timestamp,
        })
        .send()
}

pub fn send_verify_merkle_proof(
    program: &Program<&Keypair>,
    consensus_result: Pubkey,
    meta_merkle_proof: Pubkey,
    stake_merkle_proof: Option<Vec<[u8; 32]>>,
    stake_merkle_leaf: Option<StakeMerkleLeaf>,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::VerifyMerkleProof {
            consensus_result,
            meta_merkle_proof,
        })
        .args(instruction::VerifyMerkleProof {
            stake_merkle_proof,
            stake_merkle_leaf,
        })
        .send()
}

pub fn send_close_meta_merkle_proof(
    program: &Program<&Keypair>,
    payer: &Keypair,
    meta_merkle_proof: Pubkey,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::CloseMetaMerkleProof {
            payer: payer.pubkey(),
            meta_merkle_proof,
            system_program: system_program::ID,
        })
        .args(instruction::CloseMetaMerkleProof {})
        .signer(payer)
        .send()
}
