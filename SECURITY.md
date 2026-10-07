# 🛡️ Aegis Protocol — Security Architecture & Threat Model

> **Targeted Pre-Audit Review Document for Adevar Labs (SEAL-Certified Auditor)**

---

## 1. Executive Security Summary

Security in Aegis Protocol is treated as a fundamental architectural pillar rather than an afterthought. The protocol mediates high-value conditional transfers, rate-limited liquidity vaults, and automated circuit-breaker tripwires on the Solana blockchain.

This document formalizes our threat assumptions, invariant guarantees, and the specific attack vectors we request Adevar Labs to prioritize during the Pre-Audit review.

---

## 2. Protocol Invariants

The protocol guarantees the following mathematical invariants across all execution paths:

### Invariant 1: Total Liability Solvency
$$\text{Balance}(\text{VaultTokenAccount}) \ge \text{total\_active\_escrow\_amount}$$
* Every token deposited into an active escrow MUST be accounted for in `total_active_escrow_amount`.
* No tokens can be released or refunded without decrementing `total_active_escrow_amount` via verified checked arithmetic.

### Invariant 2: Non-Exceedable Epoch Flow Rate
$$\sum_{i=1}^{k} \text{withdrawal}_i \le \text{max\_epoch\_withdrawal\_limit} \quad \forall \; t \in [\text{epoch\_start}, \text{epoch\_start} + \text{epoch\_duration}]$$
* Mitigates flash drains. If cumulative withdrawals within a sliding window hit the limit, transactions fail with `AegisError::RateLimitExceeded`.

### Invariant 3: Single Settlement Terminality
* An `EscrowAccount` can transition to `is_released == true` OR `is_cancelled == true` exactly once. Re-entry, duplicate releases, or double-refunds are prevented by explicit state assertion and zeroing logic.

### Invariant 4: Dual-Role Privileged Separation
* `guardian`: Can invoke `emergency_pause()` immediately, but **cannot** call `emergency_unpause()`, withdraw funds, or alter protocol parameters.
* `admin`: Required to invoke `emergency_unpause()` and modify governance parameters.

---

## 3. Solana Attack Vectors & Mitigations

| Attack Vector | Vulnerability Description | Aegis Protocol Mitigation |
| :--- | :--- | :--- |
| **Account Confusions / Fake Accounts** | Attacker passes malicious token accounts or impostor mints. | Strict Anchor `has_one = token_mint`, `has_one = vault_token_account`, and PDA derivation seed matching. |
| **Missing Signer Checks** | Unauthenticated callers invoking privileged instructions. | Explicit `Signer<'info>` type constraints on caller keys, verified against stored public keys. |
| **Integer Overflows / Underflows** | Overflowing counters or balances to wrap amounts. | Mandatory use of Rust's `checked_add()`, `checked_sub()`, and `overflow-checks = true` in `Cargo.toml`. |
| **Timestamp Manipulation** | Miners or validators drifting timestamps. | Timelocks operate on macro intervals (hours/days). Short-interval reliance is strictly prohibited in economic logic. |
| **Reentrancy via CPI** | Malicious programs exploiting nested calls during token transfers. | Checks-Effects-Interactions pattern: state updates occur prior to or atomically alongside CPI token transfers, guarded by Anchor discriminator checks. |
| **PDA Seed Collision** | Two users colliding on the same Escrow PDA. | Deterministic seeds `[b"aegis_escrow", sender.key(), escrow_id.to_le_bytes()]` guarantee mathematical uniqueness per user. |

---

## 4. Priority Areas for Adevar Labs Pre-Audit

We specifically request Adevar Labs security engineers to focus on:
1. **Epoch Rollover Edge Cases**: Review the sliding window logic in `release_escrow.rs` when crossing epoch boundaries during rapid successive blocks.
2. **PDA Signer Seeds & Bump Security**: Validate that the seeds passed to `token::transfer` via CPI strictly enforce canonical bumps and cannot be abused by foreign programs.
3. **Token-2022 Transfer Hook Compatibility**: Potential interactions when integrating Token-2022 mints with custom fee-on-transfer or transfer-hook extensions.

---

## 5. Responsible Disclosure & Reporting

- **Contact**: `security@aegis-protocol.io`
- **PGP Key**: Published on protocol website & keybase.
