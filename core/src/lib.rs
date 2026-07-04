//! Aperture core — the L1 disclosure-workflow layer (substrate-agnostic), the L2 adapter trait,
//! and the L3 Token-2022 adapter.
//!
//! Layering:
//! ```text
//! L1  policy · package · verifier   (substrate-agnostic — the moat)
//! L2  substrate::ConfidentialSubstrate
//! L3  token2022::Token2022Substrate  (+ Arcium CSPL / ERC-7984 later)
//! ```

pub mod package;
pub mod policy;
pub mod substrate;
pub mod token2022;
pub mod verifier;

#[cfg(test)]
mod tests {
    use crate::package::{ChainAnchor, Claim, DisclosurePackage, SubstrateId, TrustModel};
    use crate::policy::{AuthzDecision, Granularity, Grant, Recipient, Trigger, Validity};
    use crate::substrate::ConfidentialSubstrate;
    use crate::token2022::{issue_range_disclosure, issue_exact_disclosure, Token2022Substrate};
    use crate::verifier::verify_package;
    use solana_zk_sdk::encryption::elgamal::ElGamalKeypair;
    use std::collections::HashSet;

    fn package_from(
        grant: &Grant,
        claim: Claim,
        proof: crate::package::ProofEnvelope,
        subject: crate::package::SubjectAccount,
    ) -> DisclosurePackage {
        let mut pkg = DisclosurePackage {
            package_id: "pkg-1".into(),
            grant_id: grant.id.clone(),
            substrate: SubstrateId::Token2022,
            issuer: "fund-A".into(),
            recipient: grant.recipient.name.clone(),
            issued_at: 1_000,
            expiry: Some(2_000),
            anchor: ChainAnchor { cluster: "mainnet-beta".into(), slot: 123_456 },
            subject: vec![subject],
            claim,
            proof,
            receipt_commitment: vec![],
            issuer_signature: vec![],
        };
        pkg.receipt_commitment = pkg.derive_receipt_commitment().to_vec();
        pkg
    }

    /// End-to-end: authorize -> issue range package -> serde round-trip -> verify.
    #[test]
    fn range_package_issue_serialize_verify() {
        let grant = Grant {
            id: "grant-lp-quarterly".into(),
            recipient: Recipient { name: "LP".into(), verifier_key: vec![] },
            granularity: Granularity::Range { min: 10_000_000, max: None },
            trigger: Trigger::OnRequest,
            validity: Validity { not_after: None, revocable: true },
        };
        let revoked = HashSet::new();
        assert_eq!(grant.authorize_new_disclosure(1_500, &revoked), AuthzDecision::Allowed);

        let fund = ElGamalKeypair::new_rand();
        let (claim, proof, subject) = issue_range_disclosure(&fund, 15_000_000, 10_000_000, "acct-1");
        let pkg = package_from(&grant, claim, proof, subject);

        // serde round-trip — the package is a portable, verifiable artifact.
        let json = serde_json::to_string(&pkg).expect("serialize");
        let back: DisclosurePackage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(pkg, back);

        let report = verify_package(&back, &Token2022Substrate, 1_500, "LP");
        assert!(report.passed(), "verify report: {:?}", report);
        assert!(report.notes.iter().any(|n| n.contains("NativeZero")));
    }

    /// Exact disclosure to a chosen third-party verifier verifies through the same package flow.
    #[test]
    fn exact_package_verifies() {
        let grant = Grant {
            id: "grant-auditor".into(),
            recipient: Recipient { name: "Auditor".into(), verifier_key: vec![] },
            granularity: Granularity::Exact,
            trigger: Trigger::Periodic,
            validity: Validity { not_after: None, revocable: true },
        };
        let fund = ElGamalKeypair::new_rand();
        let auditor = ElGamalKeypair::new_rand();
        let (claim, proof, subject) =
            issue_exact_disclosure(&fund, auditor.pubkey(), 15_000_000, "acct-1");
        let pkg = package_from(&grant, claim, proof, subject);

        let report = verify_package(&pkg, &Token2022Substrate, 1_500, "Auditor");
        assert!(report.passed(), "verify report: {:?}", report);
    }

    /// A tampered proof must fail verification.
    #[test]
    fn tampered_range_proof_fails() {
        let grant = Grant {
            id: "g".into(),
            recipient: Recipient { name: "LP".into(), verifier_key: vec![] },
            granularity: Granularity::Range { min: 10, max: None },
            trigger: Trigger::OnRequest,
            validity: Validity { not_after: None, revocable: true },
        };
        let fund = ElGamalKeypair::new_rand();
        let (claim, mut proof, subject) = issue_range_disclosure(&fund, 15_000_000, 10_000_000, "a");
        // flip bytes in the proof
        for b in proof.bytes.iter_mut().take(64) {
            *b ^= 0xff;
        }
        let pkg = package_from(&grant, claim, proof, subject);
        let report = verify_package(&pkg, &Token2022Substrate, 1_500, "LP");
        assert!(!report.proof_ok, "tampered proof must not verify: {:?}", report);
    }

    /// Trust model is surfaced from the adapter, not assumed by L1.
    #[test]
    fn adapter_reports_native_zero() {
        assert_eq!(Token2022Substrate.substrate_trust_model(), TrustModel::NativeZero);
    }

    /// The receipt commitment must bind the package contents.
    #[test]
    fn tampered_receipt_commitment_fails() {
        let grant = Grant {
            id: "g".into(),
            recipient: Recipient { name: "LP".into(), verifier_key: vec![] },
            granularity: Granularity::Range { min: 10, max: None },
            trigger: Trigger::OnRequest,
            validity: Validity { not_after: None, revocable: true },
        };
        let fund = ElGamalKeypair::new_rand();
        let (claim, proof, subject) = issue_range_disclosure(&fund, 15_000_000, 10_000_000, "a");
        let mut pkg = package_from(&grant, claim, proof, subject);
        pkg.receipt_commitment[0] ^= 0xff; // corrupt the anchor commitment
        let report = verify_package(&pkg, &Token2022Substrate, 1_500, "LP");
        assert!(!report.receipt_ok, "corrupt receipt commitment must fail: {:?}", report);
    }
}
