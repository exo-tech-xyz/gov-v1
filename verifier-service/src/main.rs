mod database;
mod upload;
mod utils;

use std::net::SocketAddr;

use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use database::{constants::DEFAULT_DB_PATH, models::*, Database};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;
use upload::handle_upload;

use crate::utils::validate_network;

// Helper functions for shared endpoint logic
fn get_db_connection(db_path: &str) -> Result<Connection, StatusCode> {
    Connection::open(db_path).map_err(|e| {
        info!("Failed to open database: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

// Get the latest snapshot slot if not specified
fn get_snapshot_slot(
    conn: &Connection,
    network: &str,
    requested_slot: Option<u64>,
) -> Result<u64, StatusCode> {
    if let Some(slot) = requested_slot {
        Ok(slot)
    } else {
        let record_option = db_operation(
            || SnapshotMetaRecord::get_latest(conn, network),
            "Database error getting latest snapshot",
        )?;

        if let Some(record) = record_option {
            Ok(record.slot)
        } else {
            info!("No snapshots found for network: {}", network);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

// Helper to wrap database operations with consistent error handling
fn db_operation<T, F>(operation: F, error_msg: &str) -> Result<T, StatusCode>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    operation().map_err(|e| {
        info!("{}: {}", error_msg, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(Debug, Deserialize)]
struct NetworkQuery {
    network: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VoterQuery {
    network: Option<String>,
    slot: Option<u64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Governance Merkle Verifier Service");

    // Initialize database (create tables, run migrations)
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let _db = Database::new(&db_path)?;
    info!("Database initialized successfully");

    // Build application with route
    // TODO: Current approach passes db_path and creates connections per-request.
    // For high QPS, replace with SQLx connection pool for better performance.
    // TODO: Add rate limiting middleware to prevent DoS attacks (e.g., 10 requests/min per IP)
    let app = Router::new()
        .route("/healthz", get(health_check))
        .route("/meta", get(get_meta))
        .route("/upload", post(handle_upload))
        .route("/voter/{voting_wallet}", get(get_voter_summary))
        .route("/proof/vote_account/{vote_account}", get(get_vote_proof))
        .route("/proof/stake_account/{stake_account}", get(get_stake_proof))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB limit for snapshot uploads
        .with_state(db_path);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// TODO: Health check endpoint
async fn health_check() -> &'static str {
    info!("GET /healthz - Health check requested");
    "ok"
}

async fn get_meta(
    State(db_path): State<String>,
    Query(params): Query<NetworkQuery>,
) -> Result<Json<SnapshotMetaRecord>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;

    info!("GET /meta - for network: {}", network);

    let conn = get_db_connection(&db_path)?;
    let meta_record_option = db_operation(
        || SnapshotMetaRecord::get_latest(&conn, &network),
        "Failed to get snapshot meta record",
    )?;

    if let Some(record) = meta_record_option {
        Ok(Json(record))
    } else {
        info!("No snapshots found for network: {}", network);
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_voter_summary(
    State(db_path): State<String>,
    Path(voting_wallet): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;

    info!("GET /voter/{} - for network: {}", voting_wallet, network);

    let conn = get_db_connection(&db_path)?;
    let snapshot_slot = get_snapshot_slot(&conn, &network, params.slot)?;

    // Get vote accounts for this voting wallet
    let vote_accounts = db_operation(
        || VoteAccountRecord::get_by_voting_wallet(&conn, &network, &voting_wallet, snapshot_slot),
        "Failed to get vote accounts",
    )?;

    // Get stake accounts for this voting wallet
    let stake_accounts = db_operation(
        || StakeAccountRecord::get_by_voting_wallet(&conn, &network, &voting_wallet, snapshot_slot),
        "Failed to get stake accounts",
    )?;

    info!(
        "Found {} vote accounts and {} stake accounts for voting wallet {}",
        vote_accounts.len(),
        stake_accounts.len(),
        voting_wallet
    );

    Ok(Json(json!({
        "network": network,
        "snapshot_slot": snapshot_slot,
        "voting_wallet": voting_wallet,
        "vote_accounts": vote_accounts,
        "stake_accounts": stake_accounts
    })))
}

async fn get_vote_proof(
    State(db_path): State<String>,
    Path(vote_account): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;

    info!(
        "GET /proof/vote_account/{} - for network: {}",
        vote_account, network
    );

    let conn = get_db_connection(&db_path)?;
    let snapshot_slot = get_snapshot_slot(&conn, &network, params.slot)?;

    // Get vote account record from database
    let vote_record_option = db_operation(
        || VoteAccountRecord::get_by_account(&conn, &network, &vote_account, snapshot_slot),
        "Failed to get vote account record",
    )?;

    if let Some(vote_record) = vote_record_option {
        let meta_merkle_leaf = json!({
            "voting_wallet": vote_record.voting_wallet,
            "vote_account": vote_record.vote_account,
            "stake_merkle_root": vote_record.stake_merkle_root,
            "active_stake": vote_record.active_stake
        });

        Ok(Json(json!({
            "snapshot_slot": snapshot_slot,
            "meta_merkle_leaf": meta_merkle_leaf,
            "meta_merkle_proof": vote_record.meta_merkle_proof
        })))
    } else {
        info!(
            "Vote account {} not found for network {} at slot {}",
            vote_account, network, snapshot_slot
        );
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_stake_proof(
    State(db_path): State<String>,
    Path(stake_account): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;

    info!(
        "GET /proof/stake_account/{} - for network: {}",
        stake_account, network
    );

    let conn = get_db_connection(&db_path)?;
    let snapshot_slot = get_snapshot_slot(&conn, &network, params.slot)?;

    // Get stake account record from database
    let stake_record_option = db_operation(
        || StakeAccountRecord::get_by_account(&conn, &network, &stake_account, snapshot_slot),
        "Failed to get stake account record",
    )?;

    if let Some(stake_record) = stake_record_option {
        let stake_merkle_leaf = json!({
            "voting_wallet": stake_record.voting_wallet,
            "stake_account": stake_record.stake_account,
            "active_stake": stake_record.active_stake
        });

        Ok(Json(json!({
            "snapshot_slot": snapshot_slot,
            "stake_merkle_leaf": stake_merkle_leaf,
            "stake_merkle_proof": stake_record.stake_merkle_proof,
            "vote_account": stake_record.vote_account
        })))
    } else {
        info!(
            "Stake account {} not found for network {} at slot {}",
            stake_account, network, snapshot_slot
        );
        Err(StatusCode::NOT_FOUND)
    }
}
