mod auth_middleware;
mod database;
mod indexer;
mod state;
mod upload;
mod utils;

use std::net::SocketAddr;

use auth_middleware::auth_middleware;
use axum::{
    extract::{DefaultBodyLimit, Path},
    http::StatusCode,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use database::{constants::DEFAULT_DB_PATH, Database};
use serde_json::{json, Value};
use tracing::info;
use state::{AppState, TokenStore};
use upload::{handle_upload, handle_upload_auth};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Governance Merkle Verifier Service");

    // Initialize database (create tables, run migrations)
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let _db = Database::new(&db_path)?;
    info!("Database initialized successfully");

    // Create shared app state
    let app_state = AppState {
        db_path: db_path.clone(),
        token_store: TokenStore::new(),
    };

    // Build application with routes
    // TODO: Current approach passes db_path and creates connections per-request.
    // For high QPS, replace with SQLx connection pool for better performance.
    // TODO: Add rate limiting middleware to prevent DoS attacks (e.g., 10 requests/min per IP)
    let app = Router::new()
        .route("/healthz", get(health_check))
        .route("/meta", get(get_meta))
        .route("/upload/auth", post(handle_upload_auth))
        .route("/upload", post(handle_upload))
        .route("/voter/{voting_wallet}", get(get_voter_summary))
        .route("/proof/vote_account/{vote_account}", get(get_vote_proof))
        .route("/proof/stake_account/{stake_account}", get(get_stake_proof))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB limit for snapshot uploads
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ))
        .with_state(app_state);

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
