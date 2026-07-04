# Aperture — design-partner brief

*Working name. One-pager for a first conversation with a crypto-native fund / prop desk and its
fund administrator. Not a pitch to close — a brief to test the core bet cheaply.*

## The one line

**Use Solana confidential balances without becoming unexplainable to your auditor, LPs, or
regulators.**

## The problem

Solana Token-2022 confidential balances hide your position and treasury size on-chain — but they
ship with only a crude **global auditor key**: one key that decrypts *everything, forever*. For a
regulated fund that's unusable:

- you can't give your auditor a full view while showing an LP only "balance ≥ $X",
- you can't scope disclosure per counterparty or per reporting period,
- you can't revoke a standing grant.

So today funds either stay fully transparent (leaking position size to competitors and copy-traders)
or stay on a CEX (opaque, but you give up self-custody).

## What Aperture is

A **selective-disclosure layer** on top of confidential balances. Amounts stay hidden on-chain; you
disclose — cryptographically, at the granularity *you* choose, to whom *you* choose:

- **Exact** value to your auditor.
- **Range** ("balance ≥ threshold") to an LP — revealing nothing else.
- **Aggregate** (sum across accounts) for proof-of-solvency without per-account exposure.

Each disclosure produces a **verifiable package** your auditor/LP checks with a tool, plus a
**content-blind on-chain receipt** (a non-repudiable record of *what was disclosed to whom, when*).
No trusted third party ever holds your keys or your data — verification runs against Solana's native
proofs.

## What we can show you today (working, not slides)

1. A fund account holding a confidential balance (amount hidden on-chain).
2. Issue disclosures: **exact** to "auditor", **range ≥ $10M** to "LP".
3. Auditor/LP runs the verifier → cryptographic **PASS**, learning only what the policy allows.
4. The commitment is anchored on-chain (Receipt Registry) → tamper-evident audit trail.
5. Revoke the standing grant → the chain shows it revoked, while already-delivered disclosures stay
   valid. We're explicit about this: **revocation controls future disclosures, not clawback of data
   already handed over** — so prefer range/predicate + just-in-time proofs over exact where you can.

## Honest status

- **Live today:** off-chain verification + on-chain commitment anchoring (the full auditor/LP
  disclosure workflow above).
- **Pending:** on-chain *proof verification* (so a smart contract could gate on a disclosure) waits
  on Solana's ZK ElGamal Proof Program being re-enabled on mainnet (feature-gated off after a
  June-2025 fix). This does **not** affect the auditor/LP workflow.

## What we're actually testing with you

1. **Do you plan to hold Solana Token-2022 confidential balances?** If not — what would change that?
   *(This is the question that matters most. The market has to exist before the tooling does.)*
2. Is "verified selective disclosure + on-chain receipt" the artifact your **auditor / fund admin /
   LPs** would actually accept?
3. Which disclosures matter most to you: exact-to-auditor, range-to-LP, or aggregate solvency?
4. Would your **fund administrator** be the right place to integrate this?

## The ask

30–45 minutes. Optionally: point us at one confidential-balance account on devnet/testnet and let
your auditor or fund admin run the verifier. No integration, no commitment.
