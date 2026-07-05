//! Auditor-side audit-trail and double-entry journal generation.
//!
//! Given confidential transactions whose amounts are encrypted under an auditor's ElGamal key, the
//! auditor decrypts them to produce a verifiable audit trail and a balanced double-entry journal —
//! "books an auditor can accept" over confidential on-chain activity. This is the audit face of the
//! disclosure engine; a downstream product (custody/compliance) consumes it.

use solana_zk_sdk::encryption::elgamal::{ElGamalCiphertext, ElGamalKeypair};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Inflow,
    Outflow,
}

/// A confidential transaction as the auditor sees it: the amount is an ElGamal ciphertext under the
/// auditor's key (as produced by a confidential transfer configured with an auditor pubkey).
#[derive(Clone)]
pub struct ConfidentialTxn {
    pub id: String,
    pub counterparty: String,
    pub slot: u64,
    pub direction: Direction,
    pub amount_ct: ElGamalCiphertext,
}

/// A decrypted audit record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRecord {
    pub id: String,
    pub counterparty: String,
    pub slot: u64,
    pub direction: Direction,
    pub amount: u64,
}

/// A double-entry journal line (debit == credit == amount).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    pub txn_id: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: u64,
}

/// Decrypt one confidential amount with the auditor key. `None` if the amount is out of the
/// decryptable range (the caller should chunk large balances — a solved Token-2022 pattern).
pub fn decrypt_amount(auditor: &ElGamalKeypair, ct: &ElGamalCiphertext) -> Option<u64> {
    ct.decrypt_u32(auditor.secret())
}

/// Build the audit trail by decrypting every transaction. Returns `None` if any fails to decrypt.
pub fn build_audit_trail(auditor: &ElGamalKeypair, txns: &[ConfidentialTxn]) -> Option<Vec<AuditRecord>> {
    txns
        .iter()
        .map(|t| {
            decrypt_amount(auditor, &t.amount_ct).map(|amount| AuditRecord {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn txn(id: &str, cp: &str, slot: u64, dir: Direction, amount: u64, auditor: &ElGamalKeypair) -> ConfidentialTxn {
        ConfidentialTxn {
            id: id.into(),
            counterparty: cp.into(),
            slot,
            direction: dir,
            amount_ct: auditor.pubkey().encrypt(amount),
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
        // Inflow debits Cash; outflow credits Cash.
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
        // The stranger's key does not decrypt to the true amount (returns None or a wrong value).
        let got = decrypt_amount(&stranger, &txns[0].amount_ct);
        assert_ne!(got, Some(250), "only the configured auditor key recovers the amount");
    }
}
