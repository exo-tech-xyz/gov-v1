use borsh::{BorshDeserialize, BorshSerialize};
use flate2::{write::GzEncoder, Compression};
use gov_v1::{MetaMerkleLeaf, StakeMerkleLeaf};
use meta_merkle_tree::{merkle_tree::MerkleTree, utils::get_proof};
use solana_program::{
    hash::{hashv, Hash},
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

    pub fn read(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Self::try_from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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

impl MetaMerkleLeafBundle {
    pub fn get_stake_merkle_proof(self, index: usize) -> Vec<[u8; 32]> {
        let hashed_nodes: Vec<[u8; 32]> = self
            .stake_merkle_leaves
            .iter()
            .map(|n| n.hash().to_bytes())
            .collect();
        let stake_merkle = MerkleTree::new(&hashed_nodes[..], true);
        get_proof(&stake_merkle, index)
    }
}
