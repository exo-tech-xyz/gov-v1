use solana_program::pubkey::Pubkey;

/// Marinade's withdraw authority for stake accounts
pub const MARINADE_WITHDRAW_AUTHORITY: Pubkey =
    Pubkey::from_str_const("9eG63CdHjsfhHmobHgLtESGC8GabbmRcaSpHAZrtmhco");

/// Marinade's operations voting wallet
pub const MARINADE_OPS_VOTING_WALLET: Pubkey =
    Pubkey::from_str_const("opLSF7LdfyWNBby5o6FT8UFsr2A4UGKteECgtLSYrSm");
