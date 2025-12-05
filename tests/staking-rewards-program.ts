import * as anchor from "@coral-xyz/anchor";
import {
  PublicKey,
  SystemProgram,
  Keypair,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddress,
  createAssociatedTokenAccount,
  createMint,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

import { StakingRewardsProgram } from "../target/types/staking_rewards_program";

describe("staking-rewards-program", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const wallet = provider.wallet as anchor.Wallet;
  const connection = provider.connection;

  const program = anchor.workspace
    .StakingRewardsProgram as anchor.Program<StakingRewardsProgram>;

  let stakeMint: PublicKey;
  let rewardMint: PublicKey;
  let admin = wallet.publicKey;

  let poolPda: PublicKey;
  let stakeVault: PublicKey;
  let rewardVault: PublicKey;

  //  derive PDAs 
  const getPoolPda = () => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), stakeMint.toBuffer(), rewardMint.toBuffer()],
      program.programId
    );
  };

  const getStakeVaultPda = (pool: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("stake_vault"), pool.toBuffer()],
      program.programId
    );
  };

  const getRewardVaultPda = (pool: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("reward_vault"), pool.toBuffer()],
      program.programId
    );
  };

  const getUserStakePda = (pool: PublicKey, user: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("user_stake"), pool.toBuffer(), user.toBuffer()],
      program.programId
    );
  };

  before(async () => {
 
    stakeMint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      6
    );

    rewardMint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      6
    );

    console.log("Stake Mint:", stakeMint.toBase58());
    console.log("Reward Mint:", rewardMint.toBase58());

    // Derive PDAs
    [poolPda] = getPoolPda();
    [stakeVault] = getStakeVaultPda(poolPda);
    [rewardVault] = getRewardVaultPda(poolPda);

    console.log("Pool PDA:", poolPda.toBase58());
    console.log("Stake Vault:", stakeVault.toBase58());
    console.log("Reward Vault:", rewardVault.toBase58());
  });

  it("Initialize Pool", async () => {
    const rewardRate = new anchor.BN(1000);

    const tx = await program.methods
      .initializePool(rewardRate)
      .accounts({
        admin,
        stakeMint,
        rewardMint,
        pool: poolPda,
        stakeVault,
        rewardVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Initialize Pool Tx:", tx);

    const poolAccount = await program.account.pool.fetch(poolPda);
    console.log("Pool Admin:", poolAccount.admin.toBase58());
    console.log("Reward Rate:", poolAccount.rewardRatePerSecond.toString());
    console.log("Total Staked:", poolAccount.totalStaked.toString());
  });

  it("Create ATAs and mint tokens", async () => {
    const adminRewardAta = await getAssociatedTokenAddress(rewardMint, admin);
    try {
      await getAccount(connection, adminRewardAta);
    } catch (_) {
      await createAssociatedTokenAccount(
        connection,
        wallet.payer,
        rewardMint,
        admin
      );
    }

    await mintTo(
      connection,
      wallet.payer,
      rewardMint,
      adminRewardAta,
      wallet.payer,
      10_000_000
    );

    const adminStakeAta = await getAssociatedTokenAddress(stakeMint, admin);
    try {
      await getAccount(connection, adminStakeAta);
    } catch (_) {
      await createAssociatedTokenAccount(
        connection,
        wallet.payer,
        stakeMint,
        admin
      );
    }

    await mintTo(
      connection,
      wallet.payer,
      stakeMint,
      adminStakeAta,
      wallet.payer,
      10_000_000
    );

    console.log("Minted tokens to admin ATAs");
  });

  it("Deposit Rewards", async () => {
    const adminRewardAta = await getAssociatedTokenAddress(rewardMint, admin);

    const tx = await program.methods
      .depositRewards(new anchor.BN(5_000_000))
      .accounts({
        admin,
        adminRewardAta,
        rewardVault,
        pool: poolPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Deposit Rewards Tx:", tx);

    const rewardVaultAccount = await getAccount(connection, rewardVault);
    console.log("Reward Vault Balance:", rewardVaultAccount.amount.toString());
  });

  it("Stake Tokens", async () => {
    const user = admin;

    // User stake ATA
    const userStakeAta = await getAssociatedTokenAddress(stakeMint, user);

    // User stake PDA
    const [userStakePda] = getUserStakePda(poolPda, user);

    const stakeAmount = new anchor.BN(1_000_000); // 1 token with 6 decimals

    const tx = await program.methods
      .stake(stakeAmount)
      .accounts({
        user,
        pool: poolPda,
        userStake: userStakePda,
        userStakeAta,
        stakeVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Stake Tx:", tx);

    const stakeVaultAccount = await getAccount(connection, stakeVault);
    console.log("Stake Vault Balance:", stakeVaultAccount.amount.toString());

    const userStakeAccount = await program.account.userStake.fetch(userStakePda);
    console.log("User Staked Amount:", userStakeAccount.amountStaked.toString());
    console.log("User Pending Rewards:", userStakeAccount.pendingRewards.toString());

    const poolAccount = await program.account.pool.fetch(poolPda);
    console.log("Pool Total Staked:", poolAccount.totalStaked.toString());
  });

  it("Claim Rewards", async () => {
    const user = admin;
    
    await new Promise(resolve => setTimeout(resolve, 2000));

    const [userStakePda] = getUserStakePda(poolPda, user);
    const userRewardAta = await getAssociatedTokenAddress(rewardMint, user);

    try {
      await getAccount(connection, userRewardAta);
    } catch (_) {
      await createAssociatedTokenAccount(
        connection,
        wallet.payer,
        rewardMint,
        user
      );
    }

    const tx = await program.methods
      .claimRewards()
      .accounts({
        user,
        pool: poolPda,
        userStake: userStakePda,
        userRewardAta,
        rewardVault,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Claim Rewards Tx:", tx);

    const userRewardAccount = await getAccount(connection, userRewardAta);
    console.log("User Reward Balance:", userRewardAccount.amount.toString());

    const userStakeAccount = await program.account.userStake.fetch(userStakePda);
    console.log("User Pending Rewards After Claim:", userStakeAccount.pendingRewards.toString());
  });

  it("Unstake Tokens", async () => {
    const user = admin;
    const [userStakePda] = getUserStakePda(poolPda, user);
    const userStakeAta = await getAssociatedTokenAddress(stakeMint, user);

    const unstakeAmount = new anchor.BN(500_000); 

    const tx = await program.methods
      .unstake(unstakeAmount)
      .accounts({
        user,
        pool: poolPda,
        userStake: userStakePda,
        userStakeAta,
        stakeVault,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Unstake Tx:", tx);

    const userStakeAccount = await getAccount(connection, userStakeAta);
    console.log("User Stake ATA Balance After Unstake:", userStakeAccount.amount.toString());

    const userStakeData = await program.account.userStake.fetch(userStakePda);
    console.log("User Staked Amount After Unstake:", userStakeData.amountStaked.toString());

    const poolAccount = await program.account.pool.fetch(poolPda);
    console.log("Pool Total Staked After Unstake:", poolAccount.totalStaked.toString());
  });

  it("Test multiple users staking", async () => {
    const user2 = Keypair.generate();
    
    // Airdrop SOL to user2
    const airdropSig = await connection.requestAirdrop(user2.publicKey, 2_000_000_000); // 2 SOL
    await connection.confirmTransaction(airdropSig);

    // Create ATAs for user2
    const user2StakeAta = await getAssociatedTokenAddress(stakeMint, user2.publicKey);
    const user2RewardAta = await getAssociatedTokenAddress(rewardMint, user2.publicKey);

    await createAssociatedTokenAccount(
      connection,
      wallet.payer,
      stakeMint,
      user2.publicKey
    );
    
    await createAssociatedTokenAccount(
      connection,
      wallet.payer,
      rewardMint,
      user2.publicKey
    );

    await mintTo(
      connection,
      wallet.payer,
      stakeMint,
      user2StakeAta,
      wallet.payer,
      5_000_000
    );

    const [user2StakePda] = getUserStakePda(poolPda, user2.publicKey);

    const stakeAmount = new anchor.BN(2_000_000);

    const tx = await program.methods
      .stake(stakeAmount)
      .accounts({
        user: user2.publicKey,
        pool: poolPda,
        userStake: user2StakePda,
        userStakeAta: user2StakeAta,
        stakeVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user2])
      .rpc();

    console.log("User2 Stake Tx:", tx);

    const [user1StakePda] = getUserStakePda(poolPda, admin);
    const user1StakeData = await program.account.userStake.fetch(user1StakePda);
    const user2StakeData = await program.account.userStake.fetch(user2StakePda);

    console.log("User1 Staked:", user1StakeData.amountStaked.toString());
    console.log("User2 Staked:", user2StakeData.amountStaked.toString());

    const poolAccount = await program.account.pool.fetch(poolPda);
    console.log("Total Staked (Both Users):", poolAccount.totalStaked.toString());
  });
});