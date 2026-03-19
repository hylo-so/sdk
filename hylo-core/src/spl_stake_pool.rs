use anchor_lang::prelude::{ProgramError, Result};
use fix::prelude::*;
use std::mem::size_of;

const U64_SIZE: usize = size_of::<u64>();
const TOTAL_LAMPORTS_OFFSET: usize = 258;
const POOL_TOKEN_SUPPLY_OFFSET: usize = TOTAL_LAMPORTS_OFFSET + U64_SIZE;

/// Simplified view of stake pool PDA used in all SPL LST programs.
#[derive(Debug)]
pub struct SplStakePool {
  pub total_lamports: UFix64<N9>,
  pub pool_token_supply: UFix64<N9>,
}

impl SplStakePool {
  /// Deserializes [`SplStakePool`] from Borsh account data.
  pub fn from_bytes(data: &[u8]) -> Result<SplStakePool> {
    let total_lamports = data
      .get(TOTAL_LAMPORTS_OFFSET..TOTAL_LAMPORTS_OFFSET + U64_SIZE)
      .and_then(|b| b.try_into().ok())
      .map(u64::from_le_bytes)
      .map(UFix64::new)
      .ok_or(ProgramError::InvalidAccountData)?;

    let pool_token_supply = data
      .get(POOL_TOKEN_SUPPLY_OFFSET..POOL_TOKEN_SUPPLY_OFFSET + U64_SIZE)
      .and_then(|b| b.try_into().ok())
      .map(u64::from_le_bytes)
      .map(UFix64::new)
      .ok_or(ProgramError::InvalidAccountData)?;

    Ok(SplStakePool {
      total_lamports,
      pool_token_supply,
    })
  }

  /// Computes the true price of the LST via `total_lamports / pool_token_supply`.
  #[must_use]
  pub fn true_price(&self) -> Option<UFix64<N9>> {
    self
      .total_lamports
      .mul_div_floor(UFix64::one(), self.pool_token_supply)
  }
}
