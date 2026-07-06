//! Build an aperture range proof and emit an (unsigned) transaction that submits it to the
//! ZK ElGamal Proof Program (`ZkE1Gama1Proof11111111111111111111111111111`). The base64 output is
//! fed to devnet `simulateTransaction` (sigVerify=false, replaceRecentBlockhash=true) — if the
//! reactivated program accepts the proof on-chain, simulation returns `err: null`.

use base64::Engine;
use solana_address::Address;
use solana_message::Message;
use solana_transaction::Transaction;
use solana_zk_elgamal_proof_interface::{
    instruction::ProofInstruction,
    proof_data::batched_range_proof::{BatchedRangeProofContext, BatchedRangeProofU64Data},
};
use solana_zk_sdk::{
    encryption::{
        elgamal::ElGamalKeypair,
        pedersen::{Pedersen, PedersenOpening},
    },
    zk_elgamal_proof_program::batched_range_proof::build_batched_range_proof_u64_data,
};
use std::str::FromStr;

fn main() {
    // 1. Range proof: prove balance (15M) >= threshold (10M) with the value hidden.
    let fund = ElGamalKeypair::new_rand();
    let (balance, threshold) = (15_000_000u64, 10_000_000u64);
    let opening = PedersenOpening::new_rand();
    let _c_bal = fund.pubkey().encrypt_with(balance, &opening);
    let delta = balance - threshold;
    let commit_delta = Pedersen::with(delta, &opening);
    let proof =
        build_batched_range_proof_u64_data(vec![&commit_delta], vec![delta], vec![64], vec![&opening])
            .expect("range proof generation");

    // 2. The real ZK ElGamal Proof Program instruction (proof carried in instruction data).
    let ix = ProofInstruction::VerifyBatchedRangeProofU64
        .encode_verify_proof::<BatchedRangeProofU64Data, BatchedRangeProofContext>(None, &proof);

    // 3. Unsigned tx (simulation replaces the blockhash and skips sig verification).
    //    The fee payer must EXIST on devnet (simulation loads it); pass a funded pubkey as arg 1.
    let payer_str = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "BjKr5GrbtX4saPvirVaW1QhjspaY9hgBFEzYvQaeRk1m".to_string());
    let payer = Address::from_str(&payer_str).expect("valid base58 payer pubkey");
    let tx = Transaction::new_unsigned(Message::new(&[ix], Some(&payer)));

    let bytes = bincode::serialize(&tx).expect("serialize tx");
    println!("{}", base64::engine::general_purpose::STANDARD.encode(bytes));
}
