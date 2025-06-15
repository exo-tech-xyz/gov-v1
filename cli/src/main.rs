use clap::Parser;
use log::info;
use std::path::PathBuf;
use tip_router_operator_cli::cli::SnapshotPaths;

/// Simple program to greet a person
#[derive(Clone, Parser)]
#[command(author, version, about)]
struct Cli {
    // #[arg(short, long, env)]
    // pub keypair_path: String,
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

#[derive(clap::Subcommand, Clone)]
pub enum Commands {
    SnapshotSlot {
        #[arg(long, env)]
        slot: u64,
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
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

            tip_router_operator_cli::ledger_utils::get_bank_from_ledger(
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
    }
}
