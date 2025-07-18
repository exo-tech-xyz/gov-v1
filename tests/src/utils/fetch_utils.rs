use anchor_client::{
    anchor_lang::AccountDeserialize,
    solana_client::rpc_config::RpcTransactionConfig,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{Keypair, Signature},
    },
    Program,
};
use gov_v1::{BallotBox, ConsensusResult, MetaMerkleProof, ProgramConfig};

use crate::utils::data_types::ProgramTestContext;

pub fn fetch_program_config(
    context: &ProgramTestContext,
    program: &Program<&Keypair>,
) -> ProgramConfig {
    let account_data = program
        .rpc()
        .get_account(&context.program_config_pda)
        .unwrap();
    ProgramConfig::try_deserialize(&mut account_data.data.as_ref()).unwrap()
}

pub fn fetch_ballot_box(program: &Program<&Keypair>, pubkey: &Pubkey) -> BallotBox {
    let account_data = program.rpc().get_account(pubkey).unwrap();
    BallotBox::try_deserialize(&mut account_data.data.as_ref()).unwrap()
}

pub fn fetch_consensus_result(program: &Program<&Keypair>, pubkey: &Pubkey) -> ConsensusResult {
    let account_data = program.rpc().get_account(pubkey).unwrap();
    ConsensusResult::try_deserialize(&mut account_data.data.as_ref()).unwrap()
}

pub fn fetch_merkle_proof(program: &Program<&Keypair>, pubkey: &Pubkey) -> MetaMerkleProof {
    let account_data = program.rpc().get_account(pubkey).unwrap();
    MetaMerkleProof::try_deserialize(&mut account_data.data.as_ref()).unwrap()
}

pub fn fetch_tx_block_details(program: &Program<&Keypair>, tx: Signature) -> (u64, i64) {
    let tx_details = program
        .rpc()
        .get_transaction_with_config(
            &tx,
            RpcTransactionConfig {
                encoding: None,
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: None,
            },
        )
        .unwrap();
    (tx_details.slot, tx_details.block_time.unwrap())
}
