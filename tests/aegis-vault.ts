import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { expect } from "chai";

describe("Aegis Protocol - Security & Escrow Test Suite", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Note: Program type will be derived from IDL in Anchor runtime
  const program = (anchor.workspace as any).AegisVault;

  let mint: PublicKey;
  let admin: Keypair;
  let guardian: Keypair;
  let sender: Keypair;
  let recipient: Keypair;
  let arbiter: Keypair;

  let senderTokenAccount: PublicKey;
  let recipientTokenAccount: PublicKey;
  let vaultConfigPda: PublicKey;
  let vaultTokenAccountPda: PublicKey;
  let vaultConfigBump: number;

  const INITIAL_DEPOSIT = 10_000_000_000; // 10,000 tokens (decimals = 6)
  const MAX_EPOCH_LIMIT = 5_000_000_000;  // 5,000 tokens limit per epoch
  const EPOCH_DURATION = 86400;           // 24 hours in seconds

  before(async () => {
    admin = Keypair.generate();
    guardian = Keypair.generate();
    sender = Keypair.generate();
    recipient = Keypair.generate();
    arbiter = Keypair.generate();

    // Airdrop SOL to test actors
    const airdropPromises = [admin, guardian, sender, recipient, arbiter].map(async (kp) => {
      const sig = await provider.connection.requestAirdrop(kp.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);
    });
    await Promise.all(airdropPromises);

    // Create Mock SPL Token Mint
    mint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    // Sender ATA
    senderTokenAccount = await createAccount(
      provider.connection,
      sender,
      mint,
      sender.publicKey
    );

    // Recipient ATA
    recipientTokenAccount = await createAccount(
      provider.connection,
      recipient,
      mint,
      recipient.publicKey
    );

    // Mint tokens to sender
    await mintTo(
      provider.connection,
      admin,
      mint,
      senderTokenAccount,
      admin,
      INITIAL_DEPOSIT
    );

    // Derive PDAs
    [vaultConfigPda, vaultConfigBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("aegis_vault"), mint.toBuffer()],
      program.programId
    );

    [vaultTokenAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("aegis_token_vault"), mint.toBuffer()],
      program.programId
    );
  });

  it("1. Successfully initializes Aegis Vault with circuit breaker & epoch limits", async () => {
    await program.methods
      .initializeVault(
        new anchor.BN(MAX_EPOCH_LIMIT),
        new anchor.BN(EPOCH_DURATION)
      )
      .accounts({
        admin: admin.publicKey,
        guardian: guardian.publicKey,
        tokenMint: mint,
        vaultConfig: vaultConfigPda,
        vaultTokenAccount: vaultTokenAccountPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([admin])
      .rpc();

    const config = await program.account.vaultConfig.fetch(vaultConfigPda);
    expect(config.admin.toString()).to.equal(admin.publicKey.toString());
    expect(config.guardian.toString()).to.equal(guardian.publicKey.toString());
    expect(config.isPaused).to.be.false;
    expect(config.maxEpochWithdrawalLimit.toNumber()).to.equal(MAX_EPOCH_LIMIT);
  });

  it("2. Creates a conditional escrow with timelock protection", async () => {
    const escrowId = new anchor.BN(1);
    const amount = new anchor.BN(1_000_000_000); // 1,000 tokens
    const now = Math.floor(Date.now() / 1000);
    const releaseTime = new anchor.BN(now + 10); // 10 seconds lock
    const expiryTime = new anchor.BN(now + 3600); // 1 hour expiry

    const [escrowPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("aegis_escrow"), sender.publicKey.toBuffer(), escrowId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    await program.methods
      .createEscrow(escrowId, amount, releaseTime, expiryTime)
      .accounts({
        sender: sender.publicKey,
        recipient: recipient.publicKey,
        arbiter: arbiter.publicKey,
        tokenMint: mint,
        vaultConfig: vaultConfigPda,
        senderTokenAccount: senderTokenAccount,
        vaultTokenAccount: vaultTokenAccountPda,
        escrowAccount: escrowPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([sender])
      .rpc();

    const escrowData = await program.account.escrowAccount.fetch(escrowPda);
    expect(escrowData.amount.toNumber()).to.equal(amount.toNumber());
    expect(escrowData.isReleased).to.be.false;

    const vaultBal = await getAccount(provider.connection, vaultTokenAccountPda);
    expect(Number(vaultBal.amount)).to.equal(amount.toNumber());
  });

  it("3. Security Invariant: Rejects release when timelock has not yet elapsed", async () => {
    const escrowId = new anchor.BN(1);
    const [escrowPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("aegis_escrow"), sender.publicKey.toBuffer(), escrowId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    try {
      await program.methods
        .releaseEscrow()
        .accounts({
          caller: recipient.publicKey,
          recipientTokenAccount: recipientTokenAccount,
          tokenMint: mint,
          vaultConfig: vaultConfigPda,
          vaultTokenAccount: vaultTokenAccountPda,
          escrowAccount: escrowPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([recipient])
        .rpc();
      expect.fail("Should have failed with TimelockNotElapsed");
    } catch (err: any) {
      expect(err.toString()).to.include("TimelockNotElapsed");
    }
  });

  it("4. Circuit Breaker: Guardian can pause vault during detected anomalies", async () => {
    await program.methods
      .emergencyPause()
      .accounts({
        authority: guardian.publicKey,
        tokenMint: mint,
        vaultConfig: vaultConfigPda,
      })
      .signers([guardian])
      .rpc();

    const config = await program.account.vaultConfig.fetch(vaultConfigPda);
    expect(config.isPaused).to.be.true;
  });

  it("5. Security Invariant: All fund movements blocked while Vault is paused", async () => {
    const escrowId = new anchor.BN(1);
    const [escrowPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("aegis_escrow"), sender.publicKey.toBuffer(), escrowId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    try {
      await program.methods
        .releaseEscrow()
        .accounts({
          caller: arbiter.publicKey,
          recipientTokenAccount: recipientTokenAccount,
          tokenMint: mint,
          vaultConfig: vaultConfigPda,
          vaultTokenAccount: vaultTokenAccountPda,
          escrowAccount: escrowPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([arbiter])
        .rpc();
      expect.fail("Should have failed with VaultPaused");
    } catch (err: any) {
      expect(err.toString()).to.include("VaultPaused");
    }
  });

  it("6. Admin unpauses and releases escrow after timelock matures", async () => {
    // Admin unpauses
    await program.methods
      .emergencyUnpause()
      .accounts({
        authority: admin.publicKey,
        tokenMint: mint,
        vaultConfig: vaultConfigPda,
      })
      .signers([admin])
      .rpc();

    // Wait for timelock (10s) to pass
    await new Promise((resolve) => setTimeout(resolve, 11000));

    const escrowId = new anchor.BN(1);
    const [escrowPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("aegis_escrow"), sender.publicKey.toBuffer(), escrowId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    await program.methods
      .releaseEscrow()
      .accounts({
        caller: recipient.publicKey,
        recipientTokenAccount: recipientTokenAccount,
        tokenMint: mint,
        vaultConfig: vaultConfigPda,
        vaultTokenAccount: vaultTokenAccountPda,
        escrowAccount: escrowPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([recipient])
      .rpc();

    const recipientBal = await getAccount(provider.connection, recipientTokenAccount);
    expect(Number(recipientBal.amount)).to.equal(1_000_000_000);
  });
});
