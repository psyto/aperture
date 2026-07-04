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
  (Token-2022 hides amounts, not the transaction graph). Channel: fund administrators.
- **Trust model as a feature:** Token-2022 is the only *zero-external-trust* substrate (native,
  on-chain ZK verification). Arcium (MPC committee) and ERC-7984 (FHE threshold committee) carry
  external trust. A disclosure package can carry a `trust_model` attestation so an auditor knows
  what assumption a claim rests on.

## Architecture (target)

```
L1  Disclosure workflow layer (substrate-agnostic — the moat)
      Policy / DisclosurePackage / Receipt Registry / Verifier
L2  Substrate Adapter interface
L3  Adapters:  [Token-2022] first   ·   [Arcium CSPL] [ERC-7984] later stubs
```

## Status (scoped — not a single color)

| Dimension | Status |
|-----------|--------|
| Disclosure-claim primitives, **off-chain** generate + verify | **GREEN** — proven in `spike/` |
| **On-chain** proof verification / non-repudiation | **Plumbing GREEN, availability RED.** `spike/` builds the real `VerifyBatchedRangeProofU64` instruction and replicates the native program's processor path (decode → `verify_proof`) offline — the program runs this exact code. Residual is purely ops: the program is feature-gated *off* on mainnet-beta since the June-2025 forged-proof incident (Fiat-Shamir transcript flaw; patched Agave ≥v2.1.21, reactivation not yet confirmed live). Degrades gracefully (see below). |
| Revocation semantics | **Open spec gap** — exact disclosure is irreversible once delivered (see Design gaps) |
| Product / market / UX | **Early** — never claimed otherwise |

The **narrow** first risk was: *can we generate + verify disclosure proofs over a **static**
confidential balance, standalone (no transfer, no validator), using stock `solana-zk-sdk`?*

`spike/` answers **yes**. Against real `solana-zk-sdk` 7.0.1, purely locally, it constructs and
verifies every disclosure claim type the L1 layer needs:

| Claim | Construction | Result |
|-------|--------------|--------|
| **Range** (`balance ≥ threshold`, value hidden) | commit `(balance − threshold)` under the balance opening → `build_batched_range_proof_u64_data` | ✅ |
| **Exact** to an arbitrary third-party verifier (no global auditor key) | re-encrypt under verifier key → `ciphertext_ciphertext_equality` → verifier decrypts | ✅ |
| **Commitment ↔ ciphertext binding** | `ciphertext_commitment_equality` | ✅ |
| **Aggregate** (portfolio sum ≥ threshold) | ElGamal additive homomorphism, then range proof on the sum | ✅ |
| **On-chain plumbing** | build real `ProofInstruction::VerifyBatchedRangeProofU64` (proof-in-instruction-data) + replicate native processor: decode → `verify_proof` | ✅ |
| Soundness: inflated exact claim | rejected at generation | ✅ |
| Soundness: range with mismatched amount | fails verification | ✅ |

Key finding: the `build_*_data` functions are **pure and take arbitrary inputs** — they are the
standalone generation API. No need to drop below `solana-zk-sdk`; the earlier concern that
generation was only wired for transfers is refuted.

### Run

```
cd spike && cargo run
# ==== 13 passed, 0 failed ====
```

## Design gaps (open)

- **Revocation is not clawback.** Exact disclosure re-encrypts the value under the recipient's
  key; once delivered it *cannot* be un-decrypted. What is revocable is the **standing
  authorization to obtain future disclosures**, not data already handed over. Design principle:
  prefer **range / predicate** disclosures and **just-in-time** proofs; treat any exact disclosure
  as permanent to that recipient (like a signed bank statement). Escrow does not fix this.
- **On-chain path is availability-gated, not compute-gated.** Proof *verification* runs in a
  **native** program, not BPF — so circuit size / compute budget is a non-issue by design, and
  proof-vs-tx-size is handled by context state accounts. The real dependency is binary: is the
  native verifier enabled on mainnet. Until it is, ship **off-chain verification** (works today)
  plus **commitment anchoring** for the audit trail — the Receipt Registry stores only hashes and
  needs no ZK program, so non-repudiation of *what was disclosed* does not depend on reactivation.

## Open (not yet closed)

- ZK ElGamal Proof Program mainnet reactivation is the sole on-chain blocker (ops/timeline, not
  code). Instruction plumbing is done; submitting to a live cluster only needs the program enabled.
- Quantify client-side proof-generation cost (WASM-in-browser vs. desktop) — mild for a
  periodic/on-request disclosure product (snapshot read, not a hot transfer path), but unmeasured.
- `decrypt_u32` covers values up to ~2³²; larger balances need the lo/hi chunked decrypt
  (a solved Token-2022 pattern).
- Portfolio decision: Aperture as an Intentio sibling vs. a standalone brand — undecided.
