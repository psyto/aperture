//! The disclosure package — the substrate-agnostic artifact handed to a recipient, and the
//! content-blind receipt anchored on-chain. Serde-serializable; carries a `trust_model` so an
//! auditor knows what assumption a claim rests on.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// SHA-256 of arbitrary bytes into a 32-byte id (used to fold strings into fixed-size on-chain
/// receipt fields — content-blind).
pub fn hash32(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Trust assumption a claim's proof rests on. This is the differentiator we can attest that the
/// commoditized primitives cannot: Token-2022 is the only zero-external-trust substrate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustModel {
    /// Native on-chain ZK verification; no external committee. (Token-2022)
    NativeZero,
    /// MPC honest-majority committee. (Arcium CSPL)
    MpcHonestMajority,
    /// FHE threshold-decryption committee. (ERC-7984 / Zama)
    FheThreshold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubstrateId {
    Token2022,
    ArciumCspl,
    Erc7984,
}

/// What the package asserts. Mirrors `policy::Granularity` but is the on-the-wire claim.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Claim {
    /// Exact value; delivered out-of-band re-encrypted under the recipient key. Irreversible.
    Exact,
    Range { min: u64, max: Option<u64> },
    Aggregate { min: u64, max: Option<u64> },
}

/// Pins the state version the claim is about.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainAnchor {
    pub cluster: String,
    pub slot: u64,
}

/// References an on-chain confidential balance by commitment (never copies plaintext).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubjectAccount {
    pub address: String,
    pub elgamal_pubkey: Vec<u8>,
    /// Hash/commitment of the on-chain balance ciphertext this claim binds to.
    pub ciphertext_commitment: Vec<u8>,
}

/// The proof and its trust context.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofEnvelope {
    /// e.g. "t22-batched-range-u64-v1".
    pub system_id: String,
    pub trust_model: TrustModel,
    /// Serialized proof data (bytemuck Pod bytes of the ZK ElGamal proof).
    pub bytes: Vec<u8>,
}

/// The full disclosure package.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisclosurePackage {
    pub package_id: String,
    pub grant_id: String,
    pub substrate: SubstrateId,
    pub issuer: String,
    pub recipient: String,
    pub issued_at: i64,
    pub expiry: Option<i64>,
    pub anchor: ChainAnchor,
    pub subject: Vec<SubjectAccount>,
    pub claim: Claim,
    pub proof: ProofEnvelope,
    /// Content-blind commitment anchored by the Receipt Registry program. Needs NO ZK program,
    /// so non-repudiation of *what was disclosed* survives even while on-chain proof verification
    /// is feature-gated off.
    pub receipt_commitment: Vec<u8>,
    /// Issuer signature over the package (stub in the skeleton).
    pub issuer_signature: Vec<u8>,
}

impl DisclosurePackage {
    /// Derive the content-blind commitment that the Receipt Registry anchors on-chain. Binds the
    /// identifying fields (never plaintext) so the on-chain receipt is tied to this exact
    /// disclosure. The verifier recomputes this and checks it matches `receipt_commitment`.
    pub fn derive_receipt_commitment(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.package_id.as_bytes());
        h.update(self.grant_id.as_bytes());
        h.update(self.recipient.as_bytes());
        h.update(format!("{:?}", self.claim).as_bytes());
        for s in &self.subject {
            h.update(&s.ciphertext_commitment);
        }
        h.finalize().into()
    }
}
