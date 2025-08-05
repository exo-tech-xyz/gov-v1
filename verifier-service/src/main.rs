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

// Health check endpoint
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
    
    info!("GET /meta - Metadata requested for network: {}", network);

    let conn = Connection::open(&db_path).map_err(|e| {
        info!("Failed to open database: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match SnapshotMetaRecord::get_latest(&conn, &network) {
        Ok(Some(record)) => {
            info!("Found latest snapshot for network {}: slot {}", network, record.slot);
            Ok(Json(record))
        }
        Ok(None) => {
            info!("No snapshots found for network: {}", network);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            info!("Database error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_voter_summary(
    State(db_path): State<String>,
    Path(voting_wallet): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;
    
    info!("GET /voter/{} - Voter summary requested for network: {}", voting_wallet, network);

    let conn = Connection::open(&db_path).map_err(|e| {
        info!("Failed to open database: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get the latest snapshot slot if not specified
    let snapshot_slot = if let Some(slot) = params.slot {
        slot
    } else {
        match SnapshotMetaRecord::get_latest(&conn, &network) {
            Ok(Some(record)) => record.slot,
            Ok(None) => {
                info!("No snapshots found for network: {}", network);
                return Err(StatusCode::NOT_FOUND);
            }
            Err(e) => {
                info!("Database error getting latest snapshot: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    };

    // Get vote accounts for this voting wallet
    let vote_accounts = VoteAccountRecord::get_by_voting_wallet(&conn, &network, &voting_wallet, snapshot_slot)
        .map_err(|e| {
            info!("Failed to get vote accounts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get stake accounts for this voting wallet
    let stake_accounts = StakeAccountRecord::get_by_voting_wallet(&conn, &network, &voting_wallet, snapshot_slot)
        .map_err(|e| {
            info!("Failed to get stake accounts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Found {} vote accounts and {} stake accounts for voting wallet {}", 
          vote_accounts.len(), stake_accounts.len(), voting_wallet);

    Ok(Json(json!({
        "network": network,
        "snapshot_slot": snapshot_slot,
        "voting_wallet": voting_wallet,
        "vote_accounts": vote_accounts,
        "stake_accounts": stake_accounts
    })))
}

async fn get_vote_proof(Path(vote_account): Path<String>) -> Result<Json<Value>, StatusCode> {
    info!(
        "GET /proof/vote_account/{} - Vote account proof requested",
        vote_account
    );
    Ok(Json(json!({
        "snapshot_slot": 0,
        "meta_merkle_leaf": {},
        "meta_merkle_proof": []
    })))
}

async fn get_stake_proof(Path(stake_account): Path<String>) -> Result<Json<Value>, StatusCode> {
    info!(
        "GET /proof/stake_account/{} - Stake account proof requested",
        stake_account
    );
    Ok(Json(json!({
        "snapshot_slot": 0,
        "stake_merkle_leaf": {},
        "stake_merkle_proof": [],
        "vote_account": ""
    })))
}
