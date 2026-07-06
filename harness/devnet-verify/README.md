# devnet-verify

Submits an aperture-generated range proof to the **reactivated** ZK ElGamal Proof Program
(`ZkE1Gama1Proof11111111111111111111111111111`) on Solana **devnet**, proving the live program
accepts our proof on-chain — the successor to `spike/`'s offline processor replication.

## Status (2026-07-06)

- The ZK ElGamal Proof Program was **reactivated on mainnet + devnet ~late June 2026** (audit
  complete). Confidential transfers are executable again.
- `cargo run` builds a valid range proof, constructs the real `VerifyBatchedRangeProofU64`
  instruction, and emits an unsigned transaction (base64).
- `simulate.sh` posts it to devnet `simulateTransaction` (`sigVerify=false`,
  `replaceRecentBlockhash=true`).
- **Reached the live devnet runtime (apiVersion 4.1.0):** the transaction is well-formed and the ZK
  program id is recognized. The only remaining step is a **funded fee payer** — the simulator loads
  the payer account, and the devnet faucet was rate-limited from this environment, so it returned
  `err: AccountNotFound` (payer issue, *not* a program/proof issue: `unitsConsumed: 0`, empty logs).

## Finish it (30 seconds, from any un-rate-limited connection)

```
# fund the throwaway payer (or use your own funded devnet pubkey)
solana airdrop 1 $(solana-keygen pubkey ../.devnet-payer.json) -u devnet

# then simulate
./simulate.sh
# expect:  err: None   units: <non-zero>   logs: [ ... Verify... ]
```

`err: None` with non-zero compute units = the live ZK program verified the aperture proof on-chain.
