mod database;
mod middleware;
mod types;
mod upload;
mod utils;

use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use database::{constants::DEFAULT_DB_PATH, init_pool, models::*, operations::db_operation};
use serde_json::{json, Value};
use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tracing::info;
use types::{NetworkQuery, VoterQuery};
use upload::handle_upload;

use crate::{middleware::inject_client_ip, utils::validate_network};

// Get the latest snapshot slot if not specified
async fn get_snapshot_slot(
    pool: &SqlitePool,
    network: &str,
    requested_slot: Option<u64>,
) -> Result<u64, StatusCode> {
    match requested_slot {
        Some(s) => Ok(s),
        None => db_operation(
            || SnapshotMetaRecord::get_latest_slot(pool, network),
            "Failed to get latest slot",
        )
        .await?
        .ok_or(StatusCode::NOT_FOUND),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Governance Merkle Verifier Service");

    // Check if OPERATOR_PUBKEY is set
    let operator_pubkey = std::env::var("OPERATOR_PUBKEY");
    if operator_pubkey.is_err() {
        anyhow::bail!("OPERATOR_PUBKEY is not set");
    }

    // Initialize database pool (create tables, run migrations)
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let pool = init_pool(&db_path).await?;
    info!("Database initialized successfully");

    // Helper closure to define rate limiters
    // Arguments:
    //  per_second: interval per refill to the bucket (in seconds)
    //  burst_size: starting number of requests in the bucket per IP
    let rl = |per_second: u64, burst_size: u32| {
        Arc::new(
            GovernorConfigBuilder::default()
                .per_second(per_second)
                .burst_size(burst_size)
                .finish()
                .unwrap(),
        )
    };

    // Rate limiting configs
    let global_rl = rl(10, 10);
    let upload_rl = rl(60, 2);

    // Requests to /upload consume from both global and upload rate limits.
    let upload_router = Router::new()
        .route("/", post(handle_upload))
        .layer(axum::middleware::from_fn(inject_client_ip))
        .layer(GovernorLayer { config: upload_rl });

    // Build application with routes
    let app = Router::new()
        .route("/healthz", get(health_check))
        .route("/meta", get(get_meta))
        .nest("/upload", upload_router)
        .route("/voter/{voting_wallet}", get(get_voter_summary))
        .route("/proof/vote_account/{vote_account}", get(get_vote_proof))
        .route("/proof/stake_account/{stake_account}", get(get_stake_proof))
        .layer(axum::middleware::from_fn(inject_client_ip))
        .layer(GovernorLayer { config: global_rl })
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB limit for snapshot uploads
        .with_state(pool);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        // Include soc
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn health_check() -> &'static str {
    info!("GET /healthz - Health check requested");
    "ok"
}

async fn get_meta(
    State(pool): State<SqlitePool>,
    Query(params): Query<NetworkQuery>,
) -> Result<Json<SnapshotMetaRecord>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;

    info!("GET /meta - for network: {}", network);

    let meta_record_option = db_operation(
        || SnapshotMetaRecord::get_latest(&pool, &network),
        "Failed to get snapshot meta record",
    )
    .await?;

    if let Some(record) = meta_record_option {
        Ok(Json(record))
    } else {
        info!("No snapshots found for network: {}", network);
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_voter_summary(
    State(pool): State<SqlitePool>,
    Path(voting_wallet): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;

    info!("GET /voter/{} - for network: {}", voting_wallet, network);

    let snapshot_slot = get_snapshot_slot(&pool, &network, params.slot).await?;

    // Get vote account summaries
    let vote_accounts = db_operation(
        || {
            VoteAccountRecord::get_summary_by_voting_wallet(
                &pool,
                &network,
                &voting_wallet,
                snapshot_slot,
            )
        },
        "Failed to get vote accounts",
    )
    .await?;

    // Get stake account summaries
    let stake_accounts = db_operation(
        || {
            StakeAccountRecord::get_summary_by_voting_wallet(
                &pool,
                &network,
                &voting_wallet,
                snapshot_slot,
            )
        },
        "Failed to get stake accounts",
    )
    .await?;

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
    State(pool): State<SqlitePool>,
    Path(vote_account): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;

    info!(
        "GET /proof/vote_account/{} - for network: {}",
        vote_account, network
    );

    let snapshot_slot = get_snapshot_slot(&pool, &network, params.slot).await?;

    // Get vote account record from database
    let vote_record_option = db_operation(
        || VoteAccountRecord::get_by_account(&pool, &network, &vote_account, snapshot_slot),
        "Failed to get vote account record",
    )
    .await?;

    if let Some(vote_record) = vote_record_option {
        let meta_merkle_leaf = json!({
            "voting_wallet": vote_record.voting_wallet,
            "vote_account": vote_record.vote_account,
            "stake_merkle_root": vote_record.stake_merkle_root,
            "active_stake": vote_record.active_stake
        });

        Ok(Json(json!({
            "network": network,
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
    State(pool): State<SqlitePool>,
    Path(stake_account): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.unwrap_or_else(|| "mainnet".to_string());
    validate_network(&network)?;

    info!(
        "GET /proof/stake_account/{} - for network: {}",
        stake_account, network
    );

    let snapshot_slot = get_snapshot_slot(&pool, &network, params.slot).await?;

    // Get stake account record from database
    let stake_record_option = db_operation(
        || StakeAccountRecord::get_by_account(&pool, &network, &stake_account, snapshot_slot),
        "Failed to get stake account record",
    )
    .await?;

    if let Some(stake_record) = stake_record_option {
        let stake_merkle_leaf = json!({
            "voting_wallet": stake_record.voting_wallet,
            "stake_account": stake_record.stake_account,
            "active_stake": stake_record.active_stake
        });

        Ok(Json(json!({
            "network": network,
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
