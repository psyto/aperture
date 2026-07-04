# Aperture (working name)

A **selective-disclosure workflow layer** for confidential on-chain balances.

The confidential-balance *primitive* is commoditizing fast (Solana Token-2022 Confidential
Balances, Arcium CSPL, EVM ERC-7984). All three stop at a **crude global-auditor model**: one
designated key that can decrypt everything, forever. Real funds and DAO treasuries need the layer
above that: **who** may see, at **what granularity**, **when**, and the ability to **revoke** — plus
a verifiable, non-repudiable record of what was disclosed to whom.

Aperture is that layer. Differentiation is not cryptography — it is **disclosure semantics + an
auditor-facing verifier + a fund-admin workflow**, sitting on top of any confidential substrate.

## Positioning

- **ICP:** crypto-native funds / prop desks whose secret is the *amount*, not the *counterparty*
  (Token-2022 hides amounts, not the transaction graph). Channel: **fund administrators**. See
  [`GTM_DESIGN_PARTNER.md`](./GTM_DESIGN_PARTNER.md) for the design-partner brief.
- **Trust model as a feature:** Token-2022 is the only *zero-external-trust* substrate (native,
  on-chain ZK verification). Arcium (MPC committee) and ERC-7984 (FHE threshold committee) carry
  external trust. Every disclosure package carries a `trust_model` attestation so an auditor knows
  what assumption a claim rests on — a distinction the substrates themselves don't surface.
- **Dominant risk is market existence, not the tech — and desk research says the market is
  pre-formation.** The substrate is non-transactable on mainnet today (ZK program disabled),
  confidential-balance-aware custody doesn't exist yet, and the visible institutional SOL holders are
  disclosure-mandated (the anti-use-case). See [`MARKET_SIGNAL.md`](./MARKET_SIGNAL.md). Recommended
  posture: **hold at option value; watch two triggers** (ZK reactivation; confidential-balance
  support in Fireblocks/Squads) rather than run heavy GTM now.

## Architecture

```
L1  Disclosure workflow layer (substrate-agnostic — the moat)
      Policy / DisclosurePackage / Receipt Registry / Verifier
L2  Substrate Adapter interface  (ConfidentialSubstrate)
L3  Adapters:  [Token-2022] real   ·   [Arcium CSPL] stub   ·   [ERC-7984] later
```

L1 is substrate-agnostic — it's the moat. Everything substrate-specific (proof math, ciphertext
binding, trust model) is confined below the L2 trait, so a competitor's primitive (Arcium CSPL,
Zama ERC-7984) becomes a swappable *substrate*, not a competitor.

## Repo layout

| Crate | What | Tests |
|-------|------|-------|
| `spike/` | Feasibility proof: static-balance disclosure primitives against real `solana-zk-sdk` 7.0.1, standalone (no transfer, no validator), + real on-chain instruction plumbing replicated offline | 13 |
| `core/` | L1 layer — `policy` (revocation semantics), `package` (serde + `trust_model` + `derive_receipt_commitment`), `substrate` (L2 trait), `token2022` (L3, real), `cspl` (L3, stub), `verifier` | 12 |
| `receipts/` | Receipt Registry — content-blind **native Solana program** (no Anchor); anchors commitments, no ZK program needed | — |
| `receipts-tests/` | LiteSVM harness (real BPF, no validator) for `receipts/` | 4 |
| `flow-tests/` | End-to-end: core issue → off-chain verify → on-chain anchor → revoke | 1 |

### Build & run

```
cd spike            && cargo run     # ==== 13 passed, 0 failed ====
cd core             && cargo test    # 12 passed
cd receipts         && cargo build-sbf     # produces target/deploy/aperture_receipts.so
cd receipts-tests   && cargo test    # 4 passed  (needs the .so above)
cd flow-tests       && cargo test    # 1 passed  (needs the .so above)
```

## Status (scoped — not a single color)

| Dimension | Status |
|-----------|--------|
| Disclosure-claim primitives, **off-chain** generate + verify | **GREEN** — proven in `spike/` (range / exact-to-third-party / aggregate / commitment binding) |
| **On-chain** proof verification | **Plumbing GREEN, availability RED.** `spike/` builds the real `VerifyBatchedRangeProofU64` instruction and replicates the native processor path offline. Residual is ops: the ZK ElGamal Proof Program is feature-gated *off* on mainnet-beta since the June-2025 forged-proof incident (patched Agave ≥v2.1.21, reactivation not yet confirmed live). |
| **On-chain** anchoring / non-repudiation | **GREEN, live** — `receipts/` (content-blind, needs no ZK program), LiteSVM-tested; independent of the ZK reactivation. |
| Revocation semantics | **Encoded** — revocation ≠ clawback made explicit in `core::policy` and demonstrated in `flow-tests/`. |
| Product / market / UX | **Early** — market-existence is the dominant open risk (see Positioning). |

## What `spike/` proves

The narrow first risk was: *can we generate + verify disclosure proofs over a **static** confidential
balance, standalone (no transfer, no validator), using stock `solana-zk-sdk`?* Answer: **yes**.

| Claim | Construction | Result |
|-------|--------------|--------|
| **Range** (`balance ≥ threshold`, value hidden) | commit `(balance − threshold)` under the balance opening → `build_batched_range_proof_u64_data` | ✅ |
| **Exact** to an arbitrary third-party verifier (no global auditor key) | re-encrypt under verifier key → `ciphertext_ciphertext_equality` → verifier decrypts | ✅ |
| **Commitment ↔ ciphertext binding** | `ciphertext_commitment_equality` | ✅ |
| **Aggregate** (portfolio sum ≥ threshold) | ElGamal additive homomorphism, then range proof on the sum | ✅ |
| **On-chain plumbing** | build real `ProofInstruction::VerifyBatchedRangeProofU64` + replicate native processor: decode → `verify_proof` | ✅ |
| Soundness: inflated exact claim | rejected at generation | ✅ |
| Soundness: range with mismatched amount | fails verification | ✅ |

Key finding: the `build_*_data` functions are **pure and take arbitrary inputs** — they *are* the
standalone generation API. No need to drop below `solana-zk-sdk`; the earlier concern that generation
was only wired for transfers is refuted.

## Design notes

- **Revocation is not clawback.** Exact disclosure re-encrypts the value under the recipient's key;
  once delivered it *cannot* be un-decrypted. What is revocable is the **standing authorization to
  obtain future disclosures**, not data already handed over. Principle: prefer **range / predicate**
  disclosures and **just-in-time** proofs; treat any exact disclosure as permanent to that recipient
  (like a signed bank statement). Escrow does not fix this. Encoded in `core::policy`
  (`Granularity::reversibility()` marks `Exact` as `Irreversible`; `authorize_new_disclosure()`
  gates only future disclosures; `revocation_note()` surfaces the truth) and demonstrated in
  `flow-tests/` (post-revoke, the delivered package still verifies).
- **On-chain path is availability-gated, not compute-gated.** Proof *verification* runs in a
  **native** program, not BPF — so circuit size / compute budget is a non-issue by design, and
  proof-vs-tx-size is handled by context state accounts. The real dependency is binary: is the native
  verifier enabled on mainnet. Until it is, ship **off-chain verification** (works today) plus
  **commitment anchoring** (`receipts/`, live) — so non-repudiation of *what was disclosed* does not
  depend on reactivation.

## Open

- **Portfolio (recommended, pending ratification):** *don't* fold Aperture into Intentio (that would
  dilute Intentio's Reth/Revm/Alloy narrative — Aperture shares its *buyer*, not its *tech*), and
  *don't* build a standalone brand yet (scope-spread). Middle path: a **Fabrknt-umbrella standalone
  product** named `aperture`, unified at the **buyer layer** (crypto-native funds/treasuries), with
  brand investment deferred until one design partner validates.
- **GTM (the critical path):** desk research finds the market pre-formation
  ([`MARKET_SIGNAL.md`](./MARKET_SIGNAL.md)) — so **watch two unlock triggers** (ZK reactivation;
  confidential-balance custody support) and keep the built stack warm, rather than run a heavy
  design-partner campaign into a market that can't yet transact. Leading indicator: Arcium CSPL
  institutional traction (Aperture can ride it via the `cspl` adapter).
- ZK ElGamal Proof Program mainnet reactivation — sole on-chain blocker (ops, not code); plumbing done.
- Quantify client-side proof-generation cost (WASM-in-browser vs desktop) — mild for a
  periodic/on-request product (snapshot read, not a hot transfer path), but unmeasured.
- `decrypt_u32` covers values up to ~2³²; larger balances need the lo/hi chunked decrypt (a solved
  Token-2022 pattern).
- `issuer_signature` in the package is a stub; wiring real signing is breadth, deferred until after
  buyer validation.

---

*Private — `psyto/aperture`. Working name; `aperture` is committed but trivially renameable.*
