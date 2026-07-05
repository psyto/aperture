//! L2 — the Substrate Adapter interface. Everything above this (policy, package, verifier) is
//! substrate-agnostic; everything substrate-specific (proof math, ciphertext binding, trust model)
//! is confined below it. Swapping Token-2022 for Arcium CSPL / ERC-7984 means implementing this
//! trait, not touching L1.

use crate::package::{Claim, ProofEnvelope, SubjectAccount, TrustModel};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyOutcome {
    Valid,
    Invalid(String),
}

pub trait ConfidentialSubstrate {
    /// The trust assumption this substrate's proofs carry. Surfaced into every disclosure package.
    fn substrate_trust_model(&self) -> TrustModel;

    /// Verify that `proof` establishes `claim` about the balances referenced by `subject`.
    ///
    /// NB: binding the proof's commitment to the *live* on-chain account requires a chain fetch at
    /// `anchor.slot`; that RPC step is out of the skeleton's scope. What is verified here is the
    /// proof itself against the commitments embedded in it — the exact code the native ZK ElGamal
    /// Proof Program runs.
    fn verify_disclosure(
        &self,
        claim: &Claim,
        subject: &[SubjectAccount],
        proof: &ProofEnvelope,
    ) -> VerifyOutcome;
}
