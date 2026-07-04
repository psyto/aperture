//! LiteSVM integration tests for the Receipt Registry program.
//! Prereq: `cd ../receipts && cargo build-sbf` (produces the .so loaded below).

use litesvm::LiteSVM;
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::path::PathBuf;

fn program_so() -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../receipts/target/deploy/aperture_receipts.so");
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {p:?}: {e} — run `cargo build-sbf` in ../receipts"))
}

const SYSTEM_PROGRAM: Pubkey = solana_sdk_ids::system_program::ID;

fn record_ix(
    program_id: Pubkey,
    issuer: Pubkey,
    receipt: Pubkey,
    commitment: [u8; 32],
    recipient: [u8; 32],
    grant_id: [u8; 32],
    expiry: i64,
) -> Instruction {
    let mut data = Vec::with_capacity(105);
    data.push(0u8); // RecordDisclosure
    data.extend_from_slice(&commitment);
    data.extend_from_slice(&recipient);
    data.extend_from_slice(&grant_id);
    data.extend_from_slice(&expiry.to_le_bytes());
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

fn revoke_ix(program_id: Pubkey, issuer: Pubkey, receipt: Pubkey) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(issuer, true),
            AccountMeta::new(receipt, false),
        ],
        data: vec![1u8], // RevokeDisclosure
    }
}

struct Fixture {
    svm: LiteSVM,
    program_id: Pubkey,
    issuer: Keypair,
    commitment: [u8; 32],
    recipient: [u8; 32],
    grant_id: [u8; 32],
    receipt: Pubkey,
}

fn setup() -> Fixture {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::new_unique();
    svm.add_program(program_id, &program_so()).expect("add program");
    let issuer = Keypair::new();
    svm.airdrop(&issuer.pubkey(), 1_000_000_000).unwrap();
    let commitment = [0xC1u8; 32];
    let recipient = [0x2Au8; 32];
    let grant_id = [0x9Fu8; 32];
    let (receipt, _bump) = Pubkey::find_program_address(
        &[b"receipt", issuer.pubkey().as_ref(), &commitment],
        &program_id,
    );
    Fixture { svm, program_id, issuer, commitment, recipient, grant_id, receipt }
}

fn do_record(f: &mut Fixture) -> Result<(), String> {
    let ix = record_ix(
        f.program_id,
        f.issuer.pubkey(),
        f.receipt,
        f.commitment,
        f.recipient,
        f.grant_id,
        0,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&f.issuer.pubkey()),
        &[&f.issuer],
        f.svm.latest_blockhash(),
    );
    f.svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{e:?}"))
}

#[test]
fn record_creates_content_blind_receipt() {
    let mut f = setup();
    f.svm.warp_to_slot(100); // advance the on-chain clock so the stamp is observable
    do_record(&mut f).expect("record");

    let acct = f.svm.get_account(&f.receipt).expect("receipt exists");
    assert_eq!(acct.owner, f.program_id, "owned by the registry program");
    assert_eq!(acct.data.len(), 147, "fixed receipt length");
    assert_eq!(acct.data[0], 1, "initialized tag");
    assert_eq!(&acct.data[1..33], f.issuer.pubkey().as_ref(), "issuer");
    assert_eq!(&acct.data[33..65], &f.recipient, "recipient hash");
    assert_eq!(&acct.data[65..97], &f.commitment, "commitment (only a hash — content-blind)");
    assert_eq!(&acct.data[97..129], &f.grant_id, "grant id hash");
    let slot = u64::from_le_bytes(acct.data[129..137].try_into().unwrap());
    assert_eq!(slot, 100, "issued slot reflects the on-chain clock");
    assert_eq!(acct.data[145], 0, "not revoked");
}

#[test]
fn issuer_can_revoke() {
    let mut f = setup();
    do_record(&mut f).expect("record");

    let ix = revoke_ix(f.program_id, f.issuer.pubkey(), f.receipt);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&f.issuer.pubkey()),
        &[&f.issuer],
        f.svm.latest_blockhash(),
    );
    f.svm.send_transaction(tx).expect("revoke");

    let acct = f.svm.get_account(&f.receipt).unwrap();
    assert_eq!(acct.data[145], 1, "revoked flag set");
}

#[test]
fn non_issuer_cannot_revoke() {
    let mut f = setup();
    do_record(&mut f).expect("record");

    let attacker = Keypair::new();
    f.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();
    // Attacker points at the real receipt PDA but signs as themselves.
    let ix = revoke_ix(f.program_id, attacker.pubkey(), f.receipt);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        f.svm.latest_blockhash(),
    );
    assert!(f.svm.send_transaction(tx).is_err(), "only the recorded issuer may revoke");

    let acct = f.svm.get_account(&f.receipt).unwrap();
    assert_eq!(acct.data[145], 0, "still not revoked");
}

#[test]
fn double_record_is_rejected() {
    let mut f = setup();
    do_record(&mut f).expect("first record");
    assert!(do_record(&mut f).is_err(), "same commitment cannot be recorded twice");
}
