# Application draft: Aegis Protocol

This is a preparation document. Actual form questions and current bounty availability have not been verified. It does not assert that an application, tweet, follow, or hackathon submission has occurred.

## Project name and tagline

Aegis Protocol — Escrow payments for Solana teams with shared outflow limits and guardian emergency pause.

## Project overview

Aegis is an unaudited Rust/Anchor prototype for teams paying contributors and service providers through scheduled escrow tranches. Each escrow is bound to one independently administered vault. Payments and refunds consume a shared reset-window budget, while a guardian can pause deposits and settlements. The admin controls recovery and configuration. Senders or designated arbiters can authorize early payment, and the recipient or arbiter can authorize a refund. These permissions are explicit trust assumptions, not automatic dispute adjudication.

## Architecture and technical readiness

The implementation separates initialization, deposit, release, refund and emergency controls. Vault addresses include admin and mint, and escrow addresses include the vault, sender and escrow ID. Settlement validates the stored vault relationship and token accounts. Both outflow paths share checked budget accounting and reserve-versus-liability checks. Oversized escrows are rejected; administrators cannot reduce the cap while active liabilities remain.

The repository includes Rust boundary tests and transaction-level regression tests for cross-vault substitution, access control, settlement replay, deadlines, pause and shared outflow accounting. Report only the observed results recorded in VALIDATION.md, together with the commit reviewed. These tests are not a substitute for independent audit.

## Why a pre-audit is useful now

The core custody and settlement paths are implemented, with a narrow review scope. We want independent scrutiny before a public beta, especially for account substitution, authority overlap, refund/payment budget interactions, time boundaries, failed transaction rollback and operational recovery. A pre-audit would help validate these design decisions and guide remediation before exposing users to real deposits.

## Ecosystem impact

The initial intended users are Solana teams making contributor or service-provider payments. Aegis aims to provide reusable escrow controls with explicit permissions and bounded outflows. Adoption, customer demand and integrations are not yet established; a pilot with one team is the next product-validation milestone.

## Launch plan

1. Reproduce local build, security regressions and demo at a recorded commit.
2. Obtain independent pre-audit, fix findings, and complete the included verification review.
3. Configure operational authorities and a private security contact; publish a devnet deployment and explorer evidence.
4. Run a small user pilot and assess whether production deployment is justified.

No fixed mainnet date, audit result, certification, or guaranteed exploit prevention is claimed.

## Owner-supplied evidence needed before submitting

- Real public repository URL and reviewed commit hash.
- Actual Colosseum Crypto World's Fair submission link and eligibility confirmation.
- Team identities, relevant experience and pilot/user evidence where available.
- Demo recording and, if deployed, verified devnet program/explorer links.
- Current bounty form, availability and deadline confirmation.
- Working private security contact.

## Tweet template — use only after actually applying

Just submitted Aegis Protocol to the @AdevarLabs Security Sidetrack on @superteamearn for @Colosseum's Crypto World's Fair Hackathon.

Excited for a security Pre-Audit of our Solana escrow and shared outflow limits.

[PROJECT_LINK]

Replace the link, verify the final post length, follow @AdevarLabs, and retain the published tweet URL. Posting and following have not been performed by this work.

## Reward description

According to the supplied bounty text: five selected teams receive a pre-audit service valued at $4,000 each, with one follow-up fix verification. This is an in-kind service, not a cash grant. Verify the live terms before applying.
