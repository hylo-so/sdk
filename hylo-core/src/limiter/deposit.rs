use anchor_lang::prelude::*;
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError::{
  DepositLimitArithmetic, DepositLimitExceeded, DepositLimitValidation,
};

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
pub struct DepositLimiter {
  pub limit: UFixValue64,
}

impl DepositLimiter {
  /// Creates a limiter with the given raw deposit limit.
  #[must_use]
  pub fn new(limit: UFixValue64) -> DepositLimiter {
    DepositLimiter { limit }
  }

  /// Sets new deposit limit.
  ///
  /// # Errors
  /// * Numeric conversion
  /// * New limit less than or equal to current pool amount
  pub fn update_limit(
    &mut self,
    pool_amount: UFix64<N6>,
    new_limit_raw: UFixValue64,
  ) -> Result<()> {
    let new_limit: UFix64<N6> = new_limit_raw.try_into()?;
    if new_limit >= pool_amount {
      self.limit = new_limit_raw;
      Ok(())
    } else {
      Err(DepositLimitValidation.into())
    }
  }

  /// Converts configured limit to `UFix64`.
  ///
  /// # Errors
  /// * Numeric conversion
  pub fn limit(&self) -> Result<UFix64<N6>> {
    self.limit.try_into()
  }

  /// Validates incoming deposit against limit.
  ///
  /// # Errors
  /// * Numeric conversion
  /// * Deposit takes pool over limit
  pub fn validate_deposit(
    &self,
    pool_amount: UFix64<N6>,
    deposit: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    let limit = self.limit()?;
    let new_pool_amount = pool_amount
      .checked_add(&deposit)
      .ok_or(DepositLimitArithmetic)?;
    if new_pool_amount <= limit {
      Ok(deposit)
    } else {
      Err(DepositLimitExceeded.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::CoreError::{
    DepositLimitArithmetic, DepositLimitExceeded, DepositLimitValidation,
  };

  fn limiter() -> DepositLimiter {
    DepositLimiter::new(UFixValue64::new(1_000_000, -6))
  }

  #[test]
  fn update_limit_above_pool() -> Result<()> {
    let mut limiter = limiter();
    let pool = UFix64::constant(500_000);
    let new_limit = UFixValue64::new(2_000_000, -6);
    limiter.update_limit(pool, new_limit)?;
    assert_eq!(limiter.limit()?, UFix64::constant(2_000_000));
    Ok(())
  }

  #[test]
  fn reject_update_below_pool() {
    let mut limiter = limiter();
    let pool = UFix64::constant(1_000_000);
    let new_limit = UFixValue64::new(500_000, -6);
    let result = limiter.update_limit(pool, new_limit);
    assert_eq!(result.err(), Some(DepositLimitValidation.into()));
  }

  #[test]
  fn reject_update_wrong_exp() {
    let mut limiter = limiter();
    let pool = UFix64::constant(500_000);
    let new_limit = UFixValue64::new(2_000_000, -9);
    assert!(limiter.update_limit(pool, new_limit).is_err());
  }

  #[test]
  fn accept_deposit_under_limit() -> Result<()> {
    let pool = UFix64::constant(400_000);
    let deposit = UFix64::constant(500_000);
    assert_eq!(limiter().validate_deposit(pool, deposit)?, deposit);
    Ok(())
  }

  #[test]
  fn reject_deposit_over_limit() {
    let pool = UFix64::constant(400_000);
    let deposit = UFix64::constant(600_001);
    let result = limiter().validate_deposit(pool, deposit);
    assert_eq!(result.err(), Some(DepositLimitExceeded.into()));
  }

  #[test]
  fn reject_deposit_on_overflow() {
    let pool = UFix64::constant(u64::MAX);
    let deposit = UFix64::constant(1);
    let result = limiter().validate_deposit(pool, deposit);
    assert_eq!(result.err(), Some(DepositLimitArithmetic.into()));
  }
}
