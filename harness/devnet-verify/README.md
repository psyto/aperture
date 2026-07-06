# devnet-verify

Submits an aperture-generated range proof to the **reactivated** ZK ElGamal Proof Program
(`ZkE1Gama1Proof11111111111111111111111111111`) on Solana **devnet**, proving the live program
accepts our proof on-chain — the successor to `spike/`'s offline processor replication.

## Status (2026-07-06): CONFIRMED ✅

The ZK ElGamal Proof Program was **reactivated on mainnet + devnet ~late June 2026** (audit
complete), and an aperture range proof was **verified on-chain by the live devnet program**:

```
$ ./simulate.sh <funded-devnet-pubkey>
err  : None
units: 111000
logs :
   Program ZkE1Gama1Proof11111111111111111111111111111 invoke [1]
   VerifyBatchedRangeProofU64
   Program ZkE1Gama1Proof11111111111111111111111111111 success
```

`err: None` + ~111k compute units + a `VerifyBatchedRangeProofU64` → `success` log = the live,
reactivated program accepted the aperture proof on-chain. This is the on-chain confirmation of what
`spike/` replicated offline: aperture's disclosure proofs verify against the real ZK ElGamal Proof
Program.

## Run it yourself

```
# any funded (or merely existing) devnet pubkey works — simulation charges no fee and needs no sig
./simulate.sh <your-devnet-pubkey>
```
