//! Lightweight SPL stake pool deserialization.

use std::mem::size_of;

use anchor_lang::prelude::{ProgramError, Result};
use fix::prelude::*;

use crate::error::CoreError;
use crate::lst_sol_price::LstSolPrice;

/// Byte offsets in [`StakePool`].
/// <https://docs.rs/spl-stake-pool/latest/spl_stake_pool/state/struct.StakePool.html>
const TOTAL_LAMPORTS_OFFSET: usize = 258;
const POOL_TOKEN_SUPPLY_OFFSET: usize = TOTAL_LAMPORTS_OFFSET + U64_SIZE;
const LAST_UPDATE_EPOCH_OFFSET: usize = POOL_TOKEN_SUPPLY_OFFSET + U64_SIZE;
const U64_SIZE: usize = size_of::<u64>();

/// Minimal view of stake pool PDA used in all SPL LST programs.
#[derive(Debug)]
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
    let price = self
      .total_lamports
      .mul_div_floor(UFix64::one(), self.pool_token_supply)
      .ok_or(CoreError::StakePoolDivByZero)?;
    Ok(LstSolPrice::new(price.into(), self.last_update_epoch))
  }
}
