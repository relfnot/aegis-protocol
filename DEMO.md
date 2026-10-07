# Repeatable local demo

Audience: Solana teams paying contributors in escrow tranches.

Run the full setup and tests in README.md first. With a local validator running and this revision loaded, set `ANCHOR_PROVIDER_URL` and `ANCHOR_WALLET`, then run:

```sh
npm run demo
```

The demo is the executable integration scenario named `demo:` in tests/aegis-vault.ts. It creates fresh test accounts each run and never requires real tokens. It proves transaction behavior through assertions; this repository does not yet include a web interface or a recorded video.

## Two-minute narration

1. A team configures a vault with a 100-unit outflow budget and creates two escrow tranches of 60 and 50 units.
2. The first tranche pays successfully. The second payment is rejected because total outflow would be 110.
3. The guardian pauses the vault. A refund is also rejected while paused.
4. The admin resumes the vault. Resuming does not replenish the budget.
5. For the local demo only, the admin shortens the window. The test waits for the chain clock to reach its end, then refunds the second tranche using the new window's budget.
6. The final assertions verify zero outstanding liabilities, zero vault balance and 50 units consumed in the new window.

Explain that refund and release share one cap, guardians cannot resume service, and the admin remains trusted. Window shortening in this demonstration is an admin privilege, not a way for ordinary claimants to bypass the cap. Separate tests verify guardian cannot change parameters or unpause.

## Evidence to capture for submission

Record the checked-out commit, build result, test summary and this demo result. For a public devnet demonstration, publish the actual program ID, deployment transaction and explorer links after deployment; do not use the repository's development ID as deployment evidence.

A public deployment, wallet funding, recorded video and social post are not included in the local test run.
