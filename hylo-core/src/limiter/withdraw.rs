use std::cmp::Ordering;

use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
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
  #[cfg(any(test, feature = "offchain"))]
  #[must_use]
  pub fn new(
    limit: UFixValue64,
    withdrawal_ledger: VirtualStablecoin,
    epoch: u64,
  ) -> WithdrawalLimiter {
    WithdrawalLimiter {
      limit,
      withdrawal_ledger,
      epoch,
    }
  }

  /// Converts configured limit to `UFix64`.
  ///
  /// # Errors
  /// * Numeric conversion
  pub fn limit(&self) -> Result<UFix64<N6>, CoreError> {
    Ok(self.limit.try_into()?)
  }

  /// Sets new withdrawal limit, resetting ledger and epoch.
  ///
  /// # Errors
  /// * Numeric conversion
  /// * New limit zero
  /// * Ledger epoch greater than current epoch
  pub fn update_limit(
    &mut self,
    new_limit_raw: UFixValue64,
    current_epoch: u64,
  ) -> Result<(), CoreError> {
    let new_limit: UFix64<N6> = new_limit_raw.try_into()?;
    if current_epoch < self.epoch {
      Err(WithdrawalLimitInvalidEpoch)
    } else if new_limit > UFix64::zero() {
      self.limit = new_limit_raw;
      self.withdrawal_ledger = VirtualStablecoin::new();
      self.epoch = current_epoch;
      Ok(())
    } else {
      Err(WithdrawalLimitValidation)
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
  ) -> Result<(), CoreError> {
    let validated = self.validate_withdrawal(withdrawal, current_epoch)?;
    let mut ledger = self.epoch_ledger(current_epoch)?;
    ledger.mint(validated)?;
    self.withdrawal_ledger = ledger;
    self.epoch = current_epoch;
    Ok(())
  }

  /// Validates a withdrawal against the limit for `current_epoch`,
  /// returning it on success. The ledger resets on epoch rollover.
  ///
  /// # Errors
  /// * Numeric conversion
  /// * Overflow while totaling withdrawals
  /// * Withdrawal exceeds limit for epoch
  /// * Ledger epoch greater than current epoch
  pub fn validate_withdrawal(
    &self,
    withdrawal: UFix64<N6>,
    current_epoch: u64,
  ) -> Result<UFix64<N6>, CoreError> {
    let ledger_total = self.epoch_ledger(current_epoch)?.supply()?;
    let projected = ledger_total
      .checked_add(&withdrawal)
      .ok_or(WithdrawalLimitArithmetic)?;
    if projected <= self.limit()? {
      Ok(withdrawal)
    } else {
      Err(WithdrawalLimitExceededForEpoch)
    }
  }

  /// Largest withdrawal the limit admits for `current_epoch`.
  ///
  /// # Errors
  /// * Numeric conversion
  /// * Ledger epoch greater than current epoch
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_withdrawal(
    &self,
    current_epoch: u64,
  ) -> Result<UFix64<N6>, CoreError> {
    let ledger_total = self.epoch_ledger(current_epoch)?.supply()?;
    Ok(
      self
        .limit()?
        .checked_sub(&ledger_total)
        .unwrap_or(UFix64::zero()),
    )
  }

  fn epoch_ledger(
    &self,
    current_epoch: u64,
  ) -> Result<VirtualStablecoin, CoreError> {
    match current_epoch.cmp(&self.epoch) {
      Ordering::Less => Err(WithdrawalLimitInvalidEpoch),
      Ordering::Equal => Ok(self.withdrawal_ledger),
      Ordering::Greater => Ok(VirtualStablecoin::new()),
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
    WithdrawalLimiter::new(
      UFixValue64::new(1_000_000, -6),
      VirtualStablecoin::new(),
      EPOCH,
    )
  }

  #[test]
  fn update_limit_nonzero() -> Result<(), CoreError> {
    let mut limiter = limiter();
    let new_limit = UFixValue64::new(2_000_000, -6);
    limiter.update_limit(new_limit, EPOCH)?;
    assert_eq!(limiter.limit()?, UFix64::constant(2_000_000));
    Ok(())
  }

  #[test]
  fn reject_update_zero_limit() {
    let mut limiter = limiter();
    let result = limiter.update_limit(UFixValue64::new(0, -6), EPOCH);
    assert_eq!(result.err(), Some(WithdrawalLimitValidation));
  }

  #[test]
  fn update_limit_resets_ledger_and_epoch() -> Result<(), CoreError> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(1_000_000), EPOCH)?;
    limiter.update_limit(UFixValue64::new(1_000_000, -6), EPOCH)?;
    assert_eq!(limiter.epoch, EPOCH);
    assert_eq!(limiter.withdrawal_ledger.supply()?, UFix64::zero());
    limiter.register_withdrawal(UFix64::constant(1_000_000), EPOCH)?;
    Ok(())
  }

  #[test]
  fn accept_withdrawal_under_limit() -> Result<(), CoreError> {
    let mut limiter = limiter();
    let amount = UFix64::constant(400_000);
    limiter.register_withdrawal(amount, EPOCH)?;
    assert_eq!(limiter.withdrawal_ledger.supply()?, amount);
    Ok(())
  }

  #[test]
  fn accept_cumulative_at_limit() -> Result<(), CoreError> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(400_000), EPOCH)?;
    limiter.register_withdrawal(UFix64::constant(600_000), EPOCH)?;
    assert_eq!(limiter.withdrawal_ledger.supply()?, limiter.limit()?);
    Ok(())
  }

  #[test]
  fn reject_cumulative_over_limit() -> Result<(), CoreError> {
    let mut limiter = limiter();
    let amount = UFix64::constant(400_000);
    limiter.register_withdrawal(amount, EPOCH)?;
    let result = limiter.register_withdrawal(UFix64::constant(600_001), EPOCH);
    assert_eq!(result.err(), Some(WithdrawalLimitExceededForEpoch));
    assert_eq!(limiter.withdrawal_ledger.supply()?, amount);
    Ok(())
  }

  #[test]
  fn reject_on_ledger_overflow() -> Result<(), CoreError> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(1), EPOCH)?;
    let result = limiter.register_withdrawal(UFix64::constant(u64::MAX), EPOCH);
    assert_eq!(result.err(), Some(WithdrawalLimitArithmetic));
    Ok(())
  }

  #[test]
  fn rollover_resets_ledger() -> Result<(), CoreError> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(1_000_000), EPOCH)?;
    let amount = UFix64::constant(700_000);
    limiter.register_withdrawal(amount, EPOCH + 1)?;
    assert_eq!(limiter.epoch, EPOCH + 1);
    assert_eq!(limiter.withdrawal_ledger.supply()?, amount);
    Ok(())
  }

  #[test]
  fn reject_rollover_over_limit() -> Result<(), CoreError> {
    let mut limiter = limiter();
    limiter.register_withdrawal(UFix64::constant(500_000), EPOCH)?;
    let result =
      limiter.register_withdrawal(UFix64::constant(1_000_001), EPOCH + 1);
    assert_eq!(result.err(), Some(WithdrawalLimitExceededForEpoch));
    Ok(())
  }

  #[test]
  fn reject_update_stale_epoch() {
    let mut limiter = limiter();
    let result =
      limiter.update_limit(UFixValue64::new(2_000_000, -6), EPOCH - 1);
    assert_eq!(result.err(), Some(WithdrawalLimitInvalidEpoch));
  }

  #[test]
  fn reject_stale_epoch() {
    let mut limiter = limiter();
    let result = limiter.register_withdrawal(UFix64::constant(1), EPOCH - 1);
    assert_eq!(result.err(), Some(WithdrawalLimitInvalidEpoch));
  }
}
