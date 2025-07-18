use anchor_client::solana_sdk::hash::{hashv, Hash};
use borsh::BorshDeserialize;
use gov_v1::StakeMerkleLeaf;
use meta_merkle_tree::merkle_tree::MerkleTree;
use std::fs::File;
use std::io::{self, Read};

use crate::utils::data_types::MetaMerkleSnapshot;

pub fn read_meta_merkle_snapshot(path: &str) -> io::Result<MetaMerkleSnapshot> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    MetaMerkleSnapshot::try_from_slice(&buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn get_stake_merkle_proof(
    stake_merkle_leaves: Vec<StakeMerkleLeaf>,
    index: usize,
) -> Vec<[u8; 32]> {
    pub fn hash(leaf: &StakeMerkleLeaf) -> Hash {
        hashv(&[
            &leaf.voting_wallet.to_bytes(),
            &leaf.stake_account.to_bytes(),
            &leaf.active_stake.to_le_bytes(),
        ])
    }

    fn get_proof(merkle_tree: &MerkleTree, index: usize) -> Vec<[u8; 32]> {
        let mut proof = Vec::new();
        let path = merkle_tree.find_path(index).expect("path to index");
        for branch in path.get_proof_entries() {
            if let Some(hash) = branch.get_left_sibling() {
                proof.push(hash.to_bytes());
            } else if let Some(hash) = branch.get_right_sibling() {
                proof.push(hash.to_bytes());
            } else {
                panic!("expected some hash at each level of the tree");
            }
        }
        proof
    }

    let hashed_nodes: Vec<[u8; 32]> = stake_merkle_leaves
        .iter()
        .map(|n| hash(n).to_bytes())
        .collect();
    let stake_merkle = MerkleTree::new(&hashed_nodes[..], true);
    get_proof(&stake_merkle, index)
}
