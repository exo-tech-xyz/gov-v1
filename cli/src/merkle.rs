use serde::{Deserialize, Serialize};
use serde_json;
use solana_program::hash::hashv;
use solana_sdk::{hash::Hash, pubkey::Pubkey};
use std::fs::File;
use std::io::Write;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MetaMerkleSnapshot {
    /// Hash of MetaMerkleTree
    pub root: Hash,
    /// Each bundle contains the meta-level leaf, its stake-level leaves, and proof.
    pub leaf_bundles: Vec<MetaMerkleLeafBundle>,
    /// Slot where the tree was generated.
    pub slot: u64,
}

impl MetaMerkleSnapshot {
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).expect("Failed to serialize");
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MetaMerkleLeafBundle {
    /// MetaMerkleLeaf constructed from the StakeMerkleTree.
    pub meta_merkle_leaf: MetaMerkleLeaf,
    /// Leaf nodes of the StakeMerkleTree.
    pub stake_merkle_leaves: Vec<StakeMerkleLeaf>,
    /// Proof to verify MetaMerkleLeaf existence in MetaMerkleTree.
    pub proof: Option<Vec<[u8; 32]>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StakeMerkleLeaf {
    /// Wallet designated for governance voting for the stake account.
    pub voting_wallet: Pubkey,
    /// The stake account address.
    pub stake_account: Pubkey,
    /// Active delegated stake amount.
    pub active_stake: u64,
}

impl StakeMerkleLeaf {
    pub fn hash(&self) -> Hash {
        hashv(&[
            &self.voting_wallet.to_bytes(),
            &self.stake_account.to_bytes(),
            &self.active_stake.to_le_bytes(),
        ])
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MetaMerkleLeaf {
    /// Wallet designated for governance voting for the vote account.
    pub voting_wallet: Pubkey,
    /// Validator's vote account.
    pub vote_account: Pubkey,
    /// Root hash of the StakeMerkleTree, representing all active stake accounts
    /// delegated to the current vote account.
    pub stake_merkle_root: Hash,
    /// Total active delegated stake under this vote account.
    pub active_stake: u64,
}

impl MetaMerkleLeaf {
    pub fn hash(&self) -> Hash {
        hashv(&[
            &self.voting_wallet.to_bytes(),
            &self.vote_account.to_bytes(),
            &self.stake_merkle_root.to_bytes(),
            &self.active_stake.to_le_bytes(),
        ])
    }
}
