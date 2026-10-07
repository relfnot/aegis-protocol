# Validation record

Date: 2026-10-07. Scope: local working-tree revision, not an independent audit or a public deployment.

## Observed results

| Check | Result |
| --- | --- |
| `cargo test --lib --locked` | PASS: 7 tests, including 6 accounting/boundary tests and Anchor program-ID test |
| `anchor build` | PASS: SBF program and IDL generated |
| `npm run test:types` | PASS |
| `npm run lint` | PASS |
| `npm test` against local Agave validator | PASS: 20 tests, approximately one minute |
| Executable demo | PASS as part of the integration suite |
| `git diff --check` | PASS |
| `cargo clippy --all-targets --locked` | PASS with existing Anchor/Solana macro cfg warnings (no errors) |
| GitHub Actions workflow | Updated; no remote CI run claimed |
| Devnet deployment | Not performed or verified |
| Independent security audit | Not performed |

## Transaction coverage

- Independent admins sharing one mint; escrow address and stored vault binding.
- Release and refund against a funded wrong vault, both same and different mint, with unchanged state after rejection.
- Recipient timelock, trusted sender/arbiter early release, expiry/refund behavior.
- Unauthorized callers, guardian settlement attempts and wrong destination owner/mint.
- Duplicate release/refund and each settlement path after the other.
- Shared release/refund budget in both directions.
- Oversized deposits rejected before tokens move; zero amounts and invalid deadlines.
- Limit decreases blocked while liabilities exist; parameter edits retain spent budget.
- Guardian cannot unpause or reconfigure; unauthorized pause rejected.
- Pause blocks deposit, payment and refund without changing escrow balances.
- Local demo: exhausted budget, pause, admin resume and settlement in next window.

Rust tests separately verify the exact rollover timestamp, long idle intervals, failed rollover state, overflow, backwards time, reserve shortfall/donation checks and serialized account allocation sizes.

## Environment

- Windows Node.js 24.12.0; TypeScript 5.9.3; Mocha 11.8.0.
- Ubuntu/WSL Cargo 1.94.1; Agave/Solana CLI and validator 3.1.12.
- Installed Anchor CLI 0.32.1; contract and JS Anchor client 0.30.1.
- SPL Token client 0.4.15; web3.js client 1.99.0.
- npm lockfile is included. Rust lockfile was already present as an untracked file and was preserved/used for testing.

The Anchor CLI emits a version-mismatch warning with the 0.30.1 library. This combination built and passed the local integration suite; it is not a claim of universal compatibility. Legacy Anchor/Solana macros also emit cfg warnings under the newer Rust compiler. The JS bigint package used its pure-JavaScript fallback in this environment.

## Local execution details

A disposable wallet in ignored `target/` received 100 simulated SOL from the local validator. No public-chain transaction or real asset was used. The validator loaded the generated SBF file directly at the development program ID; this bypasses public deployment and upgrade-authority setup.

Example Linux setup for repeating that approach on a fresh ledger:

```sh
solana-keygen new --no-bip39-passphrase --silent --outfile target/local-test-wallet.json
solana-test-validator --ledger target/local-demo-ledger --bind-address 127.0.0.1 \
  --bpf-program Fg6PaFpoGXkYsidMpWxTWqkZqXmJbYyHhP7FZxQzLhQZ target/deploy/aegis_vault.so
# In another terminal:
solana airdrop 100 --url http://127.0.0.1:8899 --keypair target/local-test-wallet.json
export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
export ANCHOR_WALLET="$PWD/target/local-test-wallet.json"
npm test
npm run demo
```

If `anchor keys sync` changes the development ID, use the synchronized ID above instead. Do not reuse a ledger that already contains a different program revision.

Tested SBF SHA-256:

`3950b6cb02f88124146e57a5184e531d71eb5b3181d52bafc16267b7410ef22b`

## Outstanding dependency findings

After compatible `npm audit fix`, npm reported **15 findings: 1 low, 8 moderate and 6 high**. These include transitive findings through Anchor, Solana clients and test tooling (including bigint-buffer, toml and serialization dependencies). Some have no compatible fix; suggested force changes include incompatible major versions or downgrades. No forced dependency replacement was applied.

These are dependency-audit findings, not 15 demonstrated vulnerabilities in the Rust contract. They remain a production-readiness limitation. A dependency migration needs separate compatibility and security testing. The successful tests do not clear these findings.

## Scope limits

No claim is made about exhaustive attack coverage, formal verification, live customer adoption, private security reporting setup, public devnet readiness, mainnet safety or bounty eligibility. Account layout and PDA changes require fresh test accounts; there is no live migration in this revision.
