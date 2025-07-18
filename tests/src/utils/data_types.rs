use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Keypair};
use borsh::{BorshDeserialize, BorshSerialize};
use gov_v1::{MetaMerkleLeaf, StakeMerkleLeaf};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct MetaMerkleSnapshot {
    /// Hash of MetaMerkleTree
    pub root: [u8; 32],
    /// Each bundle contains the meta-level leaf, its stake-level leaves, and proof.
    pub leaf_bundles: Vec<MetaMerkleLeafBundle>,
    /// Slot where the tree was generated.
    pub slot: u64,
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

pub struct ProgramTestContext {
    pub payer: Keypair,
    pub program_config_pda: Pubkey,
    pub operators: Vec<Keypair>,
    pub meta_merkle_snapshot: MetaMerkleSnapshot,
}
