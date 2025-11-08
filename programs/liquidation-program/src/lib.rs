use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("Liqd8UyMVwSYFsETMWhJWEQ7DnDjEYwETaAh6hFkwxv");

#[program]
pub mod liquidation_program {
    use super::*;

    /// Initialize a user's position account.
    pub fn initialize_position(ctx: Context<InitializePosition>) -> Result<()> {
        let position = &mut ctx.accounts.position;
        position.owner = ctx.accounts.user.key();
        position.bump = ctx.bumps.position;
        position.collateral = 0;
        position.debt = 0;
        Ok(())
    }

    /// Deposit collateral into the position.
    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        // Transfer tokens from user to vault
        transfer_tokens(
            &ctx.accounts.token_program,
            &ctx.accounts.user_token_account.to_account_info(),
            &ctx.accounts.vault.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            amount,
        )?;

        ctx.accounts.position.collateral += amount;
        Ok(())
    }

    /// Liquidate an undercollateralized position.
    pub fn liquidate(ctx: Context<LiquidatePosition>, repay_amount: u64) -> Result<()> {

        let position = &mut ctx.accounts.position;

        // Check if liquidation is allowed
        require!(
            position.collateral < position.debt,
            LiquidationError::PositionHealthy
        );

        // Transfer repayment from liquidator to vault
        transfer_tokens(
            &ctx.accounts.token_program,
            &ctx.accounts.liquidator_token_account.to_account_info(),
            &ctx.accounts.vault.to_account_info(),
            &ctx.accounts.liquidator.to_account_info(),
            repay_amount,
        )?;

        // Give liquidator a reward
        let reward = repay_amount / 10; // 10% reward
        transfer_tokens(
            &ctx.accounts.token_program,
            &ctx.accounts.vault.to_account_info(),
            &ctx.accounts.liquidator_token_account.to_account_info(),
            &ctx.accounts.vault_authority.to_account_info(),
            reward,
        )?;

        // Adjust position
        position.debt = position.debt.saturating_sub(repay_amount);
        position.collateral = position.collateral.saturating_sub(reward);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePosition<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + 8 + 8 + 32 + 1,
        seeds = [b"position", user.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub position: Account<'info, Position>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct LiquidatePosition<'info> {
    #[account(mut)]
    pub position: Account<'info, Position>,
    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub liquidator_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub insurance_fund_vault: Account<'info, TokenAccount>,
    pub vault_authority: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub oracle: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub liquidator: Signer<'info>,
}


/// state account representing a user’s margin position.
#[account]
pub struct Position {
    pub owner: Pubkey,
    pub bump: u8,
    pub collateral: u64,
    pub debt: u64,
}

#[error_code]
pub enum LiquidationError {
    #[msg("Position is healthy and cannot be liquidated.")]
    PositionHealthy,
}

/// Utility for safe token transfers.
fn transfer_tokens<'info>(
    token_program: &Program<'info, Token>,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: from.clone(),
        to: to.clone(),
        authority: authority.clone(),
    };
    let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}
