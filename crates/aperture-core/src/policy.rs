//! Disclosure policy — the L1 semantics that none of the confidential-token substrates provide.
//!
//! The sharpest correction baked in here: **revocation is not clawback.** An exact disclosure
//! hands the recipient a value they can decrypt forever; you cannot un-disclose it. So a `Grant`
//! is a *standing authorization to obtain FUTURE disclosures* — revoking it stops future
//! disclosures, never data already delivered. The type system surfaces this rather than pretending
//! a delivered plaintext can be recalled.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub type UnixTime = i64;

/// A party allowed to receive disclosures (auditor, LP, regulator, fund admin).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Recipient {
    pub name: String,
    /// Verification key. For `Exact`, the recipient's ElGamal pubkey (bytes); otherwise an
    /// identity key used only to address the package.
    pub verifier_key: Vec<u8>,
}

/// How much a disclosure reveals.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Granularity {
    /// Reveals the exact value to the recipient. IRREVERSIBLE once delivered.
    Exact,
    /// Reveals only `value >= min` (and optionally `value <= max`). No plaintext leaves.
    Range { min: u64, max: Option<u64> },
    /// Reveals only an aggregate predicate over several accounts. No per-account plaintext leaves.
    Aggregate { min: u64, max: Option<u64> },
}

/// Whether a disclosure at a given granularity can ever be "taken back".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reversibility {
    /// Plaintext delivered; cannot be un-disclosed. Revocation only stops FUTURE grants.
    Irreversible,
    /// No plaintext delivered; the standing grant fully governs exposure.
    PredicateOnly,
}

impl Granularity {
    pub fn reversibility(&self) -> Reversibility {
        match self {
            Granularity::Exact => Reversibility::Irreversible,
            Granularity::Range { .. } | Granularity::Aggregate { .. } => Reversibility::PredicateOnly,
        }
    }
}

/// When a disclosure may be produced under a grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trigger {
    Periodic,
    OnRequest,
    OnThreshold,
}

/// Standing validity window for a grant.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validity {
    pub not_after: Option<UnixTime>,
    pub revocable: bool,
}

/// A STANDING authorization to obtain future disclosures. This — not delivered data — is the thing
/// that can be revoked.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Grant {
    pub id: String,
    pub recipient: Recipient,
    pub granularity: Granularity,
    pub trigger: Trigger,
    pub validity: Validity,
}

/// Whether a NEW disclosure under a grant is permitted right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthzDecision {
    Allowed,
    Expired,
    Revoked,
}

impl Grant {
    /// Decide whether a NEW disclosure under this grant is permitted at `now`, given the set of
    /// revoked grant ids. Revocation and expiry gate only *future* disclosures.
    pub fn authorize_new_disclosure(&self, now: UnixTime, revoked: &HashSet<String>) -> AuthzDecision {
        if self.validity.revocable && revoked.contains(&self.id) {
            return AuthzDecision::Revoked;
        }
        if let Some(exp) = self.validity.not_after {
            if now > exp {
                return AuthzDecision::Expired;
            }
        }
        AuthzDecision::Allowed
    }

    /// Surface the clawback truth to callers instead of hiding it.
    pub fn revocation_note(&self) -> &'static str {
        match self.granularity.reversibility() {
            Reversibility::Irreversible => {
                "revocation stops FUTURE exact disclosures only; a value already delivered \
                 cannot be recalled — prefer range/predicate + just-in-time proofs"
            }
            Reversibility::PredicateOnly => {
                "predicate-only: no plaintext was ever delivered, so the standing grant fully \
                 controls exposure"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(id: &str, g: Granularity, revocable: bool, not_after: Option<UnixTime>) -> Grant {
        Grant {
            id: id.into(),
            recipient: Recipient { name: "LP".into(), verifier_key: vec![] },
            granularity: g,
            trigger: Trigger::OnRequest,
            validity: Validity { not_after, revocable },
        }
    }

    #[test]
    fn revoked_grant_blocks_future_disclosures() {
        let g = grant("g1", Granularity::Range { min: 10, max: None }, true, None);
        let mut revoked = HashSet::new();
        assert_eq!(g.authorize_new_disclosure(100, &revoked), AuthzDecision::Allowed);
        revoked.insert("g1".to_string());
        assert_eq!(g.authorize_new_disclosure(100, &revoked), AuthzDecision::Revoked);
    }

    #[test]
    fn non_revocable_grant_ignores_revocation() {
        let g = grant("g2", Granularity::Range { min: 10, max: None }, false, None);
        let mut revoked = HashSet::new();
        revoked.insert("g2".to_string());
        assert_eq!(g.authorize_new_disclosure(100, &revoked), AuthzDecision::Allowed);
    }

    #[test]
    fn expiry_gates_future_disclosures() {
        let g = grant("g3", Granularity::Exact, true, Some(50));
        let revoked = HashSet::new();
        assert_eq!(g.authorize_new_disclosure(40, &revoked), AuthzDecision::Allowed);
        assert_eq!(g.authorize_new_disclosure(60, &revoked), AuthzDecision::Expired);
    }

    #[test]
    fn exact_is_irreversible_and_says_so() {
        let g = grant("g4", Granularity::Exact, true, None);
        assert_eq!(g.granularity.reversibility(), Reversibility::Irreversible);
        assert!(g.revocation_note().contains("cannot be recalled"));
    }

    #[test]
    fn range_is_predicate_only() {
        let g = grant("g5", Granularity::Range { min: 1, max: Some(2) }, true, None);
        assert_eq!(g.granularity.reversibility(), Reversibility::PredicateOnly);
    }
}
