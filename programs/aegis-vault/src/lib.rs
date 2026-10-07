use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWxTWqkZqXmJbYyHhP7FZxQzLhQZ");

#[program]
pub mod aegis_vault {
    use super::*;

    /// Initialize the Aegis security vault and configure rate limits and guardian authority
    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        max_epoch_withdrawal_limit: u64,
        epoch_duration_seconds: i64,
    ) -> Result<()> {
        instructions::initialize_vault::process_initialize_vault(
            ctx,
            max_epoch_withdrawal_limit,
            epoch_duration_seconds,
        )
    }

    /// Deposit funds into a new conditional escrow with timelock and dispute resolution parameters
    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        escrow_id: u64,
        amount: u64,
        release_timestamp: i64,
        expiry_timestamp: i64,
    ) -> Result<()> {
        instructions::create_escrow::process_create_escrow(
            ctx,
            escrow_id,
            amount,
            release_timestamp,
            expiry_timestamp,
        )
    }

    /// Release funds from escrow to recipient subject to rate limits and timelock checks
    pub fn release_escrow(ctx: Context<ReleaseEscrow>) -> Result<()> {
        instructions::release_escrow::process_release_escrow(ctx)
    }

    /// Cancel expired escrow and return locked tokens to sender
    pub fn cancel_escrow(ctx: Context<CancelEscrow>) -> Result<()> {
        instructions::cancel_escrow::process_cancel_escrow(ctx)
    }

    /// Emergency Circuit Breaker: Pause deposits and settlements (Admin or Guardian)
    pub fn emergency_pause(ctx: Context<EmergencyPause>) -> Result<()> {
        instructions::emergency_actions::handle_pause(ctx)
    }

    /// Resume normal vault operations after verification (Admin only)
    pub fn emergency_unpause(ctx: Context<EmergencyPause>) -> Result<()> {
        instructions::emergency_actions::handle_unpause(ctx)
    }

    /// Update security parameters: guardian key, epoch limit, epoch duration
    pub fn update_vault_params(
        ctx: Context<UpdateVaultParams>,
        new_guardian: Option<Pubkey>,
        new_epoch_limit: Option<u64>,
        new_epoch_duration: Option<i64>,
    ) -> Result<()> {
        instructions::emergency_actions::handle_update_params(
            ctx,
            new_guardian,
            new_epoch_limit,
            new_epoch_duration,
        )
    }
}
