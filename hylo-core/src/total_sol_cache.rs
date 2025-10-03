use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::{
  TotalSolCacheDecrement, TotalSolCacheIncrement, TotalSolCacheOutdated,
  TotalSolCacheOverflow, TotalSolCacheUnderflow,
};

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy)]
pub struct TotalSolCache {
  pub current_update_epoch: u64,
  pub total_sol: UFixValue64,
}

impl TotalSolCache {
  #[must_use]
  pub fn new(current_update_epoch: u64) -> TotalSolCache {
    let total_sol = UFix64::<N9>::zero().into();
    TotalSolCache {
      current_update_epoch,
      total_sol,
    }
  }

  /// Adds lamports to the cached amount.
  pub fn increment(
    &mut self,
    sol_in: UFix64<N9>,
    current_epoch: u64,
  ) -> Result<()> {
    if current_epoch == self.current_update_epoch {
      let prev_total: UFix64<N9> = self.total_sol.try_into()?;
      let new_total = prev_total
        .checked_add(&sol_in)
        .ok_or(TotalSolCacheOverflow)?;
      self.total_sol = new_total.into();
      Ok(())
    } else {
      Err(TotalSolCacheIncrement.into())
    }
  }

  /// Subtracts lamports from the cached amount.
  pub fn decrement(
    &mut self,
    sol_out: UFix64<N9>,
    current_epoch: u64,
  ) -> Result<()> {
    if current_epoch == self.current_update_epoch {
      let prev_total: UFix64<N9> = self.total_sol.try_into()?;
      let new_total = prev_total
        .checked_sub(&sol_out)
        .ok_or(TotalSolCacheUnderflow)?;
      self.total_sol = new_total.into();
      Ok(())
    } else {
      Err(TotalSolCacheDecrement.into())
    }
  }

  /// Resets cache and current epoch. Used only in price update instruction.
  pub fn set(
    &mut self,
    total_sol: UFix64<N9>,
    current_epoch: u64,
  ) -> Result<()> {
    self.current_update_epoch = current_epoch;
    self.total_sol = total_sol.into();
    Ok(())
  }

  /// Gets the total SOL cached value, if last update epoch is same as current.
  pub fn get_validated(&self, current_epoch: u64) -> Result<UFix64<N9>> {
    if current_epoch == self.current_update_epoch {
      self.total_sol.try_into()
    } else {
      Err(TotalSolCacheOutdated.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const CURRENT_EPOCH: u64 = 0;

  #[test]
  fn increment_ok() -> Result<()> {
    let mut cache = TotalSolCache::new(CURRENT_EPOCH);
    cache.increment(UFix64::new(364), CURRENT_EPOCH)?;
    cache.increment(UFix64::new(69), CURRENT_EPOCH)?;
    assert_eq!(UFix64::<N9>::new(433), cache.total_sol.try_into()?);
    Ok(())
  }

  #[test]
  fn increment_decrement_ok() -> Result<()> {
    let mut cache = TotalSolCache::new(CURRENT_EPOCH);
    cache.increment(UFix64::new(420), CURRENT_EPOCH)?;
    cache.decrement(UFix64::new(69), CURRENT_EPOCH)?;
    assert_eq!(UFix64::<N9>::new(351), cache.total_sol.try_into()?);
    Ok(())
  }

  #[test]
  fn wrong_epoch_err() {
    let mut cache = TotalSolCache::new(CURRENT_EPOCH);
    let inc = cache.increment(UFix64::new(420), CURRENT_EPOCH + 1);
    let dec = cache.decrement(UFix64::new(420), CURRENT_EPOCH + 1);
    assert!(inc.is_err_and(|e| e == TotalSolCacheIncrement.into()));
    assert!(dec.is_err_and(|e| e == TotalSolCacheDecrement.into()));
  }

  #[test]
  fn overflow_underflow_err() -> Result<()> {
    let mut cache = TotalSolCache::new(CURRENT_EPOCH);
    cache.set(UFix64::new(u64::MAX), CURRENT_EPOCH)?;
    let inc = cache.increment(UFix64::new(1), CURRENT_EPOCH);
    cache.set(UFix64::new(u64::MIN), CURRENT_EPOCH)?;
    let dec = cache.decrement(UFix64::new(1), CURRENT_EPOCH);
    assert!(inc.is_err_and(|e| e == TotalSolCacheOverflow.into()));
    assert!(dec.is_err_and(|e| e == TotalSolCacheUnderflow.into()));
    Ok(())
  }
}
