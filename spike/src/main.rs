// Aperture spike — resolve engineering-unknown #1:
// Can we GENERATE + VERIFY disclosure proofs over a STATIC confidential balance,
// standalone (no transfer, no validator), using only solana-zk-sdk primitives?
//
// Maps each disclosure claim type from the L1 design to a concrete construction.

use solana_zk_sdk::{
    encryption::{
        elgamal::ElGamalKeypair,
        pedersen::{Pedersen, PedersenOpening},
    },
    zk_elgamal_proof_program::{
        batched_range_proof::build_batched_range_proof_u64_data,
        ciphertext_ciphertext_equality::build_ciphertext_ciphertext_equality_proof_data,
        ciphertext_commitment_equality::build_ciphertext_commitment_equality_proof_data,
        VerifyZkProof,
    },
};
use solana_zk_elgamal_proof_interface::{
    id as zk_proof_program_id,
    instruction::ProofInstruction,
    proof_data::batched_range_proof::{BatchedRangeProofContext, BatchedRangeProofU64Data},
};

fn main() {
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut check = |name: &str, ok: bool| {
        if ok {
            pass += 1;
            println!("  [PASS] {name}");
        } else {
            fail += 1;
            println!("  [FAIL] {name}");
        }
    };

    // ---- The "on-chain" confidential balance (what a fund's Token-2022 account holds) ----
    // available_balance_ct = ElGamal ciphertext of the balance under the fund's ElGamal key.
    let fund = ElGamalKeypair::new_rand();
    let balance: u64 = 15_000_000; // $15.000000M, 6-decimals scaled
    let bal_opening = PedersenOpening::new_rand();
    let c_bal = fund.pubkey().encrypt_with(balance, &bal_opening);

    println!("Aperture spike — static confidential-balance disclosure primitives\n");

    // =====================================================================
    // Claim A — RANGE disclosure to an LP: prove balance >= threshold, reveal nothing else.
    // Construction: commit to (balance - threshold) with the SAME opening as c_bal, so the
    // commitment is c_bal.commitment - threshold*G (bound to the real balance by construction),
    // then a u64 range proof certifies delta in [0, 2^64) i.e. balance >= threshold.
    // =====================================================================
    println!("Claim A — RANGE (balance >= threshold), value hidden:");
    let threshold: u64 = 10_000_000;
    let delta = balance - threshold;
    let commit_delta = Pedersen::with(delta, &bal_opening);
    // sanity: ciphertext commitment == Pedersen commitment of balance under same opening
    check(
        "c_bal.commitment == Pedersen(balance, opening)  (ciphertext<->commitment coherent)",
        c_bal.commitment == Pedersen::with(balance, &bal_opening),
    );
    let range_proof =
        build_batched_range_proof_u64_data(vec![&commit_delta], vec![delta], vec![64], vec![&bal_opening])
            .expect("range proof generation");
    check("range proof verifies (balance >= 10M)", range_proof.verify_proof().is_ok());

    // =====================================================================
    // Claim B — EXACT disclosure to an arbitrary third-party verifier V (auditor/LP),
    // WITHOUT the mint's global auditor key. Re-encrypt the balance under V's key and prove
    // ciphertext-ciphertext equality against the on-chain c_bal. V decrypts to learn the value.
    // =====================================================================
    println!("Claim B — EXACT to a chosen third-party verifier (no global auditor key):");
    let verifier = ElGamalKeypair::new_rand();
    let v_opening = PedersenOpening::new_rand();
    let c_for_v = verifier.pubkey().encrypt_with(balance, &v_opening);
    let cc_eq = build_ciphertext_ciphertext_equality_proof_data(
        &fund,
        verifier.pubkey(),
        &c_bal,
        &c_for_v,
        &v_opening,
        balance,
    )
    .expect("ciphertext-ciphertext equality generation");
    check("ciphertext-ciphertext equality verifies", cc_eq.verify_proof().is_ok());
    let recovered = c_for_v.decrypt_u32(verifier.secret());
    check("verifier decrypts to exact balance (15M)", recovered == Some(balance));

    // =====================================================================
    // Claim C — BIND an arbitrary Pedersen commitment to the on-chain ciphertext
    // (the primitive our range/aggregate claims lean on for soundness).
    // =====================================================================
    println!("Claim C — bind commitment <-> on-chain ciphertext:");
    let (commit_bal, open_bal) = Pedersen::new(balance);
    let cm_eq = build_ciphertext_commitment_equality_proof_data(&fund, &c_bal, &commit_bal, &open_bal, balance)
        .expect("ciphertext-commitment equality generation");
    check("ciphertext-commitment equality verifies", cm_eq.verify_proof().is_ok());

    // =====================================================================
    // Claim D — AGGREGATE across accounts via ElGamal additive homomorphism:
    // sum two account ciphertexts, then range-prove the SUM >= threshold.
    // =====================================================================
    println!("Claim D — AGGREGATE (portfolio sum >= threshold) via homomorphic add:");
    let bal1: u64 = 15_000_000;
    let bal2: u64 = 8_000_000;
    let o1 = PedersenOpening::new_rand();
    let o2 = PedersenOpening::new_rand();
    let c1 = fund.pubkey().encrypt_with(bal1, &o1);
    let c2 = fund.pubkey().encrypt_with(bal2, &o2);
    let c_sum = &c1 + &c2; // encrypts bal1 + bal2
    let sum = bal1 + bal2;
    check(
        "homomorphic sum decrypts to 23M",
        c_sum.decrypt_u32(fund.secret()) == Some(sum),
    );
    let sum_threshold: u64 = 20_000_000;
    let sum_delta = sum - sum_threshold;
    let o_sum = &o1 + &o2;
    let commit_sum_delta = Pedersen::with(sum_delta, &o_sum);
    let sum_range = build_batched_range_proof_u64_data(
        vec![&commit_sum_delta],
        vec![sum_delta],
        vec![64],
        vec![&o_sum],
    )
    .expect("aggregate range proof generation");
    check("aggregate range proof verifies (sum >= 20M)", sum_range.verify_proof().is_ok());

    // =====================================================================
    // On-chain plumbing — build the ACTUAL ZK ElGamal Proof Program instruction
    // from the range proof, then replicate the native program's processor path
    // offline: read discriminator -> decode proof_data -> verify_proof().
    // The program runs this exact verify logic (same solana-zk-sdk code); the
    // only residual is ops-level: the program is feature-gated OFF on mainnet
    // since the June-2025 incident, pending reactivation.
    // =====================================================================
    println!("On-chain plumbing — real VerifyBatchedRangeProofU64 instruction:");
    let ix = ProofInstruction::VerifyBatchedRangeProofU64
        .encode_verify_proof::<BatchedRangeProofU64Data, BatchedRangeProofContext>(None, &range_proof);
    check(
        "instruction targets the ZK ElGamal Proof program id",
        ix.program_id == zk_proof_program_id(),
    );
    check(
        "proof-in-instruction-data mode (no context-state accounts)",
        ix.accounts.is_empty(),
    );
    check(
        "discriminator decodes to VerifyBatchedRangeProofU64",
        matches!(
            ProofInstruction::instruction_type(&ix.data),
            Some(ProofInstruction::VerifyBatchedRangeProofU64)
        ),
    );
    // Replicate the native processor: decode the proof from instruction data and verify it.
    let decoded =
        ProofInstruction::proof_data::<BatchedRangeProofU64Data, BatchedRangeProofContext>(&ix.data);
    check(
        "program processor path: decode from ix.data then verify_proof() passes",
        decoded.map(|p| p.verify_proof().is_ok()) == Some(true),
    );

    // =====================================================================
    // Soundness negatives — a lie must not produce a passing proof.
    // =====================================================================
    println!("Soundness — lies must fail:");
    // N1: claim an inflated exact value -> builder rejects (decrypt != claimed amount).
    let lie = build_ciphertext_commitment_equality_proof_data(&fund, &c_bal, &commit_bal, &open_bal, 99_000_000);
    check("inflated exact-value claim is rejected at generation", lie.is_err());

    // N2: range proof whose committed value != claimed amount must fail verification.
    let mismatched = build_batched_range_proof_u64_data(
        vec![&commit_delta], // commits to `delta`
        vec![delta + 1],     // but claims delta+1
        vec![64],
        vec![&bal_opening],
    );
    let n2_fails = match mismatched {
        Err(_) => true,
        Ok(p) => p.verify_proof().is_err(),
    };
    check("range proof with mismatched amount fails", n2_fails);

    println!("\n==== {pass} passed, {fail} failed ====");
    std::process::exit(if fail == 0 { 0 } else { 1 });
}
