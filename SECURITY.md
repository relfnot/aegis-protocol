# Aegis security model

Status: unaudited prototype. This document describes intended and implemented controls, not a certification or formal proof. Read VALIDATION.md for checks actually performed.

## Assets and account binding

Vaults hold classic SPL tokens on behalf of active escrows. Vault configuration is derived from admin and mint, and records its token account. Each escrow stores its vault public key and includes that key in its seeds. Both settlement instructions verify the stored vault binding. Token account mint, authority and configured address are checked.

Different admins may operate independent vaults for the same mint. Clients must display and verify the chosen admin and vault address before deposit.

## Permission and time rules

| Operation | Who | Conditions |
| --- | --- | --- |
| Create | Sender | Amount positive and no greater than cap; release < expiry; expiry in future; unpaused |
| Release | Recipient | release <= chain time < expiry; unpaused; budget available |
| Early release | Sender or nonzero arbiter | chain time < expiry; unpaused; budget available; destination remains recipient |
| Refund | Sender | chain time >= expiry; unpaused; budget available |
| Early refund | Recipient or nonzero arbiter | unpaused; budget available; destination remains sender |
| Pause | Guardian or admin | No settlement rights conferred by role |
| Unpause | Admin | Guardian cannot resume |
| Change parameters | Admin | Positive cap/duration; cannot lower cap with outstanding liabilities |

System Program/default public key indicates no arbiter. Roles may overlap. An arbiter who is also sender can refund early. A sender who is also recipient but is not arbiter still waits until expiry to refund. A sender who is also recipient can release early as sender. Applications should disclose these overlaps.

A past release timestamp is allowed for immediate claims. At expiry, release is rejected and refund is the remaining settlement path. Admin parameter changes remain allowed while paused; pause blocks deposits and fund movement, not recovery configuration.

## Accounting properties

- Actual token balance must be at least recorded active liabilities before and after each deposit or settlement. The token account is reloaded after CPI before checking post-transfer solvency.
- Active liabilities increase by the deposited amount and decrease once on settlement, using checked arithmetic.
- Escrow can settle once; failed transactions must leave balances, status and counters unchanged.
- Release and refund call the same outflow accounting method.
- No partial settlement. Oversized escrows are rejected before transfer. Lowering limits while liabilities exist is rejected to avoid making previously accepted escrows too large to settle.
- Direct token donations may create surplus. There is no surplus withdrawal instruction.

## Window semantics and limitations

The window resets on the first successful outflow at or after start + duration. It is not a sliding window or a Solana consensus epoch. The exact boundary allows a new budget, and two windows can permit nearly twice the cap within a short interval. Admin edits preserve existing spent amount and start but may change when the next reset happens. Refunds can wait for budget. There is no fairness queue between competing claims.

## Trust assumptions

Admin is trusted to configure sensible budgets and durations and recover from pause. A malicious or unavailable admin can weaken limits or prevent settlement. A compromised guardian can pause indefinitely until admin intervenes. An arbiter can choose early release or refund and is trusted for that decision; no on-chain dispute evidence is assessed. Program upgrade authority, if retained, is trusted. Token mint/freeze authorities retain the powers of the underlying token program. There is no automatic anomaly detector and no guarantee that a guardian can react before an exploit.

Only classic SPL Token is accepted by the handlers. Token-2022 support is not claimed. No protection is promised for other protocols' assets that do not enter these vaults.

## Review priorities

1. Cross-vault/cross-mint account substitution, including same-mint different-admin vaults.
2. Release/refund shared budget, exact rollover boundaries and admin parameter changes.
3. Role overlap, early arbiter actions, expiry exclusivity and double settlement.
4. Actual reserve accounting, failed CPI rollback and malformed token accounts.
5. Pause availability tradeoffs, operational key management and upgrade authority.

Rust tests cover arithmetic and exact boundaries. Integration tests exercise account constraints, CPI transfers, authorization and rejected-transaction state preservation. Passing tests are evidence, not an audit or exhaustive proof.

## Migration and reporting

This version changes both PDA derivation and EscrowAccount layout. No migration is implemented. Use a fresh test deployment; a live migration needs its own review and recovery plan.

No verified private disclosure email or PGP key has been supplied. The project owner must publish a working private contact before accepting public deposits. Do not claim an invented address or key as an existing reporting channel.
