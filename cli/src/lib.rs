pub mod consts;
pub mod merkle;
pub mod utils;

use crate::consts::{MARINADE_OPS_VOTING_WALLET, MARINADE_WITHDRAW_AUTHORITY};
use im::HashMap;
pub use merkle::*;

use anyhow::Error;
use borsh_stake::BorshDeserialize;
use gov_v1::{MetaMerkleLeaf, StakeMerkleLeaf};
use itertools::Itertools;
use meta_merkle_tree::{
    generated_merkle_tree::Delegation, merkle_tree::MerkleTree, utils::get_proof,
};
use solana_program::{pubkey::Pubkey, stake_history::StakeHistory, sysvar};
use solana_runtime::{bank::Bank, stakes::StakeAccount};
use solana_sdk::account::from_account;
use solana_sdk::account::AccountSharedData;
use solana_sdk::account::ReadableAccount;
use spl_stake_pool::state::AccountType;
use spl_stake_pool::state::StakePool;
use std::sync::Arc;

/// Given an [EpochStakes] object, return delegations grouped by voter_pubkey (validator delegated to).
/// Delegations store the active stake of the delegator.
fn group_delegations_by_voter_pubkey_active_stake(
    delegations: &im::HashMap<Pubkey, StakeAccount>,
    bank: &Bank,
) -> im::HashMap<Pubkey, Vec<Delegation>> {
    let stake_history =
        from_account::<StakeHistory, _>(&bank.get_account(&sysvar::stake_history::id()).unwrap())
            .unwrap();
    let grouped = delegations
        .iter()
        .filter_map(|(stake_pubkey, stake_account)| {
            let active_stake = stake_account.delegation().stake(
                bank.epoch(),
                &stake_history,
                bank.new_warmup_cooldown_rate_epoch(),
            );
            if active_stake == 0 {
                return None;
            }

            Some((
                stake_account.delegation().voter_pubkey,
                Delegation {
                    stake_account_pubkey: *stake_pubkey,
                    staker_pubkey: stake_account
                        .stake_state()
                        .authorized()
                        .map(|a| a.staker)
                        .unwrap_or_default(),
                    withdrawer_pubkey: stake_account
                        .stake_state()
                        .authorized()
                        .map(|a| a.withdrawer)
                        .unwrap_or_default(),
                    lamports_delegated: active_stake,
                },
            ))
        })
        .into_group_map();

    im::HashMap::from_iter(grouped)
}

/// Updates given map with new entry mapping withdraw authority to manager authority
/// if account is a StakePool.
fn update_stake_pool_voter_map(
    stake_pool_voter_map: &mut HashMap<Pubkey, Pubkey>,
    account: &AccountSharedData,
) {
    if account.owner() != &spl_stake_pool::id() {
        return;
    }

    // Check discriminator: first byte should be 1 (AccountType::StakePool)
    let data = account.data();
    if data.is_empty() || data[0] != AccountType::StakePool as u8 {
        return;
    }

    if let Ok(stake_pool) = StakePool::deserialize(&mut &account.data()[..]) {
        if let Some(withdraw_authority) = stake_pool.sol_withdraw_authority {
            // Sanity check: ensure the manager is not the default/zero pubkey
            if stake_pool.manager == Pubkey::default() {
                return;
            }

            stake_pool_voter_map.insert(withdraw_authority, stake_pool.manager);
        }
    }
}

/// Creates a MetaMerkleSnapshot from the given bank.
pub fn generate_meta_merkle_snapshot(bank: &Arc<Bank>) -> Result<MetaMerkleSnapshot, Error> {
    assert!(bank.is_frozen());

    println!("Bank loaded for epoch: {:?}", bank.epoch());

    // Pre-process: Find all Stake Pools and map withdraw_authority to their voting wallet
    // (StakePool manager by default)
    let mut stake_pool_voter_map: HashMap<Pubkey, Pubkey> = HashMap::new();

    // Maps Marinade LST stake pool withdraw authority to its ops wallet.
    stake_pool_voter_map.insert(MARINADE_WITHDRAW_AUTHORITY, MARINADE_OPS_VOTING_WALLET);

    // Scan all accounts owned by the stake pool program
    bank.scan_all_accounts(
        |item| {
            if let Some((_pubkey, account, _slot)) = item {
                update_stake_pool_voter_map(&mut stake_pool_voter_map, &account);
            }
        },
        false,
    )?;
    println!("Stake Pools Count: {}", stake_pool_voter_map.len());

    let l_stakes = bank.stakes_cache.stakes();
    let delegations = l_stakes.stake_delegations();
    let epoch_vote_accounts = bank.epoch_vote_accounts(bank.epoch()).unwrap_or_else(|| {
        panic!(
            "No epoch_vote_accounts found for slot {} at epoch {}",
            bank.slot(),
            bank.epoch()
        )
    });
    println!("Vote Accounts Count: {}", epoch_vote_accounts.len());
    let voter_pubkey_to_delegations =
        group_delegations_by_voter_pubkey_active_stake(delegations, bank)
            .into_iter()
            .collect::<HashMap<_, _>>();

    // 1. Generate leaf nodes for MetaMerkleTree.
    let (meta_merkle_leaves, stake_merkle_leaves_collection) = voter_pubkey_to_delegations
        .iter()
        .filter_map(|(voter_pubkey, delegations)| {
            let (vote_account_stake, vote_account) =
                epoch_vote_accounts.get(voter_pubkey).or_else(|| {
                    // Vote account may be missing from epoch_vote_accounts if the validator is not active,
                    // despite having delegated stake.
                    eprintln!("Missing {} from epoch_vote_accounts", voter_pubkey);
                    None
                })?;

            // 1. Create leaf nodes for StakeMerkleTree.
            let mut stake_merkle_leaves = delegations
                .iter()
                .map(|delegation| {
                    let mut voting_wallet = delegation.withdrawer_pubkey;

                    // Overwrite voting wallet if stake account has a withdraw authority that is 
                    // mapped to a different wallet. Otherwise, use the withdrawer authority.
                    if let Some(manager) = stake_pool_voter_map.get(&delegation.withdrawer_pubkey) {
                        voting_wallet = *manager;
                        println!(
                            "Using stake pool manager {} for stake account {}",
                            voting_wallet, delegation.stake_account_pubkey
                        );
                    }

                    StakeMerkleLeaf {
                        voting_wallet,
                        stake_account: delegation.stake_account_pubkey,
                        active_stake: delegation.lamports_delegated,
                    }
                })
                .collect::<Vec<StakeMerkleLeaf>>();

            // 2. Sort leaves by stake account key.
            stake_merkle_leaves.sort_by_key(|leaf| leaf.stake_account);

            // 3. Build StakeMerkleTree to get a root node.
            let hashed_nodes: Vec<[u8; 32]> = stake_merkle_leaves
                .iter()
                .map(|n| n.hash().to_bytes())
                .collect();
            let stake_merkle = MerkleTree::new(&hashed_nodes[..], true);

            // 4. Build MetaMerkleLeaf using root node of StakeMerkleTree.
            let meta_merkle_leaf = MetaMerkleLeaf {
                vote_account: *voter_pubkey,
                voting_wallet: vote_account.vote_state().authorized_withdrawer,
                stake_merkle_root: stake_merkle.get_root().unwrap().to_bytes(),
                active_stake: *vote_account_stake,
            };

            Some((meta_merkle_leaf, stake_merkle_leaves))
        })
        .collect::<(Vec<MetaMerkleLeaf>, Vec<Vec<StakeMerkleLeaf>>)>();

    // 2. Sort leaves by vote account key.
    let mut combined: Vec<(MetaMerkleLeaf, Vec<StakeMerkleLeaf>)> = meta_merkle_leaves
        .into_iter()
        .zip(stake_merkle_leaves_collection)
        .collect();
    combined.sort_by_key(|(leaf, _)| leaf.vote_account);
    let (meta_merkle_leaves, stake_merkle_leaves_collection): (Vec<_>, Vec<_>) =
        combined.into_iter().unzip();

    // 3. Build MetaMerkleTree to get a root node.
    let hashed_nodes: Vec<[u8; 32]> = meta_merkle_leaves
        .iter()
        .map(|n| n.hash().to_bytes())
        .collect();
    let meta_merkle = MerkleTree::new(&hashed_nodes[..], true);

    // 4. Generate MetaMerkleLeafBundle with proof.
    let meta_merkle_bundles = meta_merkle_leaves
        .into_iter()
        .zip(stake_merkle_leaves_collection)
        .enumerate()
        .map(
            |(i, (meta_merkle_leaf, stake_merkle_leaves))| MetaMerkleLeafBundle {
                meta_merkle_leaf,
                stake_merkle_leaves,
                proof: Some(get_proof(&meta_merkle, i)),
            },
        )
        .collect();

    Ok(MetaMerkleSnapshot {
        root: meta_merkle.get_root().unwrap().to_bytes(),
        leaf_bundles: meta_merkle_bundles,
        slot: bank.slot(),
    })
}
