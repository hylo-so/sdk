//! Memo formatting for Squads vault transactions. The memo is the
//! instruction name followed by a hash of its canonical encoding,
//! letting reviewers verify a proposal matches expected args by
//! independently rebuilding the instruction and comparing hashes.

use std::mem::size_of;

use anchor_client::solana_sdk::hash::{hash, Hash};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_lang::prelude::instruction::Instruction;
use anchor_lang::prelude::AccountMeta;

#[must_use]
fn account_meta_bytes(
  &AccountMeta {
    pubkey,
    is_signer,
    is_writable,
  }: &AccountMeta,
) -> [u8; size_of::<AccountMeta>()] {
  let mut bytes = [0; 34];
  bytes[..32].copy_from_slice(pubkey.as_array());
  bytes[32..].copy_from_slice(&[is_signer.into(), is_writable.into()]);
  bytes
}

fn instruction_bytes(
  Instruction {
    program_id,
    accounts,
    data,
  }: &Instruction,
) -> Vec<u8> {
  let program_id_len = size_of::<Pubkey>();
  let accounts_len = accounts.len() * size_of::<AccountMeta>();
  let mut bytes =
    Vec::with_capacity(program_id_len + accounts_len + data.len());
  bytes.extend_from_slice(program_id.as_array());
  bytes.extend(accounts.iter().flat_map(account_meta_bytes));
  bytes.extend(data);
  bytes
}

/// SHA-256 of the instruction's program id, accounts, and data.
#[must_use]
pub fn instruction_hash(i: &Instruction) -> Hash {
  hash(&instruction_bytes(i))
}

/// Builds a Squads memo as `<instruction_name> <hash>`.
#[must_use]
pub fn build_memo(instruction_name: &str, i: &Instruction) -> String {
  format!("{instruction_name} {}", instruction_hash(i))
}

#[cfg(test)]
mod tests {
  use anchor_client::solana_sdk::pubkey::Pubkey;
  use hylo_idl::exchange::client::args;
  use hylo_idl::exchange::instruction_builders;
  use hylo_idl::exchange::types::UFixValue64;

  use super::build_memo;

  fn pk(byte: u8) -> Pubkey {
    Pubkey::new_from_array([byte; 32])
  }

  #[test]
  fn memo_is_deterministic() {
    let args = args::UpdateLstSwapFee {
      new_lst_swap_fee: UFixValue64 { bits: 50, exp: -4 },
    };
    let instruction = instruction_builders::update_lst_swap_fee(pk(1), &args);
    let a = build_memo("update_lst_swap_fee", &instruction);
    let b = build_memo("update_lst_swap_fee", &instruction);
    assert_eq!(a, b);
    assert!(a.starts_with("update_lst_swap_fee "));
  }

  #[test]
  fn memo_snapshot() {
    let args = args::UpdateLstSwapFee {
      new_lst_swap_fee: UFixValue64 { bits: 50, exp: -4 },
    };
    let instruction = instruction_builders::update_lst_swap_fee(pk(7), &args);
    let memo = build_memo("update_lst_swap_fee", &instruction);
    assert_eq!(
      memo,
      "update_lst_swap_fee E9wxi5EQDPtc5u9aSYfWY4zh6X8HYJufLN1ZNqXfdT5v",
    );
  }

  #[test]
  fn memo_changes_with_args() {
    let paused = args::UpdatePaused { new_paused: true };
    let unpaused = args::UpdatePaused { new_paused: false };
    let ix_paused = instruction_builders::update_paused(pk(1), &paused);
    let ix_unpaused = instruction_builders::update_paused(pk(1), &unpaused);
    assert_ne!(
      build_memo("update_paused", &ix_paused),
      build_memo("update_paused", &ix_unpaused),
    );
  }
}
