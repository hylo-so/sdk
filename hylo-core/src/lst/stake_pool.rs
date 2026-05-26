//! Lightweight SPL stake pool deserialization.

use std::mem::size_of;

use anchor_lang::prelude::{ProgramError, Result};
use fix::prelude::*;

use super::sol_price::LstSolPrice;
use crate::error::CoreError;

/// Byte offsets in [`StakePool`].
/// <https://docs.rs/spl-stake-pool/latest/spl_stake_pool/state/struct.StakePool.html>
const TOTAL_LAMPORTS_OFFSET: usize = 258;
const POOL_TOKEN_SUPPLY_OFFSET: usize = TOTAL_LAMPORTS_OFFSET + U64_SIZE;
const LAST_UPDATE_EPOCH_OFFSET: usize = POOL_TOKEN_SUPPLY_OFFSET + U64_SIZE;
const U64_SIZE: usize = size_of::<u64>();

/// Minimal view of stake pool PDA used in all SPL LST programs.
#[derive(Debug, Clone, Copy)]
pub struct SplStakePool {
  pub total_lamports: UFix64<N9>,
  pub pool_token_supply: UFix64<N9>,
  pub last_update_epoch: u64,
}

impl SplStakePool {
  /// Deserializes [`SplStakePool`] from Borsh account data.
  ///
  /// # Errors
  /// * Invalid account data
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

    let last_update_epoch = data
      .get(LAST_UPDATE_EPOCH_OFFSET..LAST_UPDATE_EPOCH_OFFSET + U64_SIZE)
      .and_then(|b| b.try_into().ok())
      .map(u64::from_le_bytes)
      .ok_or(ProgramError::InvalidAccountData)?;

    Ok(SplStakePool {
      total_lamports,
      pool_token_supply,
      last_update_epoch,
    })
  }

  /// Computes true price of this stake pool's LST as an
  /// [`LstSolPrice`] tagged with `last_update_epoch`.
  ///
  /// # Errors
  /// * `pool_token_supply` is zero
  pub fn true_price(&self) -> Result<LstSolPrice> {
    let price = SplStakePool::true_price_inner(
      self.total_lamports,
      self.pool_token_supply,
    )
    .ok_or(CoreError::StakePoolDivByZero)?;
    Ok(LstSolPrice::new(price.into(), self.last_update_epoch))
  }

  fn true_price_inner(
    total_lamports: UFix64<N9>,
    pool_token_supply: UFix64<N9>,
  ) -> Option<UFix64<N9>> {
    (pool_token_supply != UFix64::zero())
      .then_some(total_lamports)
      .and_then(|tl| tl.mul_div_floor(UFix64::one(), pool_token_supply))
  }
}

#[cfg(test)]
mod tests {
  use fix::prelude::*;

  use crate::error::CoreError;
  use crate::lst::stake_pool::SplStakePool;

  #[test]
  fn true_price_zero_supply_returns_div_by_zero() {
    let pool = SplStakePool {
      total_lamports: UFix64::<N9>::one(),
      pool_token_supply: UFix64::<N9>::zero(),
      last_update_epoch: 0,
    };
    assert_eq!(
      pool.true_price().err(),
      Some(CoreError::StakePoolDivByZero.into()),
    );
  }
}
