use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
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
  #[cfg(any(test, feature = "offchain"))]
  #[must_use]
  pub fn new(limit: UFixValue64) -> DepositLimiter {
    DepositLimiter { limit }
  }

  /// Sets new deposit limit.
  ///
  /// # Errors
  /// * Numeric conversion
  /// * New limit less than current pool amount
  pub fn update_limit(
    &mut self,
    pool_amount: UFix64<N6>,
    new_limit_raw: UFixValue64,
  ) -> Result<(), CoreError> {
    let new_limit: UFix64<N6> = new_limit_raw.try_into()?;
    if new_limit >= pool_amount {
      self.limit = new_limit_raw;
      Ok(())
    } else {
      Err(DepositLimitValidation)
    }
  }

  /// Converts configured limit to `UFix64`.
  ///
  /// # Errors
  /// * Numeric conversion
  pub fn limit(&self) -> Result<UFix64<N6>, CoreError> {
    Ok(self.limit.try_into()?)
  }

  /// Remaining deposit headroom at the current pool balance.
  ///
  /// # Errors
  /// * Numeric conversion
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_deposit(
    &self,
    pool_amount: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let max = self.limit()?.checked_sub(&pool_amount);
    Ok(max.unwrap_or_default())
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
  ) -> Result<UFix64<N6>, CoreError> {
    let limit = self.limit()?;
    let new_pool_amount = pool_amount
      .checked_add(&deposit)
      .ok_or(DepositLimitArithmetic)?;
    if new_pool_amount <= limit {
      Ok(deposit)
    } else {
      Err(DepositLimitExceeded)
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
  fn update_limit_above_pool() -> Result<(), CoreError> {
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
    assert_eq!(result.err(), Some(DepositLimitValidation));
  }

  #[test]
  fn reject_update_wrong_exp() {
    let mut limiter = limiter();
    let pool = UFix64::constant(500_000);
    let new_limit = UFixValue64::new(2_000_000, -9);
    assert!(limiter.update_limit(pool, new_limit).is_err());
  }

  #[test]
  fn accept_deposit_under_limit() -> Result<(), CoreError> {
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
    assert_eq!(result.err(), Some(DepositLimitExceeded));
  }

  #[test]
  fn reject_deposit_on_overflow() {
    let pool = UFix64::constant(u64::MAX);
    let deposit = UFix64::constant(1);
    let result = limiter().validate_deposit(pool, deposit);
    assert_eq!(result.err(), Some(DepositLimitArithmetic));
  }
}
