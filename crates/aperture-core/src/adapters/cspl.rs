//! L3 stub adapter for Arcium **CSPL** (Confidential SPL). Its only purpose is to demonstrate that
//! L1 (policy / package / verifier) is genuinely substrate-agnostic: the exact same flow runs
//! unchanged, and only the trust model and proof format differ.
//!
//! The MPC proof verification is a **PLACEHOLDER** — Arcium exposes no verify-in-Rust path here — so
//! this stands in to prove *composition*, not to verify real CSPL proofs. A real adapter would
//! validate an Arcium MXE attestation / MPC proof in `verify_disclosure`. Note the trust model is
//! `MpcHonestMajority`: unlike Token-2022's `NativeZero`, a CSPL claim rests on an external
//! committee — which is exactly the distinction Aperture surfaces to an auditor.

use crate::package::{Claim, ProofEnvelope, SubjectAccount, TrustModel};
use crate::substrate::{ConfidentialSubstrate, VerifyOutcome};

/// Placeholder attestation a stub CSPL proof carries (stands in for a real MPC proof).
pub const CSPL_STUB_ATTESTATION: &[u8] = b"arcium-cspl-stub-attestation-v0";

pub struct CsplSubstrate;

impl ConfidentialSubstrate for CsplSubstrate {
    fn substrate_trust_model(&self) -> TrustModel {
        TrustModel::MpcHonestMajority
    }

    fn verify_disclosure(
        &self,
        _claim: &Claim,
        _subject: &[SubjectAccount],
        proof: &ProofEnvelope,
    ) -> VerifyOutcome {
        // PLACEHOLDER — a real impl verifies an Arcium MXE attestation / MPC proof.
        if proof.system_id.starts_with("cspl-") && proof.bytes == CSPL_STUB_ATTESTATION {
            VerifyOutcome::Valid
        } else {
            VerifyOutcome::Invalid("CSPL proof placeholder mismatch (real MPC verify not wired)".into())
        }
    }
}

/// Issuer-side stub: a placeholder CSPL range disclosure, shaped exactly like a real one so the L1
/// package/verifier path is identical to Token-2022's.
pub fn issue_range_disclosure_stub(min: u64) -> (Claim, ProofEnvelope, SubjectAccount) {
    let claim = Claim::Range { min, max: None };
    let proof = ProofEnvelope {
        system_id: "cspl-range-stub-v0".into(),
        trust_model: TrustModel::MpcHonestMajority,
        bytes: CSPL_STUB_ATTESTATION.to_vec(),
    };
    let subject = SubjectAccount {
        address: "cspl-acct-1".into(),
        elgamal_pubkey: Vec::new(),
        ciphertext_commitment: vec![0x11; 32],
    };
    (claim, proof, subject)
}
