use crate::errors::AegisError;
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

    /// A reset window, not a sliding window. Both settlement paths share this budget.
    pub fn record_outflow(&mut self, now: i64, amount: u64) -> Result<()> {
        require!(amount > 0, AegisError::ZeroAmountInvalid);
        require!(
            self.epoch_duration > 0,
            AegisError::InvalidTimelockConfiguration
        );
        require!(now >= self.current_epoch_start, AegisError::InvalidClock);
        let elapsed = now
            .checked_sub(self.current_epoch_start)
            .ok_or(AegisError::ArithmeticOverflow)?;
        let reset = elapsed >= self.epoch_duration;
        let previous = if reset {
            0
        } else {
            self.current_epoch_withdrawn
        };
        let next = previous
            .checked_add(amount)
            .ok_or(AegisError::ArithmeticOverflow)?;
        require!(
            next <= self.max_epoch_withdrawal_limit,
            AegisError::RateLimitExceeded
        );
        if reset {
            self.current_epoch_start = now;
        }
        self.current_epoch_withdrawn = next;
        Ok(())
    }

    pub fn assert_solvent(&self, balance: u64) -> Result<()> {
        require!(
            balance >= self.total_active_escrow_amount,
            AegisError::InvariantViolation
        );
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct EscrowAccount {
    /// Immutable vault binding; the vault also identifies the token mint.
    pub vault_config: Pubkey,
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
    // Discriminator, four keys, four 64-bit fields, three flags/bumps, padding.
    pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 1 + 21;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> VaultConfig {
        VaultConfig {
            max_epoch_withdrawal_limit: 100,
            epoch_duration: 10,
            current_epoch_start: 100,
            ..VaultConfig::default()
        }
    }

    #[test]
    fn cumulative_outflow_and_failed_attempt_preserve_budget() {
        let mut v = vault();
        v.record_outflow(100, 60).unwrap();
        assert!(v.record_outflow(109, 41).is_err());
        assert_eq!(v.current_epoch_withdrawn, 60);
        v.record_outflow(109, 40).unwrap();
        assert!(v.record_outflow(109, 1).is_err());
    }

    #[test]
    fn resets_exactly_at_boundary_and_after_long_idle() {
        let mut v = vault();
        v.record_outflow(109, 100).unwrap();
        v.record_outflow(110, 100).unwrap();
        assert_eq!(v.current_epoch_start, 110);
        v.record_outflow(1000, 25).unwrap();
        assert_eq!(v.current_epoch_withdrawn, 25);
        assert_eq!(v.current_epoch_start, 1000);
    }

    #[test]
    fn oversized_failed_rollover_does_not_reset_state() {
        let mut v = vault();
        v.record_outflow(100, 20).unwrap();
        assert!(v.record_outflow(110, 101).is_err());
        assert_eq!(v.current_epoch_start, 100);
        assert_eq!(v.current_epoch_withdrawn, 20);
    }

    #[test]
    fn rejects_backwards_clock_zero_and_overflow() {
        let mut v = vault();
        assert!(v.record_outflow(99, 1).is_err());
        assert!(v.record_outflow(100, 0).is_err());
        v.max_epoch_withdrawal_limit = u64::MAX;
        v.current_epoch_withdrawn = u64::MAX;
        assert!(v.record_outflow(100, 1).is_err());
        v.current_epoch_start = i64::MIN;
        assert!(v.record_outflow(i64::MAX, 1).is_err());
    }

    #[test]
    fn solvency_allows_donations_but_rejects_shortfall() {
        let mut v = vault();
        v.total_active_escrow_amount = 100;
        assert!(v.assert_solvent(99).is_err());
        v.assert_solvent(100).unwrap();
        v.assert_solvent(101).unwrap();
    }

    #[test]
    fn allocated_space_covers_serialized_accounts() {
        let mut data = Vec::new();
        vault().try_serialize(&mut data).unwrap();
        assert!(data.len() <= VaultConfig::LEN);
        data.clear();
        EscrowAccount::default().try_serialize(&mut data).unwrap();
        assert!(data.len() <= EscrowAccount::LEN);
    }
}
