use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use crate::state::{VaultConfig, VAULT_CONFIG_SEED};
use crate::errors::AegisError;

#[derive(Accounts)]
pub struct EmergencyPause<'info> {
    pub authority: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, token_mint.key().as_ref()],
        bump = vault_config.bump,
        has_one = token_mint @ AegisError::InvalidMint
    )]
    pub vault_config: Account<'info, VaultConfig>,
}

pub fn handle_pause(ctx: Context<EmergencyPause>) -> Result<()> {
    let authority_key = ctx.accounts.authority.key();
    let vault_config = &mut ctx.accounts.vault_config;

    require!(
        authority_key == vault_config.admin || authority_key == vault_config.guardian,
        AegisError::UnauthorizedSigner
    );

    vault_config.is_paused = true;
    msg!("EMERGENCY: Aegis Vault has been PAUSED by {}", authority_key);
    Ok(())
}

pub fn handle_unpause(ctx: Context<EmergencyPause>) -> Result<()> {
    let authority_key = ctx.accounts.authority.key();
    let vault_config = &mut ctx.accounts.vault_config;

    // Only Admin can unpause (guardian cannot resume alone)
    require!(
        authority_key == vault_config.admin,
        AegisError::UnauthorizedSigner
    );

    vault_config.is_paused = false;
    msg!("Aegis Vault has been UNPAUSED by Admin {}", authority_key);
    Ok(())
}

#[derive(Accounts)]
pub struct UpdateVaultParams<'info> {
    pub admin: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, token_mint.key().as_ref()],
        bump = vault_config.bump,
        has_one = admin @ AegisError::UnauthorizedSigner,
        has_one = token_mint @ AegisError::InvalidMint
    )]
    pub vault_config: Account<'info, VaultConfig>,
}

pub fn handle_update_params(
    ctx: Context<UpdateVaultParams>,
    new_guardian: Option<Pubkey>,
    new_epoch_limit: Option<u64>,
    new_epoch_duration: Option<i64>,
) -> Result<()> {
    let config = &mut ctx.accounts.vault_config;

    if let Some(guardian) = new_guardian {
        config.guardian = guardian;
        msg!("Vault guardian updated to {}", guardian);
    }

    if let Some(limit) = new_epoch_limit {
        require!(limit > 0, AegisError::ZeroAmountInvalid);
        config.max_epoch_withdrawal_limit = limit;
        msg!("Max epoch withdrawal limit updated to {}", limit);
    }

    if let Some(duration) = new_epoch_duration {
        require!(duration > 0, AegisError::InvalidTimelockConfiguration);
        config.epoch_duration = duration;
        msg!("Epoch duration updated to {}s", duration);
    }

    Ok(())
}
