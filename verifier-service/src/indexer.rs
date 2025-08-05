//! Snapshot data indexing functionality

use anyhow::Result;
use cli::MetaMerkleSnapshot;
use meta_merkle_tree::{merkle_tree::MerkleTree, utils::get_proof};
use rusqlite::Connection;
use tracing::{debug, info};

use crate::database::models::{SnapshotMetaRecord, StakeAccountRecord, VoteAccountRecord};

/// Index snapshot data in the database
pub async fn index_snapshot_data(
    db_path: &str,
    snapshot: &MetaMerkleSnapshot,
    network: &str,
    merkle_root: &str,
    snapshot_hash: &str,
) -> Result<()> {
    let conn = Connection::open(db_path)?;

    // Create snapshot metadata record
    let snapshot_meta = SnapshotMetaRecord {
        network: network.to_string(),
        slot: snapshot.slot,
        merkle_root: merkle_root.to_string(),
        snapshot_hash: snapshot_hash.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    snapshot_meta.insert(&conn)?;

    // Index vote accounts and stake accounts
    for (bundle_idx, bundle) in snapshot.leaf_bundles.iter().enumerate() {
        info!(
            "Indexing bundle {} / {}",
            bundle_idx,
            snapshot.leaf_bundles.len()
        );
        let meta_leaf = &bundle.meta_merkle_leaf;

        // Convert meta merkle proof to base58 strings
        let meta_merkle_proof: Vec<String> = bundle
            .proof
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|hash| bs58::encode(hash).into_string())
            .collect();

        // Create vote account record
        let vote_account_record = VoteAccountRecord {
            network: network.to_string(),
            snapshot_slot: snapshot.slot,
            vote_account: meta_leaf.vote_account.to_string(),
            voting_wallet: meta_leaf.voting_wallet.to_string(),
            stake_merkle_root: bs58::encode(meta_leaf.stake_merkle_root).into_string(),
            active_stake: meta_leaf.active_stake,
            meta_merkle_proof,
        };
        vote_account_record.insert(&conn)?;

        // Generate stake merkle tree under vote account
        let hashed_nodes: Vec<[u8; 32]> = bundle
            .stake_merkle_leaves
            .iter()
            .map(|n| n.hash().to_bytes())
            .collect();
        let stake_merkle = MerkleTree::new(&hashed_nodes[..], true);

        // Create stake account records for each stake leaf
        for (idx, stake_leaf) in bundle.stake_merkle_leaves.iter().enumerate() {
            let stake_merkle_proof = get_proof(&stake_merkle, idx)
                .iter()
                .map(|hash| bs58::encode(hash).into_string())
                .collect();

            let stake_account_record = StakeAccountRecord {
                network: network.to_string(),
                snapshot_slot: snapshot.slot,
                stake_account: stake_leaf.stake_account.to_string(),
                vote_account: meta_leaf.vote_account.to_string(),
                voting_wallet: stake_leaf.voting_wallet.to_string(),
                active_stake: stake_leaf.active_stake,
                stake_merkle_proof,
            };

            stake_account_record.insert(&conn)?;
        }

        debug!(
            "Indexed bundle {}: vote_account={}, {} stake accounts",
            bundle_idx,
            meta_leaf.vote_account,
            bundle.stake_merkle_leaves.len()
        );
    }

    info!(
        "Successfully indexed snapshot for slot {} with {} vote accounts",
        snapshot.slot,
        snapshot.leaf_bundles.len()
    );

    Ok(())
}
