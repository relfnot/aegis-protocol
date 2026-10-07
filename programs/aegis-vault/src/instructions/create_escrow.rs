use crate::errors::AegisError;
use crate::state::{EscrowAccount, VaultConfig, ESCROW_SEED, VAULT_CONFIG_SEED};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
#[instruction(escrow_id: u64)]
pub struct CreateEscrow<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    /// CHECK: The designated recipient/beneficiary account
    pub recipient: UncheckedAccount<'info>,

    /// CHECK: Optional arbiter account (can be SystemProgram ID if unused)
    pub arbiter: UncheckedAccount<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED, vault_config.admin.as_ref(), token_mint.key().as_ref()],
        bump = vault_config.bump,
        has_one = token_mint @ AegisError::InvalidMint,
        has_one = vault_token_account @ AegisError::InvalidTokenAccount
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        mut,
        constraint = sender_token_account.owner == sender.key() @ AegisError::UnauthorizedSigner,
        constraint = sender_token_account.mint == token_mint.key() @ AegisError::InvalidMint
    )]
    pub sender_token_account: Account<'info, TokenAccount>,

    #[account(mut, token::mint = token_mint, token::authority = vault_config)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = sender,
        space = EscrowAccount::LEN,
        seeds = [
            ESCROW_SEED,
            vault_config.key().as_ref(),
            sender.key().as_ref(),
            &escrow_id.to_le_bytes()
        ],
        bump
    )]
    pub escrow_account: Account<'info, EscrowAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn process_create_escrow(
    ctx: Context<CreateEscrow>,
    escrow_id: u64,
    amount: u64,
    release_timestamp: i64,
    expiry_timestamp: i64,
) -> Result<()> {
    let vault_key = ctx.accounts.vault_config.key();
    let vault_config = &mut ctx.accounts.vault_config;
    require!(!vault_config.is_paused, AegisError::VaultPaused);
    require!(amount > 0, AegisError::ZeroAmountInvalid);
    require!(
        amount <= vault_config.max_epoch_withdrawal_limit,
        AegisError::EscrowExceedsLimit
    );
    vault_config.assert_solvent(ctx.accounts.vault_token_account.amount)?;

    let clock = Clock::get()?;
    require!(
        release_timestamp < expiry_timestamp,
        AegisError::InvalidTimelockConfiguration
    );
    require!(
        expiry_timestamp > clock.unix_timestamp,
        AegisError::EscrowExpired
    );

    // 1. Transfer tokens from sender into the vault token account
    let cpi_accounts = Transfer {
        from: ctx.accounts.sender_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.sender.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // 2. Update protocol invariants
    vault_config.total_active_escrow_amount = vault_config
        .total_active_escrow_amount
        .checked_add(amount)
        .ok_or(AegisError::ArithmeticOverflow)?;

    vault_config.total_historical_volume = vault_config
        .total_historical_volume
        .checked_add(amount)
        .ok_or(AegisError::ArithmeticOverflow)?;

    ctx.accounts.vault_token_account.reload()?;
    vault_config.assert_solvent(ctx.accounts.vault_token_account.amount)?;

    // 3. Populate Escrow State
    let escrow = &mut ctx.accounts.escrow_account;
    escrow.vault_config = vault_key;
    escrow.sender = ctx.accounts.sender.key();
    escrow.recipient = ctx.accounts.recipient.key();
    escrow.arbiter = ctx.accounts.arbiter.key();
    escrow.escrow_id = escrow_id;
    escrow.amount = amount;
    escrow.release_timestamp = release_timestamp;
    escrow.expiry_timestamp = expiry_timestamp;
    escrow.is_released = false;
    escrow.is_cancelled = false;
    escrow.bump = ctx.bumps.escrow_account;

    msg!(
        "Aegis Escrow #{} created: {} tokens locked",
        escrow_id,
        amount
    );
    Ok(())
}
