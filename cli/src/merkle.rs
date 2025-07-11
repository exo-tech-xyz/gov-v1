use borsh::{BorshDeserialize, BorshSerialize};
use flate2::{Compression, write::GzEncoder};
use solana_program::{
    hash::{Hash, hashv},
    pubkey::Pubkey,
};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct MetaMerkleSnapshot {
    /// Hash of MetaMerkleTree
    pub root: [u8; 32],
    /// Each bundle contains the meta-level leaf, its stake-level leaves, and proof.
    pub leaf_bundles: Vec<MetaMerkleLeafBundle>,
    /// Slot where the tree was generated.
    pub slot: u64,
}

impl MetaMerkleSnapshot {
    pub fn save(&self, path: &str) -> io::Result<()> {
        let data = self.try_to_vec()?;
        fs::write(path, data)
    }

    pub fn save_compressed(&self, path: &str) -> io::Result<()> {
        let data = self.try_to_vec()?;
        let file = File::create(path)?;
        let mut enc = GzEncoder::new(file, Compression::default());
        enc.write_all(&data)?;
        enc.finish()?;

        Ok(())
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct MetaMerkleLeafBundle {
    /// MetaMerkleLeaf constructed from the StakeMerkleTree.
    pub meta_merkle_leaf: MetaMerkleLeaf,
    /// Leaf nodes of the StakeMerkleTree.
    pub stake_merkle_leaves: Vec<StakeMerkleLeaf>,
    /// Proof to verify MetaMerkleLeaf existence in MetaMerkleTree.
    pub proof: Option<Vec<[u8; 32]>>,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
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

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct MetaMerkleLeaf {
    /// Wallet designated for governance voting for the vote account.
    pub voting_wallet: Pubkey,
    /// Validator's vote account.
    pub vote_account: Pubkey,
    /// Root hash of the StakeMerkleTree, representing all active stake accounts
    /// delegated to the current vote account.
    pub stake_merkle_root: [u8; 32],
    /// Total active delegated stake under this vote account.
    pub active_stake: u64,
}

impl MetaMerkleLeaf {
    pub fn hash(&self) -> Hash {
        hashv(&[
            &self.voting_wallet.to_bytes(),
            &self.vote_account.to_bytes(),
            &self.stake_merkle_root,
            &self.active_stake.to_le_bytes(),
        ])
    }
}
