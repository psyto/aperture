//! L3 — the Token-2022 adapter. Wraps the primitives proven in ../spike:
//! - Range / Aggregate  -> batched u64 range proof
//! - Exact              -> ciphertext-ciphertext equality (re-encrypt under recipient key)
//! Trust model: NativeZero (on-chain ZK verification, no external committee).

use bytemuck::bytes_of;
use solana_zk_sdk::{
    encryption::{
        elgamal::ElGamalKeypair,
        pedersen::{Pedersen, PedersenOpening},
    },
    zk_elgamal_proof_program::{
        batched_range_proof::build_batched_range_proof_u64_data,
        ciphertext_ciphertext_equality::build_ciphertext_ciphertext_equality_proof_data,
        VerifyZkProof,
    },
};
use solana_zk_elgamal_proof_interface::proof_data::{
    batched_range_proof::BatchedRangeProofU64Data,
    ciphertext_ciphertext_equality::CiphertextCiphertextEqualityProofData,
};

use crate::package::{Claim, ProofEnvelope, SubjectAccount, TrustModel};
use crate::substrate::{ConfidentialSubstrate, VerifyOutcome};

pub struct Token2022Substrate;

impl ConfidentialSubstrate for Token2022Substrate {
    fn substrate_trust_model(&self) -> TrustModel {
        TrustModel::NativeZero
    }

    fn verify_disclosure(
        &self,
        claim: &Claim,
        _subject: &[SubjectAccount],
        proof: &ProofEnvelope,
    ) -> VerifyOutcome {
        match claim {
            Claim::Range { .. } | Claim::Aggregate { .. } => {
                match bytemuck::try_pod_read_unaligned::<BatchedRangeProofU64Data>(&proof.bytes) {
                    Ok(p) if p.verify_proof().is_ok() => VerifyOutcome::Valid,
                    Ok(_) => VerifyOutcome::Invalid("range proof verification failed".into()),
                    Err(_) => VerifyOutcome::Invalid("range proof bytes malformed".into()),
                }
            }
            Claim::Exact => {
                match bytemuck::try_pod_read_unaligned::<CiphertextCiphertextEqualityProofData>(
                    &proof.bytes,
                ) {
                    Ok(p) if p.verify_proof().is_ok() => VerifyOutcome::Valid,
                    Ok(_) => VerifyOutcome::Invalid("ciphertext-ciphertext equality failed".into()),
                    Err(_) => VerifyOutcome::Invalid("exact proof bytes malformed".into()),
                }
            }
        }
    }
}

/// Issuer-side helper: produce a RANGE (`balance >= threshold`) disclosure over a static balance.
/// Returns the claim, the proof envelope, and the subject account reference.
pub fn issue_range_disclosure(
    fund: &ElGamalKeypair,
    balance: u64,
    threshold: u64,
    address: &str,
) -> (Claim, ProofEnvelope, SubjectAccount) {
    // The on-chain confidential balance, and the range proof over (balance - threshold) sharing its
    // opening so the committed value is bound to the balance ciphertext by construction.
    let opening = PedersenOpening::new_rand();
    let c_bal = fund.pubkey().encrypt_with(balance, &opening);
    let delta = balance - threshold;
    let commit_delta = Pedersen::with(delta, &opening);
    let proof = build_batched_range_proof_u64_data(
        vec![&commit_delta],
        vec![delta],
        vec![64],
        vec![&opening],
    )
    .expect("range proof generation");

    let subject = SubjectAccount {
        address: address.to_string(),
        elgamal_pubkey: Vec::new(), // pubkey repr omitted in skeleton
        ciphertext_commitment: c_bal.commitment.to_bytes().to_vec(),
    };
    let envelope = ProofEnvelope {
        system_id: "t22-batched-range-u64-v1".into(),
        trust_model: TrustModel::NativeZero,
        bytes: bytes_of(&proof).to_vec(),
    };
    (Claim::Range { min: threshold, max: None }, envelope, subject)
}

/// Issuer-side helper: produce an EXACT disclosure to a chosen third-party verifier (no global
/// auditor key). The verifier decrypts the returned value out-of-band; this proof binds it.
pub fn issue_exact_disclosure(
    fund: &ElGamalKeypair,
    verifier_pubkey: &solana_zk_sdk::encryption::elgamal::ElGamalPubkey,
    balance: u64,
    address: &str,
) -> (Claim, ProofEnvelope, SubjectAccount) {
    let opening = PedersenOpening::new_rand();
    let c_bal = fund.pubkey().encrypt_with(balance, &opening);
    let v_opening = PedersenOpening::new_rand();
    let c_for_v = verifier_pubkey.encrypt_with(balance, &v_opening);
    let proof = build_ciphertext_ciphertext_equality_proof_data(
        fund,
        verifier_pubkey,
        &c_bal,
        &c_for_v,
        &v_opening,
        balance,
    )
    .expect("ciphertext-ciphertext equality generation");

    let subject = SubjectAccount {
        address: address.to_string(),
        elgamal_pubkey: Vec::new(),
        ciphertext_commitment: c_bal.commitment.to_bytes().to_vec(),
    };
    let envelope = ProofEnvelope {
        system_id: "t22-ciphertext-ciphertext-equality-v1".into(),
        trust_model: TrustModel::NativeZero,
        bytes: bytes_of(&proof).to_vec(),
    };
    (Claim::Exact, envelope, subject)
}
