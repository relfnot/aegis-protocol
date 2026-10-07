use anchor_lang::prelude::*;

#[error_code]
pub enum AegisError {
    #[msg("Operation rejected: Aegis Vault circuit breaker is currently engaged (paused).")]
    VaultPaused,

    #[msg("Unauthorized signer: Sender does not have permission for this privileged operation.")]
    UnauthorizedSigner,

    #[msg("Rate limit exceeded: Cumulative epoch withdrawal limit reached. Circuit breaker active.")]
    RateLimitExceeded,

    #[msg("Invalid timelock: Release timestamp cannot be in the past or after the expiry timestamp.")]
    InvalidTimelockConfiguration,

    #[msg("Timelock still active: Escrow release condition is not yet satisfied.")]
    TimelockNotElapsed,

    #[msg("Escrow expired: Action cannot be performed because deadline has passed.")]
    EscrowExpired,

    #[msg("Escrow not yet expired: Reclaim/Cancel can only occur after the designated expiry timestamp.")]
    EscrowNotExpiredYet,

    #[msg("Escrow already settled: This escrow has already been released or cancelled.")]
    EscrowAlreadySettled,

    #[msg("Zero amount invalid: Deposit or escrow amount must be greater than zero.")]
    ZeroAmountInvalid,

    #[msg("Arithmetic overflow/underflow detected in balance calculation.")]
    ArithmeticOverflow,

    #[msg("Invalid Token Mint: The provided token mint does not match the vault configuration.")]
    InvalidMint,

    #[msg("Invalid Token Account: Provided token account does not match expected PDA or mint.")]
    InvalidTokenAccount,

    #[msg("Protocol invariant violation: Inactive or unbalanced state detected.")]
    InvariantViolation,
}
