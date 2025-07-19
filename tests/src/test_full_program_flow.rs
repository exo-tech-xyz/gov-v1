use std::{cmp::min, str::FromStr, thread, time::Duration};

use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
        signer::Signer,
    },
    Client, ClientError, Cluster, Program,
};
use cli::MetaMerkleSnapshot;
use gov_v1::{
    Ballot, BallotBox, BallotTally, ConsensusResult, MetaMerkleProof, OperatorVote, ProgramConfig,
};

use crate::utils::{
    assert::assert_client_err, data_types::ProgramTestContext, fetch_utils::*, send_utils::*,
};

const VOTE_DURATION: i64 = 10;
const MIN_CONSENSUS_BPS: u16 = 6666;

fn test_program_config(
    program: &Program<&Keypair>,
    context: &ProgramTestContext,
) -> Result<(), ClientError> {
    send_init_program_config(program, &context.payer, context.program_config_pda)?;

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

    send_update_operator_whitelist(
        program,
        &context.payer,
        context.program_config_pda,
        Some(operators_to_add.clone()),
        None,
    )?;

    // Verify values in ProgramConfig
    let program_config = fetch_program_config(context, program);
    assert_eq!(program_config.whitelisted_operators, operators_to_add);

    // Remove operators
    let operators_to_remove = operators_to_add[8..].to_vec();

    send_update_operator_whitelist(
        program,
        &context.payer,
        context.program_config_pda,
        None,
        Some(operators_to_remove),
    )?;

    // Verify values in ProgramConfig
    let program_config = fetch_program_config(context, program);
    assert_eq!(
        program_config.whitelisted_operators,
        operators_to_add[..8].to_vec()
    );

    send_update_program_config(
        program,
        &context.payer,
        context.program_config_pda,
        None,
        Some(MIN_CONSENSUS_BPS),
        Some(program.payer()),
        Some(VOTE_DURATION),
    )?;

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

    Ok(())
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

    // Casting an invalid ballot fails.
    let ballot1 = Ballot {
        meta_merkle_root: [0; 32],
        snapshot_hash: [2; 32],
    };
    let tx = send_cast_vote(
        program,
        operator1,
        context.program_config_pda,
        ballot_box_pda,
        ballot1.clone(),
    );
    assert_client_err(tx, "Invalid ballot");

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

    // Casting ballot for non-whitelisted operator should fail.
    let tx = send_cast_vote(
        program,
        &Keypair::new(),
        context.program_config_pda,
        ballot_box_pda,
        ballot1.clone(),
    );
    assert_client_err(tx, "Operator not whitelisted");

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
        meta_merkle_root: context.meta_merkle_snapshot.root,
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

    // Casting vote after expiry should
    let tx = send_cast_vote(
        program,
        &context.operators[7],
        context.program_config_pda,
        ballot_box_pda,
        ballot1.clone(),
    );
    assert_client_err(tx, "Voting has expired");

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

fn test_merkle_proofs(
    program: &Program<&Keypair>,
    context: &ProgramTestContext,
) -> Result<(), ClientError> {
    let bundle = &context.meta_merkle_snapshot.leaf_bundles[0];
    let meta_proof = bundle.proof.clone().unwrap();
    let meta_leaf = bundle.meta_merkle_leaf.clone();

    let (consensus_result_pda, _bump) = ConsensusResult::pda(0);
    let (merkle_proof_pda, _bump) =
        MetaMerkleProof::pda(&consensus_result_pda, &meta_leaf.vote_account);

    // Init MetaMerkleProof
    send_init_meta_merkle_proof(
        program,
        merkle_proof_pda,
        consensus_result_pda,
        meta_leaf,
        meta_proof.clone(),
        1,
    )?;

    let merkle_proof = fetch_merkle_proof(program, &merkle_proof_pda);
    assert_eq!(merkle_proof.payer, program.payer());
    assert_eq!(merkle_proof.consensus_result, consensus_result_pda);
    assert_eq!(merkle_proof.meta_merkle_leaf, bundle.meta_merkle_leaf);
    assert_eq!(merkle_proof.meta_merkle_proof, meta_proof);
    assert_eq!(merkle_proof.close_timestamp, 1);

    // Verifies that leaf exist in root stored in consensus result.
    send_verify_merkle_proof(program, consensus_result_pda, merkle_proof_pda, None, None)?;

    // Verify for stake accounts under this vote account.
    let stake_leaves = &bundle.stake_merkle_leaves;
    for i in 0..min(5, stake_leaves.len()) {
        let stake_proof = bundle.clone().get_stake_merkle_proof(i);
        send_verify_merkle_proof(
            program,
            consensus_result_pda,
            merkle_proof_pda,
            Some(stake_proof),
            Some(stake_leaves[i].clone()),
        )?;
    }

    // Close MetaMerkleProof
    send_close_meta_merkle_proof(program, &context.payer, merkle_proof_pda)?;

    // Check that its closed.
    program
        .rpc()
        .get_account(&merkle_proof_pda)
        .expect_err("AccountNotFound");
    Ok(())
}

fn test_invalid_merkle_proofs(
    program: &Program<&Keypair>,
    context: &ProgramTestContext,
) -> Result<(), ClientError> {
    let bundle1 = &context.meta_merkle_snapshot.leaf_bundles[0];
    let bundle2 = &context.meta_merkle_snapshot.leaf_bundles[1];
    let meta_leaf1 = bundle1.meta_merkle_leaf.clone();
    let meta_proof1 = bundle1.proof.clone().unwrap();
    let meta_proof2 = bundle2.proof.clone().unwrap();

    let (consensus_result_pda, _bump) = ConsensusResult::pda(0);
    let (merkle_proof_pda, _bump) =
        MetaMerkleProof::pda(&consensus_result_pda, &meta_leaf1.vote_account);

    // Init MetaMerkleProof should fail when proof is invalid.
    let tx = send_init_meta_merkle_proof(
        program,
        merkle_proof_pda,
        consensus_result_pda,
        meta_leaf1.clone(),
        meta_proof2.clone(),
        1,
    );
    assert_client_err(tx, "Invalid merkle proof");

    // Init MetaMerkleProof for bundle1.
    send_init_meta_merkle_proof(
        program,
        merkle_proof_pda,
        consensus_result_pda,
        meta_leaf1.clone(),
        meta_proof1.clone(),
        1,
    )?;

    // Verify should fail with invalid proof from bundle2.
    let stake_leaves = &bundle2.stake_merkle_leaves;
    let stake_proof = bundle2.clone().get_stake_merkle_proof(0);
    let tx = send_verify_merkle_proof(
        program,
        consensus_result_pda,
        merkle_proof_pda,
        Some(stake_proof.clone()),
        Some(stake_leaves[0].clone()),
    );
    assert_client_err(tx, "Invalid merkle proof");

    let tx = send_verify_merkle_proof(
        program,
        consensus_result_pda,
        merkle_proof_pda,
        Some(stake_proof),
        None,
    );
    assert_client_err(tx, "Invalid merkle inputs");

    Ok(())
}

#[test]
fn main() {
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = read_keypair_file(&anchor_wallet).unwrap();

    let client = Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
    let program = client.program(gov_v1::id()).unwrap();

    let (program_config_pda, _bump) = ProgramConfig::pda();
    let operator_keypairs: Vec<Keypair> = (0..10).map(|_| Keypair::new()).collect();

    let path = format!(
        "{}/src/fixtures/meta_merkle.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    println!("path {}", path);
    let meta_merkle_snapshot = MetaMerkleSnapshot::read(&path).unwrap();

    let context = ProgramTestContext {
        payer: payer.insecure_clone(),
        program_config_pda,
        operators: operator_keypairs,
        meta_merkle_snapshot,
    };
    test_program_config(&program, &context);
    test_balloting(&program, &context).unwrap();
    test_merkle_proofs(&program, &context).unwrap();
    test_invalid_merkle_proofs(&program, &context).unwrap();
    test_tie_breaker(&program, &context).unwrap();
}
