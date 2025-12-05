# Staking Rewards Program

A complete Solana staking and reward distribution smart contract built with Anchor framework. Users can stake tokens, earn rewards based on time staked, and claim/unstake their tokens.
# solana path
export PATH="/home/codespace/.local/share/solana/install/active_release/bin:$PATH"
## Program Overview

This program implements a time-based staking rewards system where users earn rewards proportional to their stake amount and time staked. The reward distribution uses a global reward rate that allocates rewards fairly among all stakers.

## PDA Derivations

### 1. Pool Account
- **Seed**: `"pool"`
- **Additional Seeds**: `stake_mint`, `reward_mint`
- **Purpose**: Stores global pool configuration and state
- **Derivation**:
  ```rust
  Pubkey::find_program_address(
      &[b"pool", stake_mint.as_ref(), reward_mint.as_ref()],
      program_id
  )
  ```

### 2. Stake Vault
- **Seed**: `"stake_vault"`
- **Additional Seeds**: `pool_key`
- **Purpose**: Holds all staked tokens from users
- **Derivation**:
  ```rust
  Pubkey::find_program_address(
      &[b"stake_vault", pool.key().as_ref()],
      program_id
  )
  ```

### 3. Reward Vault
- **Seed**: `"reward_vault"`
- **Additional Seeds**: `pool_key`
- **Purpose**: Holds reward tokens for distribution
- **Derivation**:
  ```rust
  Pubkey::find_program_address(
      &[b"reward_vault", pool.key().as_ref()],
      program_id
  )
  ```

### 4. User Stake Account
- **Seed**: `"user_stake"`
- **Additional Seeds**: `pool_key`, `user_key`
- **Purpose**: Tracks individual user's staking position and rewards
- **Derivation**:
  ```rust
  Pubkey::find_program_address(
      &[b"user_stake", pool.key().as_ref(), user.key().as_ref()],
      program_id
  )
  ```

## Reward Math Explanation

### Reward Calculation Formula

```
user_reward = (user_stake * reward_rate * time_elapsed) / total_staked
```

Where:
- `user_stake`: Amount of tokens staked by the user
- `reward_rate`: Global reward rate per second (set during pool initialization)
- `time_elapsed`: Time in seconds since last reward update
- `total_staked`: Total tokens staked in the pool

### Key Features

1. **Proportional Distribution**: Rewards are distributed proportionally to each user's stake relative to the total pool
2. **Time-Based**: Rewards accumulate in real-time based on seconds staked
3. **Auto-Compounding**: Rewards are calculated and accumulated automatically on every stake/unstake/claim operation
4. **Fair Allocation**: No advantage for early or late stakers - rewards are purely based on stake-time product

### Example Calculation

If:
- Total staked: 1000 tokens
- Your stake: 100 tokens (10% of pool)
- Reward rate: 1000 tokens per second
- Time elapsed: 3600 seconds (1 hour)

Your reward = `(100 * 1000 * 3600) / 1000 = 360,000 tokens`

## How to Run

### Prerequisites

- Node.js (v16 or higher)
- Rust (v1.70.0 or higher)
- Solana CLI (v1.18.0 or higher)
- Anchor CLI (v0.30.1 or higher)

### Installation

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd staking-rewards-program
   ```

2. **Install dependencies**
   ```bash
   npm install
   # or
   yarn install
   ```

3. **Build the program**
   ```bash
   anchor build
   ```

4. **Run tests**
   ```bash
   anchor test
   ```

### Local Development

1. **Start local validator**
   ```bash
   solana-test-validator
   ```

2. **Deploy to localnet**
   ```bash
   anchor deploy
   ```

3. **Run tests against localnet**
   ```bash
   anchor test
   ```

## Command Examples

### Initialize Pool

```bash
# Using Anchor CLI
anchor run initialize-pool

# Using Solana CLI (after deployment)
solana program invoke <PROGRAM_ID> \
  --accounts <ACCOUNTS> \
  --data <INSTRUCTION_DATA>
```

### Deposit Rewards

```bash
# Admin deposits rewards into the pool
anchor run deposit-rewards --amount 5000000
```

### Stake Tokens

```bash
# User stakes tokens
anchor run stake --amount 1000000
```

### Claim Rewards

```bash
# User claims accumulated rewards
anchor run claim-rewards
```

### Unstake Tokens

```bash
# User unstakes tokens (partial or full)
anchor run unstake --amount 500000
```

## Program Instructions

### 1. `initialize_pool`
Initializes a new staking pool with specified reward rate.

**Parameters:**
- `reward_rate_per_second`: u64 - Reward tokens distributed per second across all stakers

**Accounts:**
- `admin`: Signer - Pool administrator
- `stake_mint`: Mint - Token to be staked
- `reward_mint`: Mint - Reward token
- `pool`: PDA - Pool state account
- `stake_vault`: PDA - Stake token vault
- `reward_vault`: PDA - Reward token vault

### 2. `deposit_rewards`
Allows admin to deposit reward tokens into the pool.

**Parameters:**
- `amount`: u64 - Amount of reward tokens to deposit

**Accounts:**
- `admin`: Signer - Pool administrator
- `admin_reward_ata`: TokenAccount - Admin's reward token account
- `reward_vault`: TokenAccount - Reward vault
- `pool`: Account - Pool state

### 3. `stake`
Allows users to stake tokens into the pool.

**Parameters:**
- `amount`: u64 - Amount of tokens to stake

**Accounts:**
- `user`: Signer - User staking tokens
- `pool`: Account - Pool state
- `user_stake`: PDA - User's staking position
- `user_stake_ata`: TokenAccount - User's stake token account
- `stake_vault`: TokenAccount - Stake vault

### 4. `unstake`
Allows users to unstake tokens from the pool.

**Parameters:**
- `amount`: u64 - Amount of tokens to unstake

**Accounts:**
- `user`: Signer - User unstaking tokens
- `pool`: Account - Pool state
- `user_stake`: PDA - User's staking position
- `user_stake_ata`: TokenAccount - User's stake token account
- `stake_vault`: TokenAccount - Stake vault

### 5. `claim_rewards`
Allows users to claim accumulated rewards.

**Parameters:** None

**Accounts:**
- `user`: Signer - User claiming rewards
- `pool`: Account - Pool state
- `user_stake`: PDA - User's staking position
- `user_reward_ata`: TokenAccount - User's reward token account
- `reward_vault`: TokenAccount - Reward vault

## Testing

Run the complete test suite:

```bash
anchor test
```

The test suite covers:
- Pool initialization
- Reward deposits
- Token staking
- Reward claiming
- Token unstaking
- Multiple user scenarios

## Program ID

```
BsB6SGtbubwYkUfRXQ2kd8WyQ2AqN5AVUG8LXg9gck6M
```

## Security Features

- PDA-based authority for all vaults
- Proper access controls (only admin can deposit rewards)
- Overflow protection in reward calculations
- Input validation (non-zero amounts, sufficient balances)
- Reentrancy protection through state updates

## Error Codes

- `Unauthorized`: Invalid administrator
- `InsufficientFunds`: Not enough tokens for operation
- `ZeroAmount`: Operation with zero amount
- `NoRewardsAccrued`: No rewards available to claim
- `Overflow`: Arithmetic overflow in calculations

## License

MIT License - see LICENSE file for details
