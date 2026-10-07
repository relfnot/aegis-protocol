use crate::errors::AegisError;
use crate::state::{EscrowAccount, VaultConfig, ESCROW_SEED, VAULT_CONFIG_SEED};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct ReleaseEscrow<'info> {
    /// Signer executing the release (can be sender, recipient, or arbiter)
    pub caller: Signer<'info>,

    /// Account receiving the released funds
    #[account(
        mut,
        constraint = recipient_token_account.owner == escrow_account.recipient @ AegisError::UnauthorizedSigner,
        constraint = recipient_token_account.mint == token_mint.key() @ AegisError::InvalidMint
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, vault_config.admin.as_ref(), token_mint.key().as_ref()],
        bump = vault_config.bump,
        has_one = token_mint @ AegisError::InvalidMint,
        has_one = vault_token_account @ AegisError::InvalidTokenAccount
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(mut, token::mint = token_mint, token::authority = vault_config)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [
            ESCROW_SEED,
            escrow_account.vault_config.as_ref(),
            escrow_account.sender.as_ref(),
            &escrow_account.escrow_id.to_le_bytes()
        ],
        bump = escrow_account.bump,
        has_one = vault_config @ AegisError::InvalidVault
    )]
    pub escrow_account: Account<'info, EscrowAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn process_release_escrow(ctx: Context<ReleaseEscrow>) -> Result<()> {
    let clock = Clock::get()?;
    let caller_key = ctx.accounts.caller.key();
    let escrow = &ctx.accounts.escrow_account;
    let vault_config = &mut ctx.accounts.vault_config;

    // 1. Circuit breaker check
    require!(!vault_config.is_paused, AegisError::VaultPaused);

    vault_config.assert_solvent(ctx.accounts.vault_token_account.amount)?;

    // 2. Settlement state check
    require!(
        !escrow.is_released && !escrow.is_cancelled,
        AegisError::EscrowAlreadySettled
    );

    // 3. Authorization and Timelock evaluation
    let is_sender = caller_key == escrow.sender;
    let is_recipient = caller_key == escrow.recipient;
    let is_arbiter = escrow.arbiter != Pubkey::default() && caller_key == escrow.arbiter;

    require!(
        is_sender || is_recipient || is_arbiter,
        AegisError::UnauthorizedSigner
    );

    // If caller is recipient alone, timelock MUST have elapsed
    if is_recipient && !is_sender && !is_arbiter {
        require!(
            clock.unix_timestamp >= escrow.release_timestamp,
            AegisError::TimelockNotElapsed
        );
    }

    require!(
        clock.unix_timestamp < escrow.expiry_timestamp,
        AegisError::EscrowExpired
    );
    let release_amount = escrow.amount;

    // Every release and refund consumes the same outflow budget.
    vault_config.record_outflow(clock.unix_timestamp, release_amount)?;

    // 5. Update Invariant: Deduct active escrow volume
    vault_config.total_active_escrow_amount = vault_config
        .total_active_escrow_amount
        .checked_sub(release_amount)
        .ok_or(AegisError::ArithmeticOverflow)?;

    // 6. Transfer tokens from vault PDA to recipient
    let mint_key = ctx.accounts.token_mint.key();
    let seeds = &[
        VAULT_CONFIG_SEED,
        vault_config.admin.as_ref(),
        mint_key.as_ref(),
        &[vault_config.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: vault_config.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer,
    );
    token::transfer(cpi_ctx, release_amount)?;
    ctx.accounts.vault_token_account.reload()?;
    vault_config.assert_solvent(ctx.accounts.vault_token_account.amount)?;

    // 7. Update Escrow status
    let escrow_mut = &mut ctx.accounts.escrow_account;
    escrow_mut.is_released = true;

    msg!(
        "Aegis Escrow #{} successfully released: {} tokens to recipient",
        escrow_mut.escrow_id,
        release_amount
    );
    Ok(())
}
