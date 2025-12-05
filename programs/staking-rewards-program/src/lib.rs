use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer, Mint, TokenAccount};

declare_id!("BsB6SGtbubwYkUfRXQ2kd8WyQ2AqN5AVUG8LXg9gck6M");

const POOL_SEED: &[u8] = b"pool";
const VAULT_STAKE_SEED: &[u8] = b"stake_vault";
const VAULT_REWARD_SEED: &[u8] = b"reward_vault";
const USER_SEED: &[u8] = b"user_stake";

#[program]
pub mod staking_rewards_program {
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        reward_rate_per_second: u64,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;

        pool.admin = ctx.accounts.admin.key();
        pool.stake_mint = ctx.accounts.stake_mint.key();
        pool.reward_mint = ctx.accounts.reward_mint.key();
        pool.reward_rate_per_second = reward_rate_per_second;
        pool.total_staked = 0;
        pool.bump = ctx.bumps.pool;

        Ok(())
    }

    pub fn deposit_rewards(ctx: Context<DepositRewards>, amount: u64) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.pool.admin,
            ctx.accounts.admin.key(),
            StakingError::Unauthorized
        );

        let cpi_accounts = Transfer {
            from: ctx.accounts.admin_reward_ata.to_account_info(),
            to: ctx.accounts.reward_vault.to_account_info(),
            authority: ctx.accounts.admin.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );

        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakingError::ZeroAmount);

        let user = &mut ctx.accounts.user_stake;
        let pool = &mut ctx.accounts.pool;

        // Initialize user stake if needed
        if user.owner == Pubkey::default() {
            user.owner = ctx.accounts.user.key();
            user.amount_staked = 0;
            user.pending_rewards = 0;
            user.last_update = Clock::get()?.unix_timestamp;
            user.bump = ctx.bumps.user_stake;
        }

        update_rewards(user, pool)?;

        // Transfer stake → vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_stake_ata.to_account_info(),
            to: ctx.accounts.stake_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );

        token::transfer(cpi_ctx, amount)?;

        user.amount_staked += amount as u128;
        pool.total_staked += amount as u128;
        user.last_update = Clock::get()?.unix_timestamp;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakingError::ZeroAmount);

        let user = &mut ctx.accounts.user_stake;
        
        require!(
            user.amount_staked >= amount as u128,
            StakingError::InsufficientFunds
        );

        // Get pool data BEFORE mutable operations
        let _pool_key = ctx.accounts.pool.key();
        let stake_bump = ctx.accounts.pool.bump;
        let stake_mint = ctx.accounts.pool.stake_mint;
        let reward_mint = ctx.accounts.pool.reward_mint;

        update_rewards(user, &ctx.accounts.pool)?;

        let seeds = &[
            POOL_SEED,
            stake_mint.as_ref(),
            reward_mint.as_ref(),
            &[stake_bump],
        ];

        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.stake_vault.to_account_info(),
            to: ctx.accounts.user_stake_ata.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );

        token::transfer(cpi_ctx, amount)?;

        // Now update the pool and user
        let pool = &mut ctx.accounts.pool;
        user.amount_staked -= amount as u128;
        pool.total_staked -= amount as u128;
        user.last_update = Clock::get()?.unix_timestamp;

        Ok(())
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let user = &mut ctx.accounts.user_stake;

        // Get pool data BEFORE mutable operations
        let pool_bump = ctx.accounts.pool.bump;
        let stake_mint = ctx.accounts.pool.stake_mint;
        let reward_mint = ctx.accounts.pool.reward_mint;

        update_rewards(user, &ctx.accounts.pool)?;

        let amount = user.pending_rewards as u64;
        require!(amount > 0, StakingError::NoRewardsAccrued);

        user.pending_rewards = 0;

        let seeds = &[
            POOL_SEED,
            stake_mint.as_ref(),
            reward_mint.as_ref(),
            &[pool_bump],
        ];

        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.reward_vault.to_account_info(),
            to: ctx.accounts.user_reward_ata.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );

        token::transfer(cpi_ctx, amount)?;

        user.last_update = Clock::get()?.unix_timestamp;

        Ok(())
    }
}

//    STATE STRUCTS


#[account]
pub struct Pool {
    pub admin: Pubkey,
    pub stake_mint: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_rate_per_second: u64,
    pub total_staked: u128,
    pub bump: u8,
}

impl Pool {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 8 + 16 + 1;
}

#[account]
pub struct UserStake {
    pub owner: Pubkey,
    pub amount_staked: u128,
    pub pending_rewards: u128,
    pub last_update: i64,
    pub bump: u8,
}

impl UserStake {
    pub const LEN: usize = 8 + 32 + 16 + 16 + 8 + 1;
}

//    ACCOUNT CONTEXTS


#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub stake_mint: Account<'info, Mint>,
    pub reward_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = Pool::LEN,
        seeds = [POOL_SEED, stake_mint.key().as_ref(), reward_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = admin,
        seeds = [VAULT_STAKE_SEED, pool.key().as_ref()],
        bump,
        token::mint = stake_mint,
        token::authority = pool,
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = admin,
        seeds = [VAULT_REWARD_SEED, pool.key().as_ref()],
        bump,
        token::mint = reward_mint,
        token::authority = pool,
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositRewards<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub admin_reward_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub reward_vault: Account<'info, TokenAccount>,

    pub pool: Account<'info, Pool>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub pool: Account<'info, Pool>,

    #[account(
        init_if_needed,
        payer = user,
        space = UserStake::LEN,
        seeds = [USER_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(
        mut,
        token::mint = pool.stake_mint,
        token::authority = user
    )]
    pub user_stake_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub stake_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [USER_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump = user_stake.bump
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(
        mut,
        token::mint = pool.stake_mint,
        token::authority = user
    )]
    pub user_stake_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_STAKE_SEED, pool.key().as_ref()],
        bump
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [USER_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump = user_stake.bump
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(
        mut,
        token::mint = pool.reward_mint,
        token::authority = user
    )]
    pub user_reward_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_REWARD_SEED, pool.key().as_ref()],
        bump
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

//    REWARD CALC


fn update_rewards(user: &mut UserStake, pool: &Pool) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    if user.amount_staked == 0 || pool.total_staked == 0 {
        user.last_update = now;
        return Ok(());
    }

    let elapsed = now - user.last_update;

    if elapsed <= 0 {
        return Ok(());
    }

    let reward =
        (user.amount_staked * pool.reward_rate_per_second as u128 * elapsed as u128)
            / pool.total_staked;

    user.pending_rewards = user
        .pending_rewards
        .checked_add(reward)
        .ok_or(StakingError::Overflow)?;

    user.last_update = now;

    Ok(())
}

/* -----------------------------
   ERRORS
-------------------------------- */

#[error_code]
pub enum StakingError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Not enough tokens")]
    InsufficientFunds,
    #[msg("Zero amount")]
    ZeroAmount,
    #[msg("No rewards available")]
    NoRewardsAccrued,
    #[msg("Overflow")]
    Overflow,
}