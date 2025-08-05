//! Shared application state and token management

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Token expiration time in seconds (5 minutes)
pub const TOKEN_EXPIRY_SECONDS: u64 = 300;

/// Token store for managing upload authentication tokens
#[derive(Clone)]
pub struct TokenStore {
    pub tokens: Arc<Mutex<HashMap<String, TokenData>>>,
}

#[derive(Clone)]
pub struct TokenData {
    pub slot: u64,
    pub merkle_root: String,
    pub expires_at: u64,
}

impl TokenStore {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn create_token(&self, slot: u64, merkle_root: String) -> String {
        let token = uuid::Uuid::new_v4().simple().to_string();
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + TOKEN_EXPIRY_SECONDS;
        
        let data = TokenData { slot, merkle_root, expires_at };
        self.tokens.lock().unwrap().insert(token.clone(), data);
        token
    }
    
    /// Check if token exists and is valid (doesn't consume it)
    pub fn is_token_valid(&self, token: &str) -> bool {
        let tokens = self.tokens.lock().unwrap();
        if let Some(token_data) = tokens.get(token) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now <= token_data.expires_at
        } else {
            false
        }
    }

    /// Consume token that was already validated by middleware
    pub fn consume_validated_token(&self, token: &str) -> Option<(u64, String)> {
        let mut tokens = self.tokens.lock().unwrap();
        if let Some(data) = tokens.remove(token) {
            Some((data.slot, data.merkle_root))
        } else {
            None
        }
    }
    
    // TODO: Cleanup functionality can be added later when needed
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db_path: String,
    pub token_store: TokenStore,
}