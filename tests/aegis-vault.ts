import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";

describe("Aegis escrow security", function () {
  this.timeout(120000);
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.AegisVault as anchor.Program<any>;
  const payer = (provider.wallet as anchor.Wallet).payer;
  const sender = Keypair.generate(),
    recipient = Keypair.generate();
  const guardian = Keypair.generate(),
    arbiter = Keypair.generate(),
    stranger = Keypair.generate();
  const bn = (n: number) => new anchor.BN(n);
  const accounts = program.account as any;
  let mint: PublicKey, senderToken: PublicKey, recipientToken: PublicKey;
  type Vault = {
    admin: Keypair;
    mint: PublicKey;
    config: PublicKey;
    token: PublicKey;
    senderToken: PublicKey;
    recipientToken: PublicKey;
  };

  async function fund(key: PublicKey) {
    await provider.sendAndConfirm(
      new anchor.web3.Transaction().add(
        SystemProgram.transfer({
          fromPubkey: payer.publicKey,
          toPubkey: key,
          lamports: 100_000_000,
        })
      )
    );
  }
  async function chainTime() {
    const info = await provider.connection.getAccountInfo(
      anchor.web3.SYSVAR_CLOCK_PUBKEY,
      "confirmed"
    );
    if (!info) throw new Error("Clock sysvar unavailable");
    return Number(info.data.readBigInt64LE(32));
  }
  async function waitUntil(timestamp: number) {
    const deadline = Date.now() + 45000;
    while ((await chainTime()) < timestamp) {
      if (Date.now() > deadline)
        throw new Error("Validator clock did not reach deadline");
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  }
  async function rejects(action: Promise<unknown>, code: string) {
    let failure: any;
    try {
      await action;
    } catch (err) {
      failure = err;
    }
    expect(failure, `Expected ${code}, but transaction succeeded`).to.exist;
    expect(failure.error?.errorCode?.code, String(failure)).to.equal(code);
  }
  before(async () => {
    for (const actor of [sender, recipient, guardian, arbiter, stranger])
      await fund(actor.publicKey);
    mint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      6
    );
    senderToken = await createAccount(
      provider.connection,
      payer,
      mint,
      sender.publicKey
    );
    recipientToken = await createAccount(
      provider.connection,
      payer,
      mint,
      recipient.publicKey
    );
    await mintTo(
      provider.connection,
      payer,
      mint,
      senderToken,
      payer,
      1_000_000_000
    );
  });
  async function vault(
    limit = 100,
    duration = 3600,
    otherMint = false
  ): Promise<Vault> {
    const admin = Keypair.generate();
    await fund(admin.publicKey);
    const m = otherMint
      ? await createMint(provider.connection, payer, payer.publicKey, null, 6)
      : mint;
    const from = otherMint
      ? await createAccount(provider.connection, payer, m, sender.publicKey)
      : senderToken;
    const to = otherMint
      ? await createAccount(provider.connection, payer, m, recipient.publicKey)
      : recipientToken;
    if (otherMint)
      await mintTo(provider.connection, payer, m, from, payer, 10000);
    const [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("aegis_vault"), admin.publicKey.toBuffer(), m.toBuffer()],
      program.programId
    );
    const [token] = PublicKey.findProgramAddressSync(
      [Buffer.from("aegis_token_vault"), config.toBuffer()],
      program.programId
    );
    await program.methods
      .initializeVault(bn(limit), bn(duration))
      .accountsStrict({
        admin: admin.publicKey,
        guardian: guardian.publicKey,
        tokenMint: m,
        vaultConfig: config,
        vaultTokenAccount: token,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([admin])
      .rpc();
    return {
      admin,
      mint: m,
      config,
      token,
      senderToken: from,
      recipientToken: to,
    };
  }
  function address(v: Vault, id: number) {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from("aegis_escrow"),
        v.config.toBuffer(),
        sender.publicKey.toBuffer(),
        bn(id).toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    )[0];
  }
  async function create(
    v: Vault,
    id: number,
    amount = 50,
    release?: number,
    expiry?: number,
    mediator = arbiter.publicKey
  ) {
    const now = await chainTime(),
      escrow = address(v, id);
    await program.methods
      .createEscrow(
        bn(id),
        bn(amount),
        bn(release ?? now - 1),
        bn(expiry ?? now + 3600)
      )
      .accountsStrict({
        sender: sender.publicKey,
        recipient: recipient.publicKey,
        arbiter: mediator,
        tokenMint: v.mint,
        vaultConfig: v.config,
        senderTokenAccount: v.senderToken,
        vaultTokenAccount: v.token,
        escrowAccount: escrow,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([sender])
      .rpc();
    return escrow;
  }
  function release(
    v: Vault,
    e: PublicKey,
    caller = recipient,
    destination = v.recipientToken
  ) {
    return program.methods
      .releaseEscrow()
      .accountsStrict({
        caller: caller.publicKey,
        recipientTokenAccount: destination,
        tokenMint: v.mint,
        vaultConfig: v.config,
        vaultTokenAccount: v.token,
        escrowAccount: e,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([caller])
      .rpc();
  }
  function refund(v: Vault, e: PublicKey, caller = recipient) {
    return program.methods
      .cancelEscrow()
      .accountsStrict({
        caller: caller.publicKey,
        senderTokenAccount: v.senderToken,
        tokenMint: v.mint,
        vaultConfig: v.config,
        vaultTokenAccount: v.token,
        escrowAccount: e,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([caller])
      .rpc();
  }
  function pause(v: Vault, caller = guardian, resume = false) {
    return (
      resume
        ? program.methods.emergencyUnpause()
        : program.methods.emergencyPause()
    )
      .accountsStrict({
        authority: caller.publicKey,
        tokenMint: v.mint,
        vaultConfig: v.config,
      })
      .signers([caller])
      .rpc();
  }
  function update(
    v: Vault,
    limit: number | null,
    duration: number | null = null,
    caller = v.admin
  ) {
    return program.methods
      .updateVaultParams(
        null,
        limit === null ? null : bn(limit),
        duration === null ? null : bn(duration)
      )
      .accountsStrict({
        admin: caller.publicKey,
        tokenMint: v.mint,
        vaultConfig: v.config,
      })
      .signers([caller])
      .rpc();
  }
  async function snapshot(v: Vault, e: PublicKey) {
    const c = await accounts.vaultConfig.fetch(v.config),
      s = await accounts.escrowAccount.fetch(e);
    return {
      balance: (
        await getAccount(provider.connection, v.token)
      ).amount.toString(),
      liability: c.totalActiveEscrowAmount.toString(),
      spent: c.currentEpochWithdrawn.toString(),
      start: c.currentEpochStart.toString(),
      released: s.isReleased,
      cancelled: s.isCancelled,
    };
  }
  it("isolates admins using the same mint and binds escrow to its vault", async () => {
    const a = await vault(),
      b = await vault(),
      ea = await create(a, 1),
      eb = await create(b, 1);
    expect(a.config.equals(b.config)).to.equal(false);
    expect(ea.equals(eb)).to.equal(false);
    expect(
      (await accounts.escrowAccount.fetch(ea)).vaultConfig.equals(a.config)
    ).to.equal(true);
    expect((await snapshot(a, ea)).liability).to.equal("50");
  });
  for (const otherMint of [false, true]) {
    it(`rejects release AND refund against a funded ${
      otherMint ? "different-mint" : "same-mint"
    } vault`, async () => {
      const a = await vault(),
        b = await vault(100, 3600, otherMint);
      const ea = await create(a, 1),
        eb = await create(b, 1);
      const beforeA = await snapshot(a, ea),
        beforeB = await snapshot(b, eb);
      await rejects(release(b, ea), "InvalidVault");
      await rejects(refund(b, ea), "InvalidVault");
      expect(await snapshot(a, ea)).to.deep.equal(beforeA);
      expect(await snapshot(b, eb)).to.deep.equal(beforeB);
    });
  }
  it("enforces recipient timelock, then releases with conserved balances", async () => {
    const v = await vault(),
      now = await chainTime(),
      e = await create(v, 1, 50, now + 8);
    const before = await snapshot(v, e);
    await rejects(release(v, e), "TimelockNotElapsed");
    expect(await snapshot(v, e)).to.deep.equal(before);
    const bal = (await getAccount(provider.connection, v.recipientToken))
      .amount;
    await waitUntil(now + 8);
    await release(v, e);
    expect(
      (await getAccount(provider.connection, v.recipientToken)).amount - bal
    ).to.equal(BigInt(50));
    expect(await snapshot(v, e)).to.include({
      balance: "0",
      liability: "0",
      spent: "50",
      released: true,
    });
  });
  it("permits explicitly trusted sender and arbiter early release", async () => {
    const v = await vault(),
      now = await chainTime();
    await release(v, await create(v, 1, 50, now + 600), sender);
    await release(v, await create(v, 2, 50, now + 600), arbiter);
  });
  it("rejects strangers, wrong destination owners and guardian settlement", async () => {
    const v = await vault(),
      e = await create(v, 1),
      before = await snapshot(v, e);
    for (const caller of [stranger, guardian]) {
      await rejects(release(v, e, caller), "UnauthorizedSigner");
      await rejects(refund(v, e, caller), "UnauthorizedSigner");
    }
    await rejects(
      release(v, e, recipient, v.senderToken),
      "UnauthorizedSigner"
    );
    expect(await snapshot(v, e)).to.deep.equal(before);
  });
  it("rejects destination with wrong mint", async () => {
    const v = await vault(),
      b = await vault(100, 3600, true),
      e = await create(v, 1);
    await rejects(release(v, e, recipient, b.recipientToken), "InvalidMint");
  });
  for (const cancelFirst of [false, true]) {
    it(`rejects both settlement paths after ${
      cancelFirst ? "refund" : "release"
    }`, async () => {
      const v = await vault(),
        e = await create(v, 1);
      await (cancelFirst ? refund(v, e) : release(v, e));
      const before = await snapshot(v, e);
      await rejects(release(v, e), "EscrowAlreadySettled");
      await rejects(refund(v, e), "EscrowAlreadySettled");
      expect(await snapshot(v, e)).to.deep.equal(before);
    });
  }
  for (const refundFirst of [false, true]) {
    it(`shares outflow budget: ${
      refundFirst ? "refund then release" : "release then refund"
    }`, async () => {
      const v = await vault(),
        a = await create(v, 1, 60),
        b = await create(v, 2, 50);
      await (refundFirst ? refund(v, a) : release(v, a));
      const before = await snapshot(v, b);
      await rejects(
        refundFirst ? release(v, b) : refund(v, b),
        "RateLimitExceeded"
      );
      expect(await snapshot(v, b)).to.deep.equal(before);
    });
  }
  it("rejects oversized deposit before transferring tokens", async () => {
    const v = await vault(),
      balance = (await getAccount(provider.connection, v.senderToken)).amount;
    await rejects(create(v, 1, 101), "EscrowExceedsLimit");
    expect(await provider.connection.getAccountInfo(address(v, 1))).to.equal(
      null
    );
    expect(
      (await getAccount(provider.connection, v.senderToken)).amount
    ).to.equal(balance);
  });
  it("rejects zero deposits and invalid or expired deadlines", async () => {
    const v = await vault(),
      now = await chainTime();
    await rejects(create(v, 1, 0), "ZeroAmountInvalid");
    await rejects(
      create(v, 1, 50, now + 10, now + 10),
      "InvalidTimelockConfiguration"
    );
    await rejects(create(v, 1, 50, now - 10, now - 1), "EscrowExpired");
  });
  it("protects outstanding escrow sizes against admin limit reductions", async () => {
    const v = await vault(),
      e = await create(v, 1, 100);
    await rejects(update(v, 99), "ActiveEscrowsPreventLimitDecrease");
    await release(v, e);
    await update(v, 50);
    expect(
      (
        await accounts.vaultConfig.fetch(v.config)
      ).maxEpochWithdrawalLimit.toString()
    ).to.equal("50");
  });
  it("does not reset consumed budget when admin changes parameters", async () => {
    const v = await vault(),
      e = await create(v, 1, 80);
    await release(v, e);
    await update(v, 120, 7200);
    const b = await create(v, 2, 50);
    await rejects(release(v, b), "RateLimitExceeded");
    expect((await snapshot(v, b)).spent).to.equal("80");
  });
  it("guardian cannot unpause or change parameters; strangers cannot pause", async () => {
    const v = await vault();
    await rejects(pause(v, stranger), "UnauthorizedSigner");
    await pause(v);
    await rejects(pause(v, guardian, true), "UnauthorizedSigner");
    await rejects(update(v, 200, null, guardian), "UnauthorizedSigner");
    await pause(v, v.admin, true);
    expect((await accounts.vaultConfig.fetch(v.config)).isPaused).to.equal(
      false
    );
  });
  it("pause blocks deposits, releases and refunds without changing balances", async () => {
    const v = await vault(),
      e = await create(v, 1),
      before = await snapshot(v, e);
    await pause(v);
    await rejects(create(v, 2), "VaultPaused");
    await rejects(release(v, e), "VaultPaused");
    await rejects(refund(v, e), "VaultPaused");
    expect(await snapshot(v, e)).to.deep.equal(before);
    await pause(v, v.admin, true);
    await release(v, e);
  });
  it("sender waits for expiry to refund; expired escrow cannot release", async () => {
    const v = await vault(),
      now = await chainTime(),
      e = await create(v, 1, 50, now - 1, now + 8, SystemProgram.programId);
    await rejects(refund(v, e, sender), "EscrowNotExpiredYet");
    await waitUntil(now + 8);
    await rejects(release(v, e, sender), "EscrowExpired");
    await refund(v, e, sender);
    expect(await snapshot(v, e)).to.include({
      liability: "0",
      balance: "0",
      cancelled: true,
    });
  });
  it("arbiter refunds early, with funds always returned to sender", async () => {
    const v = await vault(),
      e = await create(v, 1),
      before = (await getAccount(provider.connection, v.senderToken)).amount;
    await refund(v, e, arbiter);
    expect(
      (await getAccount(provider.connection, v.senderToken)).amount - before
    ).to.equal(BigInt(50));
  });
  it("demo: budget rejection, guardian pause, admin recovery, next-window settlement", async () => {
    const v = await vault(),
      a = await create(v, 1, 60),
      b = await create(v, 2, 50);
    await release(v, a);
    await rejects(release(v, b), "RateLimitExceeded");
    await pause(v);
    await rejects(refund(v, b), "VaultPaused");
    await pause(v, v.admin, true);
    const c = await accounts.vaultConfig.fetch(v.config),
      boundary = (await chainTime()) + 4;
    await update(v, null, boundary - c.currentEpochStart.toNumber());
    await waitUntil(boundary);
    await refund(v, b);
    expect(await snapshot(v, b)).to.include({
      liability: "0",
      balance: "0",
      spent: "50",
      cancelled: true,
    });
  });
});
