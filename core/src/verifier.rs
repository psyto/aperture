//! The uniform Verifier — substrate-agnostic. It performs the checks that don't change across
//! substrates (identity, expiry, trust-model surfacing, receipt), and delegates the cryptographic
//! step to the L2 adapter via `verify_disclosure`.

use crate::package::DisclosurePackage;
use crate::substrate::{ConfidentialSubstrate, VerifyOutcome};

#[derive(Debug)]
pub struct VerifyReport {
    pub structural_ok: bool,
    pub proof_ok: bool,
    pub receipt_ok: bool,
    pub notes: Vec<String>,
}

impl VerifyReport {
    pub fn passed(&self) -> bool {
        self.structural_ok && self.proof_ok && self.receipt_ok
    }
}

/// Verify a disclosure package on behalf of `recipient_id` at time `now`.
pub fn verify_package(
    pkg: &DisclosurePackage,
    substrate: &dyn ConfidentialSubstrate,
    now: i64,
    recipient_id: &str,
) -> VerifyReport {
    let mut notes = Vec::new();

    // 1. Structural / identity.
    let mut structural_ok = true;
    if pkg.recipient != recipient_id {
        structural_ok = false;
        notes.push(format!("recipient mismatch: package for '{}'", pkg.recipient));
    }
    if let Some(exp) = pkg.expiry {
        if now > exp {
            structural_ok = false;
            notes.push("package expired".into());
        }
    }

    // 2. Surface the trust assumption this claim rests on (the differentiator).
    notes.push(format!("trust_model = {:?}", pkg.proof.trust_model));

    // 3. Cryptographic verification, delegated to the substrate adapter.
    let outcome = substrate.verify_disclosure(&pkg.claim, &pkg.subject, &pkg.proof);
    let proof_ok = matches!(outcome, VerifyOutcome::Valid);
    if let VerifyOutcome::Invalid(m) = &outcome {
        notes.push(format!("proof invalid: {m}"));
    }

    // 4. Receipt: the on-chain commitment is content-blind and needs no ZK program. Here we only
    //    check the package carries a non-empty commitment; on-chain lookup / revocation status is a
    //    Receipt Registry call (out of skeleton scope).
    let receipt_ok = !pkg.receipt_commitment.is_empty();
    if !receipt_ok {
        notes.push("missing receipt commitment".into());
    }

    VerifyReport { structural_ok, proof_ok, receipt_ok, notes }
}
