use anyhow::Result;
use rusqlite::{params, Connection, Row};
use serde_json;
use tracing::debug;

use super::models::*;

/// Database operations for vote accounts
impl VoteAccountRecord {
    pub fn insert(&self, conn: &Connection) -> Result<()> {
        debug!(
            "Inserting vote account: {} for slot {}",
            self.vote_account, self.snapshot_slot
        );

        conn.execute(
            "INSERT OR REPLACE INTO vote_accounts 
             (network, snapshot_slot, vote_account, voting_wallet, stake_merkle_root, active_stake, meta_merkle_proof)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                self.network,
                self.snapshot_slot,
                self.vote_account,
                self.voting_wallet,
                self.stake_merkle_root,
                self.active_stake,
                serde_json::to_string(&self.meta_merkle_proof)?
            ],
        )?;

        Ok(())
    }

    /// Get vote account summaries filtered by voting wallet
    pub fn get_summary_by_voting_wallet(
        conn: &Connection,
        network: &str,
        voting_wallet: &str,
        snapshot_slot: u64,
    ) -> Result<Vec<VoteAccountSummary>> {
        let sql = "SELECT vote_account, active_stake FROM vote_accounts \
                   WHERE network = ? AND voting_wallet = ? AND snapshot_slot = ? \
                   ORDER BY vote_account";
        let params = vec![
            network.to_string(),
            voting_wallet.to_string(),
            snapshot_slot.to_string(),
        ];

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok(VoteAccountSummary {
                vote_account: row.get("vote_account")?,
                active_stake: row.get("active_stake")?,
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }

        Ok(records)
    }

    /// Get vote account by specific account, network and snapshot slot
    pub fn get_by_account(
        conn: &Connection,
        network: &str,
        vote_account: &str,
        snapshot_slot: u64,
    ) -> Result<Option<VoteAccountRecord>> {
        let sql = "SELECT * FROM vote_accounts \
                   WHERE network = ? AND vote_account = ? AND snapshot_slot = ?";
        let params = vec![
            network.to_string(),
            vote_account.to_string(),
            snapshot_slot.to_string(),
        ];

        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Self::from_row(row)
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let meta_merkle_proof_json: String = row.get("meta_merkle_proof")?;

        Ok(VoteAccountRecord {
            network: row.get("network")?,
            snapshot_slot: row.get("snapshot_slot")?,
            vote_account: row.get("vote_account")?,
            voting_wallet: row.get("voting_wallet")?,
            stake_merkle_root: row.get("stake_merkle_root")?,
            active_stake: row.get("active_stake")?,
            meta_merkle_proof: serde_json::from_str(&meta_merkle_proof_json).unwrap_or_default(),
        })
    }
}

/// Database operations for stake accounts
impl StakeAccountRecord {
    pub fn insert(&self, conn: &Connection) -> Result<()> {
        debug!(
            "Inserting stake account: {} for slot {}",
            self.stake_account, self.snapshot_slot
        );

        conn.execute(
            "INSERT OR REPLACE INTO stake_accounts 
             (network, snapshot_slot, stake_account, vote_account, voting_wallet, active_stake, stake_merkle_proof)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                self.network,
                self.snapshot_slot,
                self.stake_account,
                self.vote_account,
                self.voting_wallet,
                self.active_stake,
                serde_json::to_string(&self.stake_merkle_proof)?
            ],
        )?;

        Ok(())
    }

    /// Get stake account summaries filtered by voting wallet
    pub fn get_summary_by_voting_wallet(
        conn: &Connection,
        network: &str,
        voting_wallet: &str,
        snapshot_slot: u64,
    ) -> Result<Vec<StakeAccountSummary>> {
        let sql = "SELECT stake_account, vote_account, active_stake FROM stake_accounts \
                   WHERE network = ? AND voting_wallet = ? AND snapshot_slot = ? \
                   ORDER BY stake_account";
        let params = vec![
            network.to_string(),
            voting_wallet.to_string(),
            snapshot_slot.to_string(),
        ];

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok(StakeAccountSummary {
                stake_account: row.get("stake_account")?,
                vote_account: row.get("vote_account")?,
                active_stake: row.get("active_stake")?,
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }

        Ok(records)
    }

    /// Get stake account by specific account, network and snapshot slot
    pub fn get_by_account(
        conn: &Connection,
        network: &str,
        stake_account: &str,
        snapshot_slot: u64,
    ) -> Result<Option<StakeAccountRecord>> {
        let sql = "SELECT * FROM stake_accounts \
                   WHERE network = ? AND stake_account = ? AND snapshot_slot = ?";
        let params = vec![
            network.to_string(),
            stake_account.to_string(),
            snapshot_slot.to_string(),
        ];

        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Self::from_row(row)
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let stake_merkle_proof_json: String = row.get("stake_merkle_proof")?;

        Ok(StakeAccountRecord {
            network: row.get("network")?,
            snapshot_slot: row.get("snapshot_slot")?,
            stake_account: row.get("stake_account")?,
            vote_account: row.get("vote_account")?,
            voting_wallet: row.get("voting_wallet")?,
            active_stake: row.get("active_stake")?,
            stake_merkle_proof: serde_json::from_str(&stake_merkle_proof_json).unwrap_or_default(),
        })
    }
}

/// Database operations for snapshot metadata
impl SnapshotMetaRecord {
    pub fn insert(&self, conn: &Connection) -> Result<()> {
        debug!(
            "Inserting snapshot meta for slot {} on network {}",
            self.slot, self.network
        );

        conn.execute(
            "INSERT OR REPLACE INTO snapshot_meta 
             (network, slot, merkle_root, snapshot_hash, created_at)
             VALUES (?, ?, ?, ?, ?)",
            params![
                self.network,
                self.slot,
                self.merkle_root,
                self.snapshot_hash,
                self.created_at
            ],
        )?;

        Ok(())
    }

    /// Get the latest snapshot metadata for a network
    pub fn get_latest(conn: &Connection, network: &str) -> Result<Option<SnapshotMetaRecord>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM snapshot_meta \
             WHERE network = ? \
             ORDER BY slot DESC \
             LIMIT 1",
        )?;

        let mut rows = stmt.query_map([network], |row| {
            Ok(SnapshotMetaRecord {
                network: row.get("network")?,
                slot: row.get("slot")?,
                merkle_root: row.get("merkle_root")?,
                snapshot_hash: row.get("snapshot_hash")?,
                created_at: row.get("created_at")?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }
}
