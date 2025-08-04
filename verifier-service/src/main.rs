mod database;
mod upload;

use std::net::SocketAddr;

use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use database::{constants::DEFAULT_DB_PATH, Database};
use serde_json::{json, Value};
use tracing::info;
use upload::handle_upload;

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
    let app = Router::new()
        .route("/healthz", get(health_check))
        .route("/meta", get(get_meta))
        .route("/upload", post(handle_upload))
        .route("/voter/{voting_wallet}", get(get_voter_summary))
        .route("/proof/vote_account/{vote_account}", get(get_vote_proof))
        .route("/proof/stake_account/{stake_account}", get(get_stake_proof))
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

// Placeholder handlers - will implement in subsequent tasks
async fn get_meta() -> Result<Json<Value>, StatusCode> {
    info!("GET /meta - Metadata requested");
    Ok(Json(json!({
        "slot": 0,
        "merkle_root": "",
        "signature": "",
        "created_at": ""
    })))
}

async fn get_voter_summary(Path(voting_wallet): Path<String>) -> Result<Json<Value>, StatusCode> {
    info!("GET /voter/{} - Voter summary requested", voting_wallet);
    Ok(Json(json!({
        "snapshot_slot": 0,
        "vote_accounts": [],
        "stake_accounts": []
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
