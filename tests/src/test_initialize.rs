use std::{str::FromStr, thread, time::Duration};

use anchor_client::{
    anchor_lang::{system_program, AccountDeserialize},
    solana_client::rpc_config::RpcTransactionConfig,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signature},
        signer::Signer,
    },
    Client, ClientError, Cluster, Program,
};
use gov_v1::{
    accounts, instruction, Ballot, BallotBox, BallotTally, ConsensusResult, MetaMerkleLeaf,
    MetaMerkleProof, OperatorVote, ProgramConfig,
};

pub struct ProgramTestContext {
    pub payer: Keypair,
    pub program_config_pda: Pubkey,
    pub operators: Vec<Keypair>,
}

const VOTE_DURATION: i64 = 10;
const MIN_CONSENSUS_BPS: u16 = 6666;

pub fn assert_client_err(res: Result<Signature, ClientError>, msg: &str) {
    assert!(res.unwrap_err().to_string().contains(msg))
}

pub fn setup_program_config(payer: &Keypair, program: &Program<&Keypair>) -> ProgramTestContext {
    let (program_config_pda, _bump) = ProgramConfig::pda();
    program
        .request()
        .accounts(accounts::InitProgramConfig {
            payer: payer.pubkey(),
            authority: payer.pubkey(),
            program_config: program_config_pda,
            system_program: system_program::ID,
        })
        .args(instruction::InitProgramConfig {})
        .send()
        .unwrap();

    let operator_keypairs: Vec<Keypair> = (0..10).map(|_| Keypair::new()).collect();
    let operators_to_add: Vec<Pubkey> = operator_keypairs.iter().map(|x| x.pubkey()).collect();

    program
        .request()
        .accounts(accounts::UpdateOperatorWhitelist {
            authority: payer.pubkey(),
            program_config: program_config_pda,
        })
        .args(instruction::UpdateOperatorWhitelist {
            operators_to_add: Some(operators_to_add.clone()),
            operators_to_remove: None,
        })
        .send()
        .unwrap();

    program
        .request()
        .accounts(accounts::UpdateProgramConfig {
            authority: payer.pubkey(),
            program_config: program_config_pda,
        })
        .args(instruction::UpdateProgramConfig {
            new_authority: None,
            min_consensus_threshold_bps: Some(MIN_CONSENSUS_BPS),
            tie_breaker_admin: Some(payer.pubkey()),
            vote_duration: Some(VOTE_DURATION),
        })
        .send()
        .unwrap();

    ProgramTestContext {
        payer: payer.insecure_clone(),
        program_config_pda,
        operators: operator_keypairs,
    }
}

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

pub fn send_close_meta_merkle_proof(
    program: &Program<&Keypair>,
    meta_merkle_proof: Pubkey,
) -> Result<Signature, ClientError> {
    program
        .request()
        .accounts(accounts::CloseMetaMerkleProof {
            payer: program.payer(),
            meta_merkle_proof,
            system_program: system_program::ID,
        })
        .args(instruction::CloseMetaMerkleProof {})
        .send()
}

fn test_program_config(program: &Program<&Keypair>, context: &ProgramTestContext) {
    program
        .request()
        .accounts(accounts::InitProgramConfig {
            payer: program.payer(),
            authority: program.payer(),
            program_config: context.program_config_pda,
            system_program: system_program::ID,
        })
        .args(instruction::InitProgramConfig {})
        .send()
        .unwrap();

    // Verify values in ProgramConfig
    let program_config = fetch_program_config(context, program);
    assert_eq!(program_config.authority, program.payer());
    assert_eq!(program_config.tie_breaker_admin, Pubkey::default());
    assert_eq!(program_config.whitelisted_operators.len(), 0);
    assert_eq!(program_config.min_consensus_threshold_bps, 0);
    assert_eq!(program_config.next_ballot_id, 0);
    assert_eq!(program_config.vote_duration, 0);

    // Add operators
    let operators_to_add: Vec<Pubkey> = context.operators.iter().map(|x| x.pubkey()).collect();

    program
        .request()
        .accounts(accounts::UpdateOperatorWhitelist {
            authority: program.payer(),
            program_config: context.program_config_pda,
        })
        .args(instruction::UpdateOperatorWhitelist {
            operators_to_add: Some(operators_to_add.clone()),
            operators_to_remove: None,
        })
        .send()
        .unwrap();

    // Verify values in ProgramConfig
    let program_config = fetch_program_config(context, program);
    assert_eq!(program_config.whitelisted_operators, operators_to_add);

    // Remove operators
    let operators_to_remove = operators_to_add[8..].to_vec();
    program
        .request()
        .accounts(accounts::UpdateOperatorWhitelist {
            authority: program.payer(),
            program_config: context.program_config_pda,
        })
        .args(instruction::UpdateOperatorWhitelist {
            operators_to_add: None,
            operators_to_remove: Some(operators_to_remove),
        })
        .send()
        .unwrap();

    // Verify values in ProgramConfig
    let program_config = fetch_program_config(context, program);
    assert_eq!(
        program_config.whitelisted_operators,
        operators_to_add[..8].to_vec()
    );

    program
        .request()
        .accounts(accounts::UpdateProgramConfig {
            authority: program.payer(),
            program_config: context.program_config_pda,
        })
        .args(instruction::UpdateProgramConfig {
            new_authority: None,
            min_consensus_threshold_bps: Some(MIN_CONSENSUS_BPS),
            tie_breaker_admin: Some(program.payer()),
            vote_duration: Some(VOTE_DURATION),
        })
        .send()
        .unwrap();

    // Verify values in ProgramConfig
    let program_config = fetch_program_config(context, program);
    assert_eq!(program_config.authority, program.payer());
    assert_eq!(program_config.tie_breaker_admin, program.payer());
    assert_eq!(
        program_config.whitelisted_operators,
        operators_to_add[..8].to_vec()
    );
    assert_eq!(
        program_config.min_consensus_threshold_bps,
        MIN_CONSENSUS_BPS
    );
    assert_eq!(program_config.next_ballot_id, 0);
    assert_eq!(program_config.vote_duration, VOTE_DURATION);
}

fn test_balloting(
    program: &Program<&Keypair>,
    context: &ProgramTestContext,
) -> Result<(), ClientError> {
    let (ballot_box_pda, bump) = BallotBox::pda(0);

    // Init ballot box
    let operator1 = &context.operators[0];
    let tx = send_init_ballot_box(
        program,
        operator1,
        context.program_config_pda,
        ballot_box_pda,
    )?;
    let (slot_created, tx_block_time) = fetch_tx_block_details(program, tx);
    let epoch_info = program.rpc().get_epoch_info()?;
    let vote_expiry_timestamp = tx_block_time + VOTE_DURATION;

    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.ballot_id, 0);
    assert_eq!(ballot_box.bump, bump);
    assert_eq!(ballot_box.epoch, epoch_info.epoch);
    assert_eq!(ballot_box.slot_created, slot_created);
    assert_eq!(ballot_box.slot_consensus_reached, 0);
    assert_eq!(ballot_box.min_consensus_threshold_bps, MIN_CONSENSUS_BPS);
    assert_eq!(ballot_box.winning_ballot, Ballot::default());
    assert_eq!(ballot_box.operator_votes.len(), 0);
    assert_eq!(ballot_box.ballot_tallies.len(), 0);
    assert_eq!(ballot_box.vote_expiry_timestamp, vote_expiry_timestamp);

    // Check that next_ballot_id is incremented
    let program_config = fetch_program_config(context, program);
    assert_eq!(program_config.next_ballot_id, 1);

    // Operator 1 casts a vote.
    let ballot1 = Ballot {
        meta_merkle_root: [1; 32],
        snapshot_hash: [2; 32],
    };
    let tx = send_cast_vote(
        program,
        operator1,
        context.program_config_pda,
        ballot_box_pda,
        ballot1.clone(),
    )?;

    let (tx_slot, _tx_block_time) = fetch_tx_block_details(program, tx);
    let mut expected_operator_votes = [OperatorVote {
        operator: operator1.pubkey(),
        slot_voted: tx_slot,
        ballot_index: 0,
    }]
    .to_vec();
    let mut expected_ballot_tallies = [BallotTally {
        index: 0,
        ballot: ballot1.clone(),
        tally: 1,
    }]
    .to_vec();

    // Checks that a new ballot tally is created.
    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.ballot_id, 0);
    assert_eq!(ballot_box.bump, bump);
    assert_eq!(ballot_box.epoch, epoch_info.epoch);
    assert_eq!(ballot_box.slot_created, slot_created);
    assert_eq!(ballot_box.slot_consensus_reached, 0);
    assert_eq!(ballot_box.min_consensus_threshold_bps, MIN_CONSENSUS_BPS);
    assert_eq!(ballot_box.winning_ballot, Ballot::default());
    assert_eq!(ballot_box.operator_votes, expected_operator_votes);
    assert_eq!(ballot_box.ballot_tallies, expected_ballot_tallies);
    assert_eq!(ballot_box.vote_expiry_timestamp, vote_expiry_timestamp);

    // Operator 2 casts a different vote.
    let operator2 = &context.operators[1];
    let ballot2 = Ballot {
        meta_merkle_root: [2; 32],
        snapshot_hash: [3; 32],
    };
    let tx = send_cast_vote(
        program,
        operator2,
        context.program_config_pda,
        ballot_box_pda,
        ballot2.clone(),
    )?;
    let (tx_slot, _tx_block_time) = fetch_tx_block_details(program, tx);

    expected_operator_votes.push(OperatorVote {
        operator: operator2.pubkey(),
        slot_voted: tx_slot,
        ballot_index: 1,
    });
    expected_ballot_tallies.push(BallotTally {
        index: 1,
        ballot: ballot2.clone(),
        tally: 1,
    });

    // Checks that a new ballot tally is created.
    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.slot_consensus_reached, 0);
    assert_eq!(ballot_box.winning_ballot, Ballot::default());
    assert_eq!(ballot_box.operator_votes, expected_operator_votes);
    assert_eq!(ballot_box.ballot_tallies, expected_ballot_tallies);
    assert_eq!(ballot_box.vote_expiry_timestamp, vote_expiry_timestamp);

    // Operator 3, 4, 5, 6, 7 casts ballot 3.
    let ballot3 = Ballot {
        meta_merkle_root: [3; 32],
        snapshot_hash: [4; 32],
    };
    for i in 2..7 {
        let operator = &context.operators[i];
        let tx = send_cast_vote(
            program,
            operator,
            context.program_config_pda,
            ballot_box_pda,
            ballot3.clone(),
        )?;
        let (tx_slot, _tx_block_time) = fetch_tx_block_details(program, tx);
        expected_operator_votes.push(OperatorVote {
            operator: operator.pubkey(),
            slot_voted: tx_slot,
            ballot_index: 2,
        });
    }
    expected_ballot_tallies.push(BallotTally {
        index: 2,
        ballot: ballot3.clone(),
        tally: 5,
    });

    // Checks votes for operator 3, 4, 5, 6, 7 - no consensus reached yet.
    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.slot_consensus_reached, 0);
    assert_eq!(ballot_box.winning_ballot, Ballot::default());
    assert_eq!(ballot_box.operator_votes, expected_operator_votes);
    assert_eq!(ballot_box.ballot_tallies, expected_ballot_tallies);

    // Operator 2 removes vote (ballot 1).
    send_remove_vote(
        program,
        operator2,
        context.program_config_pda,
        ballot_box_pda,
    )?;
    expected_operator_votes.remove(1);
    expected_ballot_tallies[1].tally = 0;

    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.operator_votes, expected_operator_votes);
    assert_eq!(ballot_box.ballot_tallies, expected_ballot_tallies);

    // Removing non-existent vote should fail.
    let tx = send_remove_vote(
        program,
        operator2,
        context.program_config_pda,
        ballot_box_pda,
    );
    assert_client_err(tx, "Operator has not voted");

    // Finalize ballot should fail before consensus is reached.
    let (consensus_result_pda, _bump) = ConsensusResult::pda(0);
    let tx = send_finalize_ballot(program, ballot_box_pda, consensus_result_pda);
    assert_client_err(tx, "Consensus not reached");

    // Operator 2 votes for ballot 3 instead. Consensus expected with 6/8 votes (75%).
    let tx = send_cast_vote(
        program,
        operator2,
        context.program_config_pda,
        ballot_box_pda,
        ballot3.clone(),
    )?;
    let (consensus_slot, _tx_block_time) = fetch_tx_block_details(program, tx);

    expected_operator_votes.push(OperatorVote {
        operator: operator2.pubkey(),
        slot_voted: consensus_slot,
        ballot_index: 2,
    });
    expected_ballot_tallies[2].tally += 1;

    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.slot_consensus_reached, consensus_slot);
    assert_eq!(ballot_box.winning_ballot, ballot3);
    assert_eq!(ballot_box.operator_votes, expected_operator_votes);
    assert_eq!(ballot_box.ballot_tallies, expected_ballot_tallies);

    // Operator 8 should be able to vote even after consensus.
    let operator8 = &context.operators[7];
    let tx = send_cast_vote(
        program,
        operator8,
        context.program_config_pda,
        ballot_box_pda,
        ballot3.clone(),
    )?;
    let (tx_slot, _tx_block_time) = fetch_tx_block_details(program, tx);

    expected_operator_votes.push(OperatorVote {
        operator: operator8.pubkey(),
        slot_voted: tx_slot,
        ballot_index: 2,
    });
    expected_ballot_tallies[2].tally += 1;

    // Voting after consensus doesn't change the consensus result.
    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.slot_consensus_reached, consensus_slot);
    assert_eq!(ballot_box.winning_ballot, ballot3);
    assert_eq!(ballot_box.operator_votes, expected_operator_votes);
    assert_eq!(ballot_box.ballot_tallies, expected_ballot_tallies);

    // Voting more than once per operator should fail.
    let tx = send_cast_vote(
        program,
        operator8,
        context.program_config_pda,
        ballot_box_pda,
        ballot3.clone(),
    );
    assert_client_err(tx, "Operator has voted");

    // Removing vote after consensus fails.
    let tx = send_remove_vote(
        program,
        operator2,
        context.program_config_pda,
        ballot_box_pda,
    );
    assert_client_err(tx, "Consensus has reached");

    // Finalize ballot should succeed.
    send_finalize_ballot(program, ballot_box_pda, consensus_result_pda)?;
    let consensus_result = fetch_consensus_result(program, &consensus_result_pda);
    assert_eq!(consensus_result.ballot_id, ballot_box.ballot_id);
    assert_eq!(consensus_result.ballot, ballot_box.winning_ballot);

    Ok(())
}

fn test_tie_breaker(
    program: &Program<&Keypair>,
    context: &ProgramTestContext,
) -> Result<(), ClientError> {
    let (ballot_box_pda, bump) = BallotBox::pda(1);

    // Init ballot box
    let operator1 = &context.operators[0];
    let tx = send_init_ballot_box(
        program,
        operator1,
        context.program_config_pda,
        ballot_box_pda,
    )?;
    let (slot_created, tx_block_time) = fetch_tx_block_details(program, tx);
    let epoch_info = program.rpc().get_epoch_info()?;
    let vote_expiry_timestamp = tx_block_time + VOTE_DURATION;

    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.ballot_id, 1);
    assert_eq!(ballot_box.bump, bump);
    assert_eq!(ballot_box.epoch, epoch_info.epoch);
    assert_eq!(ballot_box.slot_created, slot_created);
    assert_eq!(ballot_box.slot_consensus_reached, 0);
    assert_eq!(ballot_box.min_consensus_threshold_bps, MIN_CONSENSUS_BPS);
    assert_eq!(ballot_box.winning_ballot, Ballot::default());
    assert_eq!(ballot_box.operator_votes.len(), 0);
    assert_eq!(ballot_box.ballot_tallies.len(), 0);
    assert_eq!(ballot_box.vote_expiry_timestamp, vote_expiry_timestamp);

    let ballot1 = Ballot {
        meta_merkle_root: [1; 32],
        snapshot_hash: [3; 32],
    };
    let ballot2 = Ballot {
        meta_merkle_root: [2; 32],
        snapshot_hash: [4; 32],
    };

    let mut expected_operator_votes = vec![];
    let mut expected_ballot_tallies = [
        BallotTally {
            index: 0,
            ballot: ballot1.clone(),
            tally: 0,
        },
        BallotTally {
            index: 1,
            ballot: ballot2.clone(),
            tally: 0,
        },
    ]
    .to_vec();

    for i in 0..2 {
        let operator = &context.operators[i];
        let tx = send_cast_vote(
            program,
            operator,
            context.program_config_pda,
            ballot_box_pda,
            ballot1.clone(),
        )?;
        let (tx_slot, _tx_block_time) = fetch_tx_block_details(program, tx);
        expected_operator_votes.push(OperatorVote {
            operator: operator.pubkey(),
            slot_voted: tx_slot,
            ballot_index: 0,
        });
        expected_ballot_tallies[0].tally += 1;
    }

    for i in 2..6 {
        let operator = &context.operators[i];
        let tx = send_cast_vote(
            program,
            operator,
            context.program_config_pda,
            ballot_box_pda,
            ballot2.clone(),
        )?;
        let (tx_slot, _tx_block_time) = fetch_tx_block_details(program, tx);
        expected_operator_votes.push(OperatorVote {
            operator: operator.pubkey(),
            slot_voted: tx_slot,
            ballot_index: 1,
        });
        expected_ballot_tallies[1].tally += 1;
    }

    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.slot_consensus_reached, 0);
    assert_eq!(ballot_box.winning_ballot, Ballot::default());
    assert_eq!(ballot_box.operator_votes, expected_operator_votes);
    assert_eq!(ballot_box.ballot_tallies, expected_ballot_tallies);

    // Setting tie breaker vote before vote expiry fails.
    let tx = send_set_tie_breaker(
        program,
        &context.payer,
        ballot_box_pda,
        context.program_config_pda,
        0,
    );
    assert_client_err(tx, "Voting not expired");

    // Sleep till expiry
    let current_slot = program.rpc().get_slot()?;
    let current_time = program.rpc().get_block_time(current_slot)?;
    let sleep_duration = vote_expiry_timestamp - current_time + 2;
    thread::sleep(Duration::from_secs(sleep_duration as u64));

    // Set tie breaker vote after expiry.
    let tx = send_set_tie_breaker(
        program,
        &context.payer,
        ballot_box_pda,
        context.program_config_pda,
        0,
    )?;
    let (consensus_slot, _tx_block_time) = fetch_tx_block_details(program, tx);

    // Verify that consensus is reached.
    let ballot_box = fetch_ballot_box(program, &ballot_box_pda);
    assert_eq!(ballot_box.slot_consensus_reached, consensus_slot);
    assert_eq!(ballot_box.winning_ballot, ballot1);
    assert_eq!(ballot_box.operator_votes, expected_operator_votes);
    assert_eq!(ballot_box.ballot_tallies, expected_ballot_tallies);

    // Finalize ballot after consensus.
    let (consensus_result_pda, _bump) = ConsensusResult::pda(1);
    send_finalize_ballot(program, ballot_box_pda, consensus_result_pda)?;
    let consensus_result = fetch_consensus_result(program, &consensus_result_pda);
    assert_eq!(consensus_result.ballot_id, ballot_box.ballot_id);
    assert_eq!(consensus_result.ballot, ballot_box.winning_ballot);

    // Setting tie breaker vote after consensus fails.
    let tx = send_set_tie_breaker(
        program,
        &context.payer,
        ballot_box_pda,
        context.program_config_pda,
        0,
    );
    assert_client_err(tx, "Consensus has reached");

    Ok(())
}

fn test_merkle_proof(
    program: &Program<&Keypair>,
    context: &ProgramTestContext,
) -> Result<(), ClientError> {
    //   let vote_account = &Pubkey::new_unique();
    //   let (consensus_result_pda, _bump) = ConsensusResult::pda(0);
    //   let (merkle_proof_pda, _bump) = MetaMerkleProof::pda(&consensus_result_pda, vote_account);
    // send_init_meta_merkle_proof(program, merkle_proof_pda, consensus_result_pda,  )

    Ok(())
}

#[test]
fn test_full_program_flow() {
    let program_id = "HQrwhDzMa7dEnUi2Nku925yeAAxioFhqpLMpQ4g6Zh5N";
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = read_keypair_file(&anchor_wallet).unwrap();

    let client = Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
    let program_id = Pubkey::from_str(program_id).unwrap();
    let program = client.program(program_id).unwrap();

    let (program_config_pda, _bump) = ProgramConfig::pda();
    let operator_keypairs: Vec<Keypair> = (0..10).map(|_| Keypair::new()).collect();
    let context = ProgramTestContext {
        payer: payer.insecure_clone(),
        program_config_pda,
        operators: operator_keypairs,
    };

    test_program_config(&program, &context);
    test_balloting(&program, &context).unwrap();
    test_tie_breaker(&program, &context).unwrap();
}
