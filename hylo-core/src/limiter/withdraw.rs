use std::cmp::Ordering;

use anchor_lang::prelude::*;
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError::{
  WithdrawalLimitArithmetic, WithdrawalLimitExceededForEpoch,
  WithdrawalLimitInvalidEpoch, WithdrawalLimitValidation,
};
use crate::virtual_stablecoin::VirtualStablecoin;

/// Per-epoch withdrawal window, reset lazily on epoch rollover.
#[derive(
  Debug,
  Clone,
  Copy,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  Serialize,
  Deserialize,
  PartialEq,
  Eq,
)]
pub struct WithdrawalLimiter {
  pub limit: UFixValue64,
  withdrawal_ledger: VirtualStablecoin,
  epoch: u64,
}

impl WithdrawalLimiter {
  /// Creates a limiter with the given raw withdrawal limit.
  #[must_use]
  pub fn new(limit: UFixValue64, epoch: u64) -> WithdrawalLimiter {
    WithdrawalLimiter {
      limit,
      withdrawal_ledger: VirtualStablecoin::new(),
      epoch,
    }
  }

  /// Converts configured limit to `UFix64`.
  ///
  /// # Errors
  /// * Numeric conversion
  pub fn limit(&self) -> Result<UFix64<N6>> {
    self.limit.try_into()
  }

  /// Sets new withdrawal limit, resetting ledger and epoch.
  ///
  /// # Errors
  /// * Numeric conversion
  /// * New limit zero or greater than current pool amount
  pub fn update_limit(
    &mut self,
    pool_amount: UFix64<N6>,
    new_limit_raw: UFixValue64,
    current_epoch: u64,
  ) -> Result<()> {
    let new_limit: UFix64<N6> = new_limit_raw.try_into()?;
    if new_limit > UFix64::zero() && new_limit <= pool_amount {
      self.limit = new_limit_raw;
      self.withdrawal_ledger = VirtualStablecoin::new();
      self.epoch = current_epoch;
      Ok(())
    } else {
      Err(WithdrawalLimitValidation.into())
    }
  }

  /// Registers a withdrawal against the ledger, resetting it on epoch
  /// rollover.
  ///
  /// # Errors
  /// * Numeric conversion
  /// * Overflow while totaling withdrawals
  /// * Withdrawal exceeds limit for epoch
  /// * Ledger epoch greater than current epoch
  pub fn register_withdrawal(
    &mut self,
    withdrawal: UFix64<N6>,
    current_epoch: u64,
  ) -> Result<()> {
    match current_epoch.cmp(&self.epoch) {
      Ordering::Less => Err(WithdrawalLimitInvalidEpoch.into()),
      Ordering::Equal => self.register_within_limit(withdrawal),
      Ordering::Greater => {
        self.epoch = current_epoch;
        self.withdrawal_ledger = VirtualStablecoin::new();
        self.register_within_limit(withdrawal)
      }
    }
  }

  /// Adds withdrawal to ledger if projected total stays within limit.
  fn register_within_limit(&mut self, withdrawal: UFix64<N6>) -> Result<()> {
    let projected = self
      .withdrawal_ledger
      .supply()?
      .checked_add(&withdrawal)
      .ok_or(WithdrawalLimitArithmetic)?;
    if projected <= self.limit()? {
      self.withdrawal_ledger.mint(withdrawal)
    } else {
      Err(WithdrawalLimitExceededForEpoch.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::CoreError::{
    WithdrawalLimitArithmetic, WithdrawalLimitExceededForEpoch,
    WithdrawalLimitInvalidEpoch, WithdrawalLimitValidation,
  };

  const EPOCH: u64 = 10;

  fn limiter() -> WithdrawalLimiter {
    WithdrawalLimiter::new(UFixValue64::new(1_000_000, -6), EPOCH)
  }

  #[test]
  fn update_limit_within_pool() -> Result<()> {
    let mut limiter = limiter();
    let pool = UFix64::constant(3_000_000);
    let new_limit = UFixValue64::new(2_000_000, -6);
    limiter.update_limit(pool, new_limit, EPOCH)?;
    assert_eq!(limiter.limit()?, UFix64::constant(2_000_000));
    Ok(())
  }

  #[test]
  fn reject_update_zero_limit() {
    let mut limiter = limiter();
    let pool = UFix64::constant(1_000_000);
    let result = limiter.update_limit(pool, UFixValue64::new(0, -6), EPOCH);
    assert_eq!(result.err(), Some(WithdrawalLimitValidation.into()));
  }

  #[test]
  fn reject_update_above_pool() {
    let mut limiter = limiter();
    let pool = UFix64::constant(1_000_000);
    let new_limit = UFixValue64::new(1_000_001, -6);
    let result = limiter.update_limit(pool, new_limit, EPOCH);
    assert_eq!(result.err(), Some(WithdrawalLimitValidation.into()));
  }

  #[test]
  fn update_limit_resets_ledger_and_epoch() -> Result<()> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(1_000_000), EPOCH)?;
    let pool = UFix64::constant(2_000_000);
    limiter.update_limit(pool, UFixValue64::new(1_000_000, -6), EPOCH)?;
    assert_eq!(limiter.epoch, EPOCH);
    assert_eq!(limiter.withdrawal_ledger.supply()?, UFix64::zero());
    limiter.register_withdrawal(UFix64::constant(1_000_000), EPOCH)?;
    Ok(())
  }

  #[test]
  fn accept_withdrawal_under_limit() -> Result<()> {
    let mut limiter = limiter();
    let amount = UFix64::constant(400_000);
    limiter.register_withdrawal(amount, EPOCH)?;
    assert_eq!(limiter.withdrawal_ledger.supply()?, amount);
    Ok(())
  }

  #[test]
  fn accept_cumulative_at_limit() -> Result<()> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(400_000), EPOCH)?;
    limiter.register_withdrawal(UFix64::constant(600_000), EPOCH)?;
    assert_eq!(limiter.withdrawal_ledger.supply()?, limiter.limit()?);
    Ok(())
  }

  #[test]
  fn reject_cumulative_over_limit() -> Result<()> {
    let mut limiter = limiter();
    let amount = UFix64::constant(400_000);
    limiter.register_withdrawal(amount, EPOCH)?;
    let result = limiter.register_withdrawal(UFix64::constant(600_001), EPOCH);
    assert_eq!(result.err(), Some(WithdrawalLimitExceededForEpoch.into()));
    assert_eq!(limiter.withdrawal_ledger.supply()?, amount);
    Ok(())
  }

  #[test]
  fn reject_on_ledger_overflow() -> Result<()> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(1), EPOCH)?;
    let result = limiter.register_withdrawal(UFix64::constant(u64::MAX), EPOCH);
    assert_eq!(result.err(), Some(WithdrawalLimitArithmetic.into()));
    Ok(())
  }

  #[test]
  fn rollover_resets_ledger() -> Result<()> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(1_000_000), EPOCH)?;
    let amount = UFix64::constant(700_000);
    limiter.register_withdrawal(amount, EPOCH + 1)?;
    assert_eq!(limiter.epoch, EPOCH + 1);
    assert_eq!(limiter.withdrawal_ledger.supply()?, amount);
    Ok(())
  }

  #[test]
  fn reject_rollover_over_limit() -> Result<()> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(500_000), EPOCH)?;
    let result =
      limiter.register_withdrawal(UFix64::constant(1_000_001), EPOCH + 1);
    assert_eq!(result.err(), Some(WithdrawalLimitExceededForEpoch.into()));
    Ok(())
  }

  #[test]
  fn reject_stale_epoch() {
    let mut limiter = limiter();
    let result = limiter.register_withdrawal(UFix64::constant(1), EPOCH - 1);
    assert_eq!(result.err(), Some(WithdrawalLimitInvalidEpoch.into()));
  }
}
