pub mod send_utils;

use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
        signer::Signer,
    },
    Client, ClientError, Cluster, Program,
};
use anyhow::Result;
use clap::Parser;
use cli::generate_meta_merkle_snapshot;
use gov_v1::{BallotBox, ProgramConfig};
use log::info;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tip_router_operator_cli::{
    cli::SnapshotPaths,
    ledger_utils::{get_bank_from_ledger, get_bank_from_snapshot_at_slot},
};

use crate::send_utils::{
    send_init_program_config, send_update_operator_whitelist, send_update_program_config,
};

fn parse_pubkey(s: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(s).map_err(|e| format!("invalid pubkey: {e}"))
}

#[derive(Clone, Debug)]
pub enum LogType {
    ProgramConfig,
}

/// Simple program to greet a person
#[derive(Clone, Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(short, long, env)]
    pub payer_path: String,

    #[arg(short, long, env)]
    pub authority_path: String,

    #[arg(short, long, env, default_value = "11111111111111111111111111111111")]
    pub operator_address: String,

    // #[arg(short, long, env, default_value = "http://localhost:8899")]
    // pub rpc_url: String,
    #[arg(short, long, env, default_value = "test-ledger")]
    pub ledger_path: PathBuf,

    #[arg(short, long, env, default_value = "tmp/full-snapshots")]
    pub full_snapshots_path: Option<PathBuf>,

    #[arg(short, long, env, default_value = "tmp/backup-snapshots")]
    pub backup_snapshots_dir: PathBuf,

    // #[arg(short, long, env, default_value = "tmp/snapshot-output")]
    // pub snapshot_output_dir: PathBuf,

    // #[arg(long, env, default_value = "false")]
    // pub submit_as_memo: bool,
    /// The price to pay for priority fee
    // #[arg(long, env, default_value_t = 1)]
    // pub micro_lamports: u64,

    // #[arg(long, env, help = "Path to save data (formerly meta-merkle-tree-dir)")]
    // pub save_path: Option<PathBuf>,

    // #[arg(long, env, default_value = "/tmp/claim_tips_epoch.txt")]
    // pub claim_tips_epoch_filepath: PathBuf,

    #[arg(long, env, default_value = "mainnet")]
    pub cluster: String,

    // #[arg(long, env, default_value = "local")]
    // pub region: String,

    // #[arg(long, env, default_value = "8899")]
    // pub localhost_port: u16,
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn get_snapshot_paths(&self) -> SnapshotPaths {
        let ledger_path = self.ledger_path.clone();
        let account_paths = None;
        let account_paths = account_paths.map_or_else(|| vec![ledger_path.clone()], |paths| paths);
        let full_snapshots_path = self.full_snapshots_path.clone();
        let full_snapshots_path = full_snapshots_path.map_or(ledger_path.clone(), |path| path);
        let incremental_snapshots_path = self.backup_snapshots_dir.clone();
        SnapshotPaths {
            ledger_path,
            account_paths,
            full_snapshots_path,
            incremental_snapshots_path,
            backup_snapshots_dir: self.backup_snapshots_dir.clone(),
        }
    }
}

fn parse_log_type(s: &str) -> Result<LogType, String> {
    match s.to_lowercase().as_str() {
        "program-config" => Ok(LogType::ProgramConfig),
        _ => Err(format!("invalid log type: {}", s)),
    }
}

#[derive(clap::Subcommand, Clone)]
pub enum Commands {
    SnapshotSlot {
        #[arg(long, env)]
        slot: u64,
    },
    GenerateMetaMerkle {
        #[arg(long, env)]
        slot: u64,

        #[arg(long, env)]
        epoch: u64,

        #[arg(long, env, default_value = "true")]
        save: bool,
    },
    InitProgramConfig {},
    UpdateOperatorWhitelist {
        #[arg(long, value_delimiter = ',', value_parser = parse_pubkey)]
        add: Option<Vec<Pubkey>>,

        #[arg(long, value_delimiter = ',', value_parser = parse_pubkey)]
        remove: Option<Vec<Pubkey>>,
    },
    UpdateProgramConfig {
        #[arg(long, value_parser = parse_pubkey)]
        new_authority: Option<Pubkey>,

        #[arg(long)]
        min_consensus_threshold_bps: Option<u16>,

        #[arg(long, value_parser = parse_pubkey)]
        tie_breaker_admin: Option<Pubkey>,

        #[arg(long)]
        vote_duration: Option<i64>,
    },
    InitBallotBox {},
    FinalizeBallot {},
    CastVote {},
    RemoveVote {},
    SetTieBreaker {},
    Log {
        // #[arg(long, help = "Pubkey of the account to fetch")]
        // pubkey: Pubkey,
        #[arg(long, value_parser = parse_log_type, help = "Account type: program-config | ballot-box")]
        ty: LogType,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        // === On-chain Instructions ===
        /// Initialize ProgramConfig on-chain
        Commands::Log { ty } => {
            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let client: Client<&Keypair> =
                Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
            let program: Program<&Keypair> = client.program(gov_v1::id()).unwrap();

            match ty {
                LogType::ProgramConfig => {
                    let data: ProgramConfig = program.account(ProgramConfig::pda().0)?;
                    println!("{:?}", data);
                }
            }
        }
        Commands::InitProgramConfig {} => {
            info!("InitProgramConfig...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let client: Client<&Keypair> =
                Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
            let program: Program<&Keypair> = client.program(gov_v1::id()).unwrap();

            let tx = send_init_program_config(&program, &authority, ProgramConfig::pda().0)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::UpdateOperatorWhitelist { add, remove } => {
            info!("UpdateOperatorWhitelist...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let client: Client<&Keypair> =
                Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
            let program: Program<&Keypair> = client.program(gov_v1::id()).unwrap();

            let tx = send_update_operator_whitelist(
                &program,
                &authority,
                ProgramConfig::pda().0,
                add,
                remove,
            )?;
            info!("Transaction sent: {}", tx);
        }
        Commands::UpdateProgramConfig {
            new_authority,
            min_consensus_threshold_bps,
            tie_breaker_admin,
            vote_duration,
        } => {
            info!("UpdateProgramConfig...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let client: Client<&Keypair> =
                Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
            let program: Program<&Keypair> = client.program(gov_v1::id()).unwrap();

            let tx = send_update_program_config(
                &program,
                &authority,
                ProgramConfig::pda().0,
                new_authority,
                min_consensus_threshold_bps,
                tie_breaker_admin,
                vote_duration,
            )?;
            info!("Transaction sent: {}", tx);
        }
        Commands::InitBallotBox {} => {}
        Commands::FinalizeBallot {} => {}
        Commands::CastVote {} => {}
        Commands::RemoveVote {} => {}
        Commands::SetTieBreaker {} => {}
        // === Snapshot Processing ===
        /// Create a snapshot at a specific slot
        Commands::SnapshotSlot { slot } => {
            info!("Snapshotting slot...");

            let save_snapshot = true;
            let SnapshotPaths {
                ledger_path,
                account_paths,
                full_snapshots_path,
                incremental_snapshots_path: _,
                backup_snapshots_dir,
            } = cli.get_snapshot_paths();

            get_bank_from_ledger(
                cli.operator_address,
                &ledger_path,
                account_paths,
                full_snapshots_path,
                backup_snapshots_dir.clone(),
                &slot,
                save_snapshot,
                backup_snapshots_dir,
                &cli.cluster,
            );
        }
        // TODO: Use `epoch` and `save` arg.
        /// Generates merkle tree from snapshot and save file locally
        Commands::GenerateMetaMerkle {
            epoch: _,
            slot,
            save: _,
        } => {
            let SnapshotPaths {
                ledger_path,
                account_paths,
                full_snapshots_path: _,
                incremental_snapshots_path: _,
                backup_snapshots_dir,
            } = cli.get_snapshot_paths();

            // We can safely expect to use the backup_snapshots_dir as the full snapshot path because
            //  _get_bank_from_snapshot_at_slot_ expects the snapshot at the exact `slot` to have
            //  already been taken.
            let bank = get_bank_from_snapshot_at_slot(
                slot,
                &backup_snapshots_dir,
                &backup_snapshots_dir,
                account_paths,
                ledger_path.as_path(),
            )?;

            let meta_merkle_snapshot = generate_meta_merkle_snapshot(&Arc::new(bank))?;

            let file_path = format!("./tmp/meta_merkle-{}.zip", slot);
            meta_merkle_snapshot.save_compressed(file_path.as_str())?;

            // TODO: publish file (e.g. upload to S3/IPFS/etc.)
        }
    }
    Ok(())
}
