use anchor_lang::prelude::{ProgramError, Result};
use fix::prelude::{UFix64, N9};

pub const TOTAL_LAMPORTS_OFFSET: usize = 258;
pub const POOL_TOKEN_SUPPLY_OFFSET: usize = TOTAL_LAMPORTS_OFFSET + 8;

/// Simplified view of stake pool PDA used in all SPL LST programs.
pub struct SplStakePool {
  pub total_lamports: UFix64<N9>,
  pub pool_token_supply: UFix64<N9>,
}

impl SplStakePool {
  pub fn from_bytes(data: &[u8]) -> Result<SplStakePool> {
    let total_lamports = data
      .get(TOTAL_LAMPORTS_OFFSET..TOTAL_LAMPORTS_OFFSET + 8)
      .and_then(|b| b.try_into().ok())
      .map(u64::from_le_bytes)
      .map(UFix64::new)
      .ok_or(ProgramError::InvalidAccountData)?;

    let pool_token_supply = data
      .get(POOL_TOKEN_SUPPLY_OFFSET..POOL_TOKEN_SUPPLY_OFFSET + 8)
      .and_then(|b| b.try_into().ok())
      .map(u64::from_le_bytes)
      .map(UFix64::new)
      .ok_or(ProgramError::InvalidAccountData)?;

    Ok(SplStakePool {
      total_lamports,
      pool_token_supply,
    })
  }
}
