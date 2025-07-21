use anchor_client::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn parse_pubkey(s: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(s).map_err(|e| format!("invalid pubkey: {e}"))
}

pub fn parse_hex_32(s: &str) -> Result<[u8; 32], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| format!("Invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("Expected 32 bytes, got {}", bytes.len()));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

pub fn parse_log_type(s: &str) -> Result<LogType, String> {
    match s.to_lowercase().as_str() {
        "program-config" => Ok(LogType::ProgramConfig),
        "ballot-box" => Ok(LogType::BallotBox),
        "consensus-result" => Ok(LogType::ConsensusResult),
        "proof" => Ok(LogType::MetaMerkleProof),
        _ => Err(format!("invalid log type: {}", s)),
    }
}

#[derive(Clone, Debug)]
pub enum LogType {
    ProgramConfig,
    BallotBox,
    ConsensusResult,
    MetaMerkleProof,
}
