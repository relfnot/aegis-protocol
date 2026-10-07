use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{VaultConfig, VAULT_CONFIG_SEED};
use crate::errors::AegisError;

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: Guardian key specified for emergency circuit breaker
    pub guardian: UncheckedAccount<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = VaultConfig::LEN,
        seeds = [VAULT_CONFIG_SEED, token_mint.key().as_ref()],
        bump
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        init,
        payer = admin,
        token::mint = token_mint,
        token::authority = vault_config,
        seeds = [b"aegis_token_vault", token_mint.key().as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializeVault>,
    max_epoch_withdrawal_limit: u64,
    epoch_duration_seconds: i64,
) -> Result<()> {
    require!(epoch_duration_seconds > 0, AegisError::InvalidTimelockConfiguration);
    require!(max_epoch_withdrawal_limit > 0, AegisError::ZeroAmountInvalid);

    let clock = Clock::get()?;
    let config = &mut ctx.accounts.vault_config;

    config.admin = ctx.accounts.admin.key();
    config.guardian = ctx.accounts.guardian.key();
    config.token_mint = ctx.accounts.token_mint.key();
    config.vault_token_account = ctx.accounts.vault_token_account.key();
    config.is_paused = false;
    config.max_epoch_withdrawal_limit = max_epoch_withdrawal_limit;
    config.current_epoch_withdrawn = 0;
    config.current_epoch_start = clock.unix_timestamp;
    config.epoch_duration = epoch_duration_seconds;
    config.total_active_escrow_amount = 0;
    config.total_historical_volume = 0;
    config.bump = ctx.bumps.vault_config;

    msg!("Aegis Vault initialized for mint: {}", ctx.accounts.token_mint.key());
    msg!("Circuit breaker max epoch limit: {}", max_epoch_withdrawal_limit);
    Ok(())
}
