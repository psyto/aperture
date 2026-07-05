//! Receipt Registry — content-blind non-repudiation for Aperture disclosures.
//!
//! Anchors "issuer disclosed commitment C (of grant G) to recipient R at slot S" on-chain,
//! storing ONLY hashes. It needs no ZK program, so the audit trail of *what was disclosed to whom
//! when* survives even while on-chain proof verification is feature-gated off on mainnet.
//!
//! Instructions:
//!   0 RecordDisclosure — create a receipt PDA. seeds = [b"receipt", issuer, commitment]
//!   1 RevokeDisclosure — mark a receipt revoked (issuer-signed). Governs FUTURE reliance on the
//!     standing grant; it does NOT and cannot claw back an exact value already delivered.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

/// Marks an initialized receipt account.
pub const RECEIPT_TAG: u8 = 1;

/// Fixed receipt layout (bytes):
/// [0] tag | [1..33] issuer | [33..65] recipient | [65..97] commitment |
/// [97..129] grant_id | [129..137] issued_slot(u64 LE) | [137..145] expiry(i64 LE) |
/// [145] revoked | [146] bump
pub const RECEIPT_LEN: usize = 147;

entrypoint!(process);

fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (&disc, rest) = data.split_first().ok_or(ProgramError::InvalidInstructionData)?;
    match disc {
        0 => record_disclosure(program_id, accounts, rest),
        1 => revoke_disclosure(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// data = [commitment:32][recipient:32][grant_id:32][expiry:i64 LE:8]
/// accounts = [issuer(signer,payer,w), receipt PDA(w), system_program]
fn record_disclosure(program_id: &Pubkey, accounts: &[AccountInfo], rest: &[u8]) -> ProgramResult {
    if rest.len() != 32 + 32 + 32 + 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let commitment = &rest[0..32];
    let recipient = &rest[32..64];
    let grant_id = &rest[64..96];
    let expiry = i64::from_le_bytes(rest[96..104].try_into().unwrap());

    let iter = &mut accounts.iter();
    let issuer = next_account_info(iter)?;
    let receipt = next_account_info(iter)?;
    let system_program = next_account_info(iter)?;

    if !issuer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, bump) =
        Pubkey::find_program_address(&[b"receipt", issuer.key.as_ref(), commitment], program_id);
    if pda != *receipt.key {
        return Err(ProgramError::InvalidSeeds);
    }
    if receipt.owner == program_id {
        // already recorded
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(RECEIPT_LEN);
    invoke_signed(
        &system_instruction::create_account(
            issuer.key,
            receipt.key,
            lamports,
            RECEIPT_LEN as u64,
            program_id,
        ),
        &[issuer.clone(), receipt.clone(), system_program.clone()],
        &[&[b"receipt", issuer.key.as_ref(), commitment, &[bump]]],
    )?;

    let clock = Clock::get()?;
    let mut d = receipt.try_borrow_mut_data()?;
    d[0] = RECEIPT_TAG;
    d[1..33].copy_from_slice(issuer.key.as_ref());
    d[33..65].copy_from_slice(recipient);
    d[65..97].copy_from_slice(commitment);
    d[97..129].copy_from_slice(grant_id);
    d[129..137].copy_from_slice(&clock.slot.to_le_bytes());
    d[137..145].copy_from_slice(&expiry.to_le_bytes());
    d[145] = 0;
    d[146] = bump;
    msg!("aperture: recorded disclosure receipt");
    Ok(())
}

/// accounts = [issuer(signer), receipt PDA(w)]
fn revoke_disclosure(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let issuer = next_account_info(iter)?;
    let receipt = next_account_info(iter)?;

    if !issuer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if receipt.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    let mut d = receipt.try_borrow_mut_data()?;
    if d.len() != RECEIPT_LEN || d[0] != RECEIPT_TAG {
        return Err(ProgramError::UninitializedAccount);
    }
    // Only the recorded issuer may revoke.
    if &d[1..33] != issuer.key.as_ref() {
        return Err(ProgramError::IllegalOwner);
    }
    d[145] = 1;
    msg!("aperture: revoked disclosure receipt (future reliance only; not clawback)");
    Ok(())
}
