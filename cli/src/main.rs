use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
    },
    Client, Cluster, Program,
};
use anyhow::Result;
use clap::Parser;
use cli::{generate_meta_merkle_snapshot, utils::*};
use gov_v1::{Ballot, BallotBox, ConsensusResult, MetaMerkleProof, ProgramConfig};
use log::info;
use std::path::PathBuf;
use std::sync::Arc;
use tip_router_operator_cli::{
    cli::SnapshotPaths,
    ledger_utils::{get_bank_from_ledger, get_bank_from_snapshot_at_slot},
};

#[derive(Clone, Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(short, long, env)]
    pub payer_path: String,

    #[arg(short, long, env)]
    pub authority_path: String,

    #[arg(short, long, env, default_value = "11111111111111111111111111111111")]
    pub operator_address: String,

    #[arg(short, long, env, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    #[arg(short, long, env, default_value = "test-ledger")]
    pub ledger_path: PathBuf,

    #[arg(short, long, env, default_value = "tmp/full-snapshots")]
    pub full_snapshots_path: Option<PathBuf>,

    #[arg(short, long, env, default_value = "tmp/backup-snapshots")]
    pub backup_snapshots_dir: PathBuf,

    #[arg(long, env, default_value = "mainnet")]
    pub cluster: String,

    // #[arg(short, long, env, default_value = "tmp/snapshot-output")]
    // pub snapshot_output_dir: PathBuf,

    // #[arg(long, env, default_value_t = 1)]
    // pub micro_lamports: u64,

    // #[arg(long, env, help = "Path to save data (formerly meta-merkle-tree-dir)")]
    // pub save_path: Option<PathBuf>,
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
    FinalizeBallot {
        #[arg(long, help = "Id of ballot box")]
        id: u64,
    },
    CastVote {
        #[arg(long, help = "Id of ballot box")]
        id: u64,
        #[arg(long, value_parser = parse_base_58_32, help = "Meta merkle tree root, base-58 encoded.")]
        root: [u8; 32],
        #[arg(long, value_parser = parse_base_58_32, help = "SHA256 hash of the meta merkle snapshot, base-58 encoded.")]
        hash: [u8; 32],
    },
    RemoveVote {
        #[arg(long, help = "Id of ballot box")]
        id: u64,
    },
    SetTieBreaker {
        #[arg(long, help = "Id of ballot box")]
        id: u64,
        #[arg(long, help = "Index in ballot tallies to set as winning ballot")]
        idx: u8,
    },
    Log {
        #[arg(long, help = "Id of ballot box to fetch")]
        id: Option<u64>,
        #[arg(long, value_parser = parse_pubkey)]
        vote_account: Option<Pubkey>,
        #[arg(long, value_parser = parse_log_type, help = "Account type: program-config | ballot-box | consensus-result | proof")]
        ty: LogType,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    fn load_client_program(payer: &Keypair, rpc_url: String) -> Program<&Keypair> {
        let client: Client<&Keypair> = Client::new_with_options(
            Cluster::Custom(rpc_url.clone(), rpc_url),
            payer,
            CommitmentConfig::confirmed(),
        );
        client.program(gov_v1::id()).unwrap()
    }

    match cli.command {
        // === On-chain Instructions ===
        Commands::Log {
            id,
            vote_account,
            ty,
        } => {
            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let program = load_client_program(&payer, cli.rpc_url);

            match ty {
                LogType::ProgramConfig => {
                    let data: ProgramConfig = program.account(ProgramConfig::pda().0)?;
                    println!("{:?}", data);
                }
                LogType::BallotBox => {
                    let data: BallotBox =
                        program.account(BallotBox::pda(id.expect("Missing --id argument")).0)?;
                    println!("{:?}", data);
                }
                LogType::ConsensusResult => {
                    let data: ConsensusResult = program
                        .account(ConsensusResult::pda(id.expect("Missing --id argument")).0)?;
                    println!("{:?}", data);
                }
                LogType::MetaMerkleProof => {
                    let consensus_result_pda =
                        ConsensusResult::pda(id.expect("Missing --id argument")).0;
                    let data: MetaMerkleProof = program.account(
                        MetaMerkleProof::pda(
                            &consensus_result_pda,
                            &vote_account.expect("Missing --vote-account argument"),
                        )
                        .0,
                    )?;
                    println!("{:?}", data);
                }
            }
        }
        Commands::InitProgramConfig {} => {
            info!("InitProgramConfig...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let program = load_client_program(&payer, cli.rpc_url);

            let tx = send_init_program_config(&program, &authority, ProgramConfig::pda().0)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::UpdateOperatorWhitelist { add, remove } => {
            info!("UpdateOperatorWhitelist...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let program = load_client_program(&payer, cli.rpc_url);

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
            let program = load_client_program(&payer, cli.rpc_url);

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
        Commands::InitBallotBox {} => {
            info!("InitBallotBox...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let program = load_client_program(&payer, cli.rpc_url);

            let program_config_pda = ProgramConfig::pda().0;
            let program_config: ProgramConfig = program.account(program_config_pda)?;
            let ballot_box_pda = BallotBox::pda(program_config.next_ballot_id).0;
            let tx =
                send_init_ballot_box(&program, &authority, program_config_pda, ballot_box_pda)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::CastVote { id, root, hash } => {
            info!("CastVote...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let program = load_client_program(&payer, cli.rpc_url);

            let program_config_pda = ProgramConfig::pda().0;
            let ballot_box_pda = BallotBox::pda(id).0;
            let tx = send_cast_vote(
                &program,
                &authority,
                program_config_pda,
                ballot_box_pda,
                Ballot {
                    meta_merkle_root: root,
                    snapshot_hash: hash,
                },
            )?;
            info!("Transaction sent: {}", tx);
        }
        Commands::RemoveVote { id } => {
            info!("RemoveVote...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let program = load_client_program(&payer, cli.rpc_url);

            let program_config_pda = ProgramConfig::pda().0;
            let ballot_box_pda = BallotBox::pda(id).0;
            let tx = send_remove_vote(&program, &authority, program_config_pda, ballot_box_pda)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::SetTieBreaker { id, idx } => {
            info!("SetTieBreaker...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
            let program = load_client_program(&payer, cli.rpc_url);

            let program_config_pda = ProgramConfig::pda().0;
            let ballot_box_pda = BallotBox::pda(id).0;
            let tx = send_set_tie_breaker(
                &program,
                &authority,
                ballot_box_pda,
                program_config_pda,
                idx,
            )?;
            info!("Transaction sent: {}", tx);
        }
        Commands::FinalizeBallot { id } => {
            info!("FinalizeBallot...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let program = load_client_program(&payer, cli.rpc_url);

            let ballot_box_pda = BallotBox::pda(id).0;
            let consensus_result_pda = ConsensusResult::pda(id).0;
            let tx = send_finalize_ballot(&program, ballot_box_pda, consensus_result_pda)?;
            info!("Transaction sent: {}", tx);
        }
        // === Snapshot Processing ===
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
