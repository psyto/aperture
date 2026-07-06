# Aperture

**A substrate-agnostic selective-disclosure engine for confidential on-chain balances.**
Apache-2.0. Designed to be consumed as an external dependency by downstream products.

Confidential-balance primitives are commoditizing (Solana Token-2022 Confidential Balances, Arcium
CSPL, EVM ERC-7984). All of them ship only a **crude global-auditor model**: one key that decrypts
everything, forever. Aperture is the layer above that — **who** may see, at **what granularity**,
**when**, and the ability to **revoke** — plus a verifiable disclosure package, a content-blind
on-chain receipt, and a `trust_model` attestation so a verifier knows what assumption a claim rests
on. Differentiation is disclosure *semantics*, not cryptography.

## Architecture

```
L1  Disclosure workflow (substrate-agnostic — the engine)
      policy · package · verifier · audit · receipt registry
L2  Substrate Adapter interface  (ConfidentialSubstrate)
L3  Adapters:  [Token-2022] real   ·   [Arcium CSPL] stub   ·   [ERC-7984] later
```

Everything substrate-specific (proof math, ciphertext binding, trust model) is confined below the L2
trait, so a competing primitive becomes a swappable *substrate*, not a competitor. The same
`aperture_core::token2022` and `aperture_core::cspl` paths are re-exported from `adapters/`.

## Repo layout

| Path | What | Tests |
|------|------|-------|
| `crates/aperture-core/` | The engine (published lib): `policy` (revocation semantics), `package` (serde + `trust_model` + `derive_receipt_commitment`), `substrate` (L2 trait), `verifier`, `audit` (auditor decrypts confidential txns → audit trail + balanced double-entry journal; lo/hi split for full-`u64` amounts; **real Token-2022 confidential-transfer auditor format** — grouped ElGamal under [source,dest,auditor], 16/32 split; CSV / JSON export), `adapters/{token2022,cspl}` | 20 |
| `programs/aperture-receipts/` | Receipt Registry — content-blind **native Solana program** (no Anchor); anchors commitments, needs no ZK program | — |
| `harness/spike/` | Feasibility proof vs real `solana-zk-sdk` 7.0.1 (standalone, no validator) + on-chain instruction plumbing replicated offline | 13 |
| `harness/receipts-tests/` | LiteSVM harness (real BPF, no validator) for the program | 4 |
| `harness/flow-tests/` | End-to-end: issue → off-chain verify → on-chain anchor → revoke | 1 |

The workspace (root `Cargo.toml`) contains only the publishable lib; `programs/` and `harness/`
are excluded standalone crates (they pull conflicting solana crate versions and build on their own).

## Using it as a dependency (the boundary)

Downstream products consume the engine at arm's length — a versioned external dependency, never a
path/monorepo entanglement:

```toml
# in a consumer's Cargo.toml
aperture-core = { git = "https://github.com/psyto/aperture", tag = "v0.1.0" }
# or, once published:  aperture-core = "0.1"
```

The consumer implements the account-control / compliance / integration layer on top; Aperture stays
the disclosure engine. `programs/aperture-receipts` is deployed on-chain and referenced by the
consumer for non-repudiation.

## Build & run

```
cargo test                                             # engine (crates/aperture-core) — 12
cd programs/aperture-receipts && cargo build-sbf       # -> target/deploy/aperture_receipts.so
cd harness/receipts-tests && cargo test                # 4   (needs the .so above)
cd harness/flow-tests     && cargo test                # 1   (needs the .so above)
cd harness/spike          && cargo run                 # 13
```

## What `harness/spike/` proves

The first risk was: *can we generate + verify disclosure proofs over a **static** confidential
balance, standalone (no transfer, no validator), using stock `solana-zk-sdk`?* Answer: **yes**.

| Claim | Construction | Result |
|-------|--------------|--------|
| **Range** (`balance ≥ threshold`, value hidden) | commit `(balance − threshold)` under the balance opening → `build_batched_range_proof_u64_data` | ✅ |
| **Exact** to an arbitrary third-party verifier (no global auditor key) | re-encrypt under verifier key → `ciphertext_ciphertext_equality` → verifier decrypts | ✅ |
| **Commitment ↔ ciphertext binding** | `ciphertext_commitment_equality` | ✅ |
| **Aggregate** (portfolio sum ≥ threshold) | ElGamal additive homomorphism, then range proof on the sum | ✅ |
| **On-chain plumbing** | build real `ProofInstruction::VerifyBatchedRangeProofU64` + replicate native processor: decode → `verify_proof` | ✅ |
| Soundness: inflated exact claim / mismatched range | rejected at generation / fails verification | ✅ |

Key finding: the `build_*_data` functions are **pure and take arbitrary inputs** — they *are* the
standalone generation API; no need to drop below `solana-zk-sdk`.

## Design notes

- **Revocation is not clawback.** Exact disclosure re-encrypts the value under the recipient's key;
  once delivered it cannot be un-decrypted. What is revocable is the standing authorization to obtain
  *future* disclosures. Encoded in `policy` (`Granularity::reversibility()` marks `Exact` as
  `Irreversible`; `authorize_new_disclosure()` gates only future disclosures) and demonstrated in
  `harness/flow-tests` (post-revoke, the delivered package still verifies). Prefer range/predicate +
  just-in-time proofs.
- **On-chain path is availability-gated, not compute-gated.** Proof *verification* runs in a native
  program, not BPF — so compute budget is a non-issue by design, and proof-vs-tx-size is handled by
  context state accounts. The real dependency is binary: the ZK ElGamal Proof Program is feature-
  gated *off* on mainnet-beta since the June-2025 forged-proof incident (patched, reactivation not
  yet confirmed live). Until then: off-chain verification works today, and `aperture-receipts` anchors
  commitments with **no ZK program**, so the non-repudiation trail is live independent of that gate.

## Status

| Dimension | Status |
|-----------|--------|
| Disclosure primitives, off-chain generate + verify | GREEN — `harness/spike` |
| On-chain proof verification | Plumbing GREEN, availability RED (ZK program disabled on mainnet) |
| On-chain anchoring / non-repudiation | GREEN, live — `programs/aperture-receipts` |
| Revocation semantics | Encoded (revocation ≠ clawback) |

---

*Private — `psyto/aperture`. Positioning, market analysis, and GTM are tracked privately, outside
this engine repo. Aperture is the reusable disclosure engine; downstream products consume it as a
licensed dependency.*
