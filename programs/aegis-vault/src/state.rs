use anchor_lang::prelude::*;

pub const VAULT_CONFIG_SEED: &[u8] = b"aegis_vault";
pub const ESCROW_SEED: &[u8] = b"aegis_escrow";

#[account]
#[derive(Default)]
pub struct VaultConfig {
    /// Protocol Administrator authority
    pub admin: Pubkey,
    /// Guardian / Circuit Breaker authority (can pause in emergency)
    pub guardian: Pubkey,
    /// SPL Token Mint supported by this vault
    pub token_mint: Pubkey,
    /// Associated token account of the vault PDA
    pub vault_token_account: Pubkey,
    /// Circuit breaker status: true if paused
    pub is_paused: bool,
    /// Rate limit: maximum allowed token withdrawal per epoch/window
    pub max_epoch_withdrawal_limit: u64,
    /// Total amount withdrawn within current epoch
    pub current_epoch_withdrawn: u64,
    /// Current epoch timestamp window start (in Unix seconds)
    pub current_epoch_start: i64,
    /// Epoch duration in seconds (e.g. 86400 for 24h)
    pub epoch_duration: i64,
    /// Invariant tracking: total pending funds locked in active escrows
    pub total_active_escrow_amount: u64,
    /// Cumulative volume routed through protocol
    pub total_historical_volume: u64,
    /// Nonce/bump for vault config PDA
    pub bump: u8,
}

impl VaultConfig {
    // 8 (discriminator) + 32*4 + 1 + 8*6 + 1 = 8 + 128 + 1 + 48 + 1 = 186 bytes -> pad to 200
    pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 1 + 16;
}

#[account]
#[derive(Default)]
pub struct EscrowAccount {
    /// Funder who deposited the tokens
    pub sender: Pubkey,
    /// Intended beneficiary of the escrow
    pub recipient: Pubkey,
    /// Optional designated arbiter/mediator for conditional disputes
    pub arbiter: Pubkey,
    /// Unique sequential escrow ID per sender
    pub escrow_id: u64,
    /// Amount of tokens deposited into escrow
    pub amount: u64,
    /// Earliest timestamp (Unix seconds) after which funds can be released / claimed
    pub release_timestamp: i64,
    /// Expiration timestamp after which sender can reclaim if unreleased
    pub expiry_timestamp: i64,
    /// Status flags
    pub is_released: bool,
    pub is_cancelled: bool,
    /// Bump seed
    pub bump: u8,
}

impl EscrowAccount {
    // 8 (discriminator) + 32*3 + 8*4 + 1 + 1 + 1 = 8 + 96 + 32 + 3 = 139 bytes -> pad to 160
    pub const LEN: usize = 8 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 1 + 21;
}
