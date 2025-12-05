use anchor_lang::prelude::*;

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