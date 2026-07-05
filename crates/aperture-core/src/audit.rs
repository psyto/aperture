//! Auditor-side audit-trail and double-entry journal generation.
//!
//! Given confidential transactions whose amounts are encrypted under an auditor's ElGamal key, the
//! auditor decrypts them to produce a verifiable audit trail and a balanced double-entry journal —
//! "books an auditor can accept" over confidential on-chain activity. This is the audit face of the
//! disclosure engine; a downstream product (custody/compliance) consumes it.
//!
//! ElGamal decryption solves a discrete log, which is only feasible up to ~2^32. Amounts up to a
//! full `u64` are therefore carried as a **lo/hi split** (low 32 bits + high 32 bits), each half
//! within decryptable range — the standard Token-2022 pattern. See [`ConfidentialAmount`].

use serde::Serialize;
use solana_zk_sdk::encryption::elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey};

/// Bit shift for the high half of a split amount: low 32 bits + (high 32 bits << 32).
pub const SPLIT_SHIFT: u32 = 32;

const HALF_BOUND: u64 = 1u64 << SPLIT_SHIFT; // 2^32

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Inflow,
    Outflow,
}

/// A large amount encrypted as two decryptable halves. A single ElGamal ciphertext of a `u64`
/// cannot be decrypted efficiently (discrete log is only feasible up to ~2^32), so the amount is
/// split into a low and high half, each within range.
#[derive(Clone)]
pub struct SplitCiphertext {
    pub lo: ElGamalCiphertext,
    pub hi: ElGamalCiphertext,
}

/// How a confidential amount is carried: a single ciphertext (amount < 2^32) or a lo/hi split.
#[derive(Clone)]
pub enum ConfidentialAmount {
    Small(ElGamalCiphertext),
    Split(SplitCiphertext),
}

/// A confidential transaction as the auditor sees it: the amount is encrypted under the auditor's
/// key (as produced by a confidential transfer configured with an auditor pubkey).
#[derive(Clone)]
pub struct ConfidentialTxn {
    pub id: String,
    pub counterparty: String,
    pub slot: u64,
    pub direction: Direction,
    pub amount: ConfidentialAmount,
}

/// A decrypted audit record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AuditRecord {
    pub id: String,
    pub counterparty: String,
    pub slot: u64,
    pub direction: Direction,
    pub amount: u64,
}

/// A double-entry journal line (debit == credit == amount).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct JournalEntry {
    pub txn_id: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: u64,
}

/// Encrypt an amount as a lo/hi split under `pubkey`.
pub fn encrypt_split(pubkey: &ElGamalPubkey, amount: u64) -> SplitCiphertext {
    SplitCiphertext {
        lo: pubkey.encrypt(amount & (HALF_BOUND - 1)),
        hi: pubkey.encrypt(amount >> SPLIT_SHIFT),
    }
}

/// Encrypt an amount, choosing `Small` for values < 2^32 and `Split` for larger values.
pub fn encrypt_amount(pubkey: &ElGamalPubkey, amount: u64) -> ConfidentialAmount {
    if amount < HALF_BOUND {
        ConfidentialAmount::Small(pubkey.encrypt(amount))
    } else {
        ConfidentialAmount::Split(encrypt_split(pubkey, amount))
    }
}

/// Decrypt a lo/hi split amount, recombining into the full `u64`. `None` if a half is undecryptable
/// or malformed (a half outside `[0, 2^32)`).
pub fn decrypt_split(auditor: &ElGamalKeypair, split: &SplitCiphertext) -> Option<u64> {
    let lo = split.lo.decrypt_u32(auditor.secret())?;
    let hi = split.hi.decrypt_u32(auditor.secret())?;
    if lo >= HALF_BOUND || hi >= HALF_BOUND {
        return None;
    }
    Some(lo + (hi << SPLIT_SHIFT))
}

/// Decrypt any confidential amount (single ciphertext or lo/hi split) with the auditor key.
pub fn decrypt_amount(auditor: &ElGamalKeypair, amount: &ConfidentialAmount) -> Option<u64> {
    match amount {
        ConfidentialAmount::Small(ct) => ct.decrypt_u32(auditor.secret()),
        ConfidentialAmount::Split(split) => decrypt_split(auditor, split),
    }
}

/// Build the audit trail by decrypting every transaction. Returns `None` if any fails to decrypt.
pub fn build_audit_trail(auditor: &ElGamalKeypair, txns: &[ConfidentialTxn]) -> Option<Vec<AuditRecord>> {
    txns
        .iter()
        .map(|t| {
            decrypt_amount(auditor, &t.amount).map(|amount| AuditRecord {
                id: t.id.clone(),
                counterparty: t.counterparty.clone(),
                slot: t.slot,
                direction: t.direction,
                amount,
            })
        })
        .collect()
}

/// Produce a double-entry journal from the audit trail.
/// Inflow: Dr Cash / Cr counterparty.  Outflow: Dr counterparty / Cr Cash.
pub fn build_journal(records: &[AuditRecord]) -> Vec<JournalEntry> {
    records
        .iter()
        .map(|r| match r.direction {
            Direction::Inflow => JournalEntry {
                txn_id: r.id.clone(),
                debit_account: "Cash".into(),
                credit_account: r.counterparty.clone(),
                amount: r.amount,
            },
            Direction::Outflow => JournalEntry {
                txn_id: r.id.clone(),
                debit_account: r.counterparty.clone(),
                credit_account: "Cash".into(),
                amount: r.amount,
            },
        })
        .collect()
}

/// (total debits, total credits). A well-formed double-entry journal is balanced — they are equal.
pub fn totals(journal: &[JournalEntry]) -> (u64, u64) {
    let debits: u64 = journal.iter().map(|e| e.amount).sum();
    let credits: u64 = journal.iter().map(|e| e.amount).sum();
    (debits, credits)
}

/// Quote/escape one CSV field per RFC 4180 (only when it contains a comma, quote, or newline).
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Render the audit trail as CSV — the artifact an auditor actually consumes.
pub fn audit_trail_csv(records: &[AuditRecord]) -> String {
    let mut out = String::from("id,counterparty,slot,direction,amount\n");
    for r in records {
        let dir = match r.direction {
            Direction::Inflow => "inflow",
            Direction::Outflow => "outflow",
        };
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_field(&r.id),
            csv_field(&r.counterparty),
            r.slot,
            dir,
            r.amount
        ));
    }
    out
}

/// Render the double-entry journal as CSV.
pub fn journal_csv(entries: &[JournalEntry]) -> String {
    let mut out = String::from("txn_id,debit_account,credit_account,amount\n");
    for e in entries {
        out.push_str(&format!(
            "{},{},{},{}\n",
            csv_field(&e.txn_id),
            csv_field(&e.debit_account),
            csv_field(&e.credit_account),
            e.amount
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn txn(id: &str, cp: &str, slot: u64, dir: Direction, amount: u64, auditor: &ElGamalKeypair) -> ConfidentialTxn {
        ConfidentialTxn {
            id: id.into(),
            counterparty: cp.into(),
            slot,
            direction: dir,
            amount: encrypt_amount(auditor.pubkey(), amount),
        }
    }

    #[test]
    fn auditor_decrypts_confidential_txns_into_a_balanced_journal() {
        let auditor = ElGamalKeypair::new_rand();
        let txns = vec![
            txn("t1", "LP-A", 100, Direction::Inflow, 250, &auditor),
            txn("t2", "Vendor-B", 101, Direction::Outflow, 1_200, &auditor),
            txn("t3", "LP-C", 102, Direction::Inflow, 75, &auditor),
        ];

        let trail = build_audit_trail(&auditor, &txns).expect("all decrypt");
        assert_eq!(trail.iter().map(|r| r.amount).collect::<Vec<_>>(), vec![250, 1_200, 75]);

        let journal = build_journal(&trail);
        assert_eq!(journal.len(), 3);
        assert_eq!(journal[0].debit_account, "Cash");
        assert_eq!(journal[1].credit_account, "Cash");

        let (debits, credits) = totals(&journal);
        assert_eq!(debits, credits, "double-entry journal is balanced");
        assert_eq!(debits, 250 + 1_200 + 75);
    }

    #[test]
    fn wrong_auditor_key_cannot_read_the_books() {
        let auditor = ElGamalKeypair::new_rand();
        let stranger = ElGamalKeypair::new_rand();
        let txns = vec![txn("t1", "LP-A", 100, Direction::Inflow, 250, &auditor)];
        assert_ne!(
            decrypt_amount(&stranger, &txns[0].amount),
            Some(250),
            "only the configured auditor key recovers the amount"
        );
    }

    #[test]
    fn large_amount_round_trips_via_lo_hi_split() {
        let auditor = ElGamalKeypair::new_rand();
        // 5 trillion (~2^42) is far beyond a single ciphertext's ~2^32 decryptable range.
        let big: u64 = 5_000_000_000_000;
        let amount = encrypt_amount(auditor.pubkey(), big);
        assert!(matches!(amount, ConfidentialAmount::Split(_)), "large → split representation");
        assert_eq!(decrypt_amount(&auditor, &amount), Some(big));
    }

    #[test]
    fn a_single_ciphertext_cannot_hold_a_large_amount() {
        let auditor = ElGamalKeypair::new_rand();
        let big: u64 = 5_000_000_000_000;
        // Forcing the large value into one ciphertext is not decryptable (discrete log > 2^32).
        let single = ConfidentialAmount::Small(auditor.pubkey().encrypt(big));
        assert_eq!(decrypt_amount(&auditor, &single), None, "motivates the split");
    }

    #[test]
    fn split_boundary_values() {
        let auditor = ElGamalKeypair::new_rand();
        for v in [0u64, 1, HALF_BOUND - 1, HALF_BOUND, HALF_BOUND + 1, u64::MAX] {
            let amount = encrypt_amount(auditor.pubkey(), v);
            assert_eq!(decrypt_amount(&auditor, &amount), Some(v), "round-trip {v}");
        }
    }

    #[test]
    fn exports_csv_with_rfc4180_escaping() {
        let auditor = ElGamalKeypair::new_rand();
        let txns = vec![
            txn("t1", "LP, Ltd.", 100, Direction::Inflow, 250, &auditor), // comma in name
            txn("t2", "Vendor", 101, Direction::Outflow, 120, &auditor),
        ];
        let trail = build_audit_trail(&auditor, &txns).unwrap();

        let csv = audit_trail_csv(&trail);
        assert!(csv.starts_with("id,counterparty,slot,direction,amount\n"));
        assert!(csv.contains("t1,\"LP, Ltd.\",100,inflow,250\n"), "comma field quoted: {csv}");

        let journal = build_journal(&trail);
        let jcsv = journal_csv(&journal);
        assert!(jcsv.starts_with("txn_id,debit_account,credit_account,amount\n"));
        assert!(jcsv.contains("t1,Cash,\"LP, Ltd.\",250\n"), "inflow debits Cash: {jcsv}");
    }

    #[test]
    fn records_serialize_to_json() {
        let r = AuditRecord {
            id: "t1".into(),
            counterparty: "LP".into(),
            slot: 5,
            direction: Direction::Outflow,
            amount: 999,
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("\"amount\":999"));
        assert!(j.contains("\"direction\":\"outflow\""), "rename_all lowercase: {j}");
    }
}
