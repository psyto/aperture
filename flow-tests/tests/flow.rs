//! End-to-end: issue a disclosure (L1 core) -> anchor its commitment on-chain (Receipt Registry
//! via LiteSVM) -> verify. Then revoke, and show the delivered package is unchanged — revocation
//! governs FUTURE reliance, not clawback.
//!
//! Prereq: `cd ../receipts && cargo build-sbf`.

use aperture_core::package::{hash32, ChainAnchor, Claim, DisclosurePackage, SubstrateId};
use aperture_core::token2022::{issue_range_disclosure, Token2022Substrate};
use aperture_core::verifier::verify_package;
use solana_zk_sdk::encryption::elgamal::ElGamalKeypair;

use litesvm::LiteSVM;
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::path::PathBuf;

const SYSTEM_PROGRAM: Pubkey = solana_sdk_ids::system_program::ID;

fn program_so() -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../receipts/target/deploy/aperture_receipts.so");
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {p:?}: {e} — run `cargo build-sbf` in ../receipts"))
}

/// Build a range-disclosure package with its receipt commitment derived from the contents.
fn issue_package() -> DisclosurePackage {
    let fund = ElGamalKeypair::new_rand();
    let (claim, proof, subject) = issue_range_disclosure(&fund, 15_000_000, 10_000_000, "acct-1");
    let mut pkg = DisclosurePackage {
        package_id: "pkg-42".into(),
        grant_id: "grant-lp-quarterly".into(),
        substrate: SubstrateId::Token2022,
        issuer: "fund-A".into(),
        recipient: "LP".into(),
        issued_at: 1_000,
        expiry: Some(9_999),
        anchor: ChainAnchor { cluster: "litesvm".into(), slot: 0 },
        subject: vec![subject],
        claim: match claim {
            Claim::Range { min, max } => Claim::Range { min, max },
            other => other,
        },
        proof,
        receipt_commitment: vec![],
        issuer_signature: vec![],
    };
    pkg.receipt_commitment = pkg.derive_receipt_commitment().to_vec();
    pkg
}

fn record_ix(program_id: Pubkey, issuer: Pubkey, receipt: Pubkey, pkg: &DisclosurePackage) -> Instruction {
    let commitment: [u8; 32] = pkg.receipt_commitment.clone().try_into().unwrap();
    let recipient = hash32(pkg.recipient.as_bytes());
    let grant_id = hash32(pkg.grant_id.as_bytes());
    let mut data = Vec::with_capacity(105);
    data.push(0u8);
    data.extend_from_slice(&commitment);
    data.extend_from_slice(&recipient);
    data.extend_from_slice(&grant_id);
    data.extend_from_slice(&0i64.to_le_bytes());
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(issuer, true),
            AccountMeta::new(receipt, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

#[test]
fn issue_anchor_verify_then_revoke() {
    // 1. Issue (L1 core + Token-2022 adapter).
    let pkg = issue_package();

    // 2. Off-chain verification passes today (crypto + structural + receipt integrity).
    let report = verify_package(&pkg, &Token2022Substrate, 1_500, "LP");
    assert!(report.passed(), "core verify failed: {:?}", report);

    // 3. Anchor the content-blind commitment on-chain (Receipt Registry via LiteSVM).
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::new_unique();
    svm.add_program(program_id, &program_so()).expect("add program");
    let issuer = Keypair::new();
    svm.airdrop(&issuer.pubkey(), 1_000_000_000).unwrap();

    let commitment: [u8; 32] = pkg.receipt_commitment.clone().try_into().unwrap();
    let (receipt_pda, _bump) =
        Pubkey::find_program_address(&[b"receipt", issuer.pubkey().as_ref(), &commitment], &program_id);

    let ix = record_ix(program_id, issuer.pubkey(), receipt_pda, &pkg);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&issuer.pubkey()),
        &[&issuer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("anchor commitment");

    // 4. The on-chain receipt binds this exact disclosure and is not revoked.
    let acct = svm.get_account(&receipt_pda).expect("receipt exists");
    assert_eq!(acct.owner, program_id);
    assert_eq!(&acct.data[65..97], &commitment, "anchored commitment == package commitment");
    assert_eq!(acct.data[145], 0, "not revoked");

    // 5. Revoke the standing grant.
    let revoke = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(issuer.pubkey(), true),
            AccountMeta::new(receipt_pda, false),
        ],
        data: vec![1u8],
    };
    let tx = Transaction::new_signed_with_payer(
        &[revoke],
        Some(&issuer.pubkey()),
        &[&issuer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("revoke");
    let acct = svm.get_account(&receipt_pda).unwrap();
    assert_eq!(acct.data[145], 1, "on-chain revoked flag set");

    // 6. Revocation is NOT clawback: the already-delivered package still verifies cryptographically.
    //    Revocation governs future reliance (the on-chain receipt), not the issued artifact.
    let report_after = verify_package(&pkg, &Token2022Substrate, 1_500, "LP");
    assert!(report_after.proof_ok, "delivered proof is unchanged by revocation");
}
