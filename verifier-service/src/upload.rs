//! Upload handling for snapshot files

use anyhow::Result;
use axum::{
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use cli::MetaMerkleSnapshot;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_program::hash::hash;

use tracing::info;

use crate::auth_middleware::extract_bearer_token;
use crate::indexer::index_snapshot_data;
use crate::state::{AppState, TOKEN_EXPIRY_SECONDS};

#[derive(Deserialize)]
pub struct AuthRequest {
    slot: u64,
    merkle_root: String,
    signature: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
    expires_in: u64,
}

/// Handle POST /upload/auth endpoint
pub async fn handle_upload_auth(
    State(app_state): State<AppState>,
    Json(auth_req): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    info!(
        "POST /upload/auth - Authentication requested for slot {}",
        auth_req.slot
    );

    // Verify signature over slot || merkle_root
    verify_signature(&auth_req.slot, &auth_req.merkle_root, &auth_req.signature)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Create token and store validated metadata
    let token = app_state
        .token_store
        .create_token(auth_req.slot, auth_req.merkle_root);

    info!(
        "Authentication successful for slot {}, token created",
        auth_req.slot
    );

    Ok(Json(AuthResponse {
        token,
        expires_in: TOKEN_EXPIRY_SECONDS,
    }))
}

/// Handle POST /upload endpoint
pub async fn handle_upload(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, StatusCode> {
    info!("POST /upload - Processing upload request");

    // 1. Extract token from header 
    let token = extract_bearer_token(&headers)?;

    // 2. Consume token (already validated by middleware)
    let (slot, merkle_root) = app_state.token_store.consume_validated_token(token).ok_or_else(|| {
        info!("Token was already consumed or doesn't exist");
        StatusCode::UNAUTHORIZED
    })?;

    info!(
        "Upload authenticated for slot={}, merkle_root={}",
        slot, merkle_root
    );

    // 3. Extract network and file data
    let (network, file_data) = extract_fields(&mut multipart).await.map_err(|e| {
        info!("Failed to extract fields: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    info!(
        "Token validated, processing file ({} bytes)",
        file_data.len()
    );

    // 4. Parse snapshot file, verify merkle_root and slot from request fields.
    let snapshot_hash = bs58::encode(hash(&file_data)).into_string();
    let snapshot = MetaMerkleSnapshot::read_from_bytes(file_data, true).map_err(|e| {
        info!("Failed to read snapshot: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    if bs58::encode(snapshot.root).into_string() != merkle_root || snapshot.slot != slot {
        info!("Merkle root or slot in snapshot mismatch");
        return Err(StatusCode::BAD_REQUEST);
    }

    // 5. Index data in database
    index_snapshot_data(&app_state.db_path, &snapshot, &network, &merkle_root, &snapshot_hash)
        .await
        .map_err(|e| {
            info!("Failed to index snapshot data: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "status": "success",
        "slot": slot,
        "merkle_root": merkle_root,
    })))
}

/// Extract metadata fields in sequence.
async fn extract_fields(multipart: &mut Multipart) -> Result<(String, Vec<u8>)> {
    let mut network_opt = None;
    let mut file_opt = None;

    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("network") => {
                network_opt = Some(field.text().await?);
            }
            Some("file") => {
                file_opt = Some(field.bytes().await?.to_vec());
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unexpected field: {}",
                    field.name().unwrap_or("")
                ));
            }
        }
    }

    let network = network_opt.ok_or_else(|| anyhow::anyhow!("Missing network field"))?;
    let file = file_opt.ok_or_else(|| anyhow::anyhow!("Missing file field"))?;

    Ok((network, file))
}

/// Verify Ed25519 signature over slot || merkle_root
fn verify_signature(slot: &u64, merkle_root: &str, signature: &str) -> Result<()> {
    // TODO: Implement Ed25519 signature verification
    // 1. Get operator pubkey from environment/config
    // 2. Construct message: slot.to_le_bytes() || merkle_root.as_bytes()
    // 3. Decode base58 signature
    // 4. Verify signature using ed25519-dalek

    info!(
        "TODO: Verify signature for slot={}, merkle_root={}, sig={}",
        slot, merkle_root, signature
    );
    Ok(())
}
