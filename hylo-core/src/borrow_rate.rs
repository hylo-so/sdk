use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use fix::typenum::Z0;
use serde::{Deserialize, Serialize};

use crate::collateral_ratio::CollateralRatio;
use crate::error::CoreError;
use crate::error::CoreError::{
  BorrowRateApply, BorrowRateValidation, InterpFeeConversion,
};
use crate::fees::interp::{FixInterp, Point};
use crate::rebalance::mode::RebalanceMode;
use crate::rebalance::pricing::narrow;

/// Per-epoch borrow rate for exogenous collateral without native yield.
#[derive(
  Copy,
  Clone,
  Debug,
  PartialEq,
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Serialize,
  Deserialize,
)]
pub struct BorrowRateCurveConfig {
  pub floor_rate: UFixValue64,
  pub ceil_rate: UFixValue64,
}

/// Maximum per-epoch rate (~30% annualized at 182 epochs/year)
const MAX_RATE: UFix64<N9> = UFix64::constant(1_648_352);

/// Maximum fee exacted against borrow rate
const MAX_FEE: UFix64<N4> = UFix64::constant(1_000);

impl BorrowRateCurveConfig {
  #[must_use]
  pub fn new(
    floor_rate: UFixValue64,
    ceil_rate: UFixValue64,
  ) -> BorrowRateCurveConfig {
    BorrowRateCurveConfig {
      floor_rate,
      ceil_rate,
    }
  }

  /// Minimum borrow rate.
  ///
  /// # Errors
  /// * Invalid rate data
  pub fn floor_rate(&self) -> Result<UFix64<N9>, CoreError> {
    Ok(self.floor_rate.try_into()?)
  }

  /// Maximum borrow rate.
  ///
  /// # Errors
  /// * Invalid rate data
  pub fn ceil_rate(&self) -> Result<UFix64<N9>, CoreError> {
    Ok(self.ceil_rate.try_into()?)
  }

  /// Rate curve over CR: constant floor through the neutral zone,
  /// linear from floor to ceiling across buy zone 1.
  ///
  /// # Errors
  /// * Conversion or curve construction
  fn build_curve(&self) -> Result<FixInterp<3, N9>, CoreError> {
    let neutral_start = RebalanceMode::Neutral
      .active_range()
      .start()
      .and_then(narrow)?;
    let buy_zone_1_start = RebalanceMode::BuyZone1
      .active_range()
      .start()
      .and_then(narrow)?;
    let buy_zone_1_end = RebalanceMode::BuyZone1
      .active_range()
      .end()
      .and_then(narrow)?;
    let floor_rate = self.floor_rate().and_then(narrow)?;
    let ceil_rate = self.ceil_rate().and_then(narrow)?;
    FixInterp::from_points([
      Point::new(neutral_start, floor_rate),
      Point::new(buy_zone_1_start, floor_rate),
      Point::new(buy_zone_1_end, ceil_rate),
    ])
  }

  /// Per-epoch borrow rate at the given CR.
  /// Constant at the ceiling above the curve domain, including
  /// [`CollateralRatio::Infinite`].
  ///
  /// # Errors
  /// * CR below the curve domain
  /// * Conversion or arithmetic
  pub fn rate(&self, cr: CollateralRatio) -> Result<UFix64<N9>, CoreError> {
    let interp = self.build_curve()?;
    let x = cr.price_curve_x();
    let rate = if x > interp.x_max() {
      interp.y_max()
    } else {
      interp.interpolate(x)?
    };
    rate.narrow().ok_or(InterpFeeConversion)
  }

  /// Applies the borrow rate at the given CR to an amount.
  /// Multiplies by elapsed epochs to cover missed harvests.
  ///
  /// # Errors
  /// * CR below the curve domain
  /// * Arithmetic overflow
  pub fn apply_borrow_rate(
    &self,
    amount: UFix64<N9>,
    cr: CollateralRatio,
    elapsed_epochs: UFix64<Z0>,
  ) -> Result<UFix64<N9>, CoreError> {
    let rate = self.rate(cr)?;
    amount
      .mul_div_floor(rate, UFix64::one())
      .and_then(|base| base.checked_mul(&elapsed_epochs))
      .ok_or(BorrowRateApply)
  }

  /// Floor and ceiling must satisfy `0 < floor <= ceil <= MAX_RATE`.
  ///
  /// # Errors
  /// * Floor is zero or exceeds ceiling
  /// * Ceiling exceeds maximum rate
  pub fn validate(&self) -> Result<BorrowRateCurveConfig, CoreError> {
    let floor = self.floor_rate()?;
    let ceil = self.ceil_rate()?;
    (floor > UFix64::zero() && floor <= ceil && ceil <= MAX_RATE)
      .then_some(*self)
      .ok_or(BorrowRateValidation)
  }
}

/// Borrow rate fee must be in `(0, MAX_FEE]`.
///
/// # Errors
/// * Fee is zero or exceeds maximum
pub fn validate_borrow_rate_fee(
  fee: UFixValue64,
) -> Result<UFixValue64, CoreError> {
  let bps: UFix64<N4> = fee.try_into()?;
  (bps > UFix64::zero() && bps <= MAX_FEE)
    .then_some(fee)
    .ok_or(BorrowRateValidation)
}

#[cfg(test)]
mod tests {
  use more_asserts::{assert_gt, assert_lt};

  use super::*;
  use crate::collateral_ratio::CR;

  #[test]
  fn validate_fee_pos() -> Result<(), CoreError> {
    validate_borrow_rate_fee(UFixValue64::new(1, -4))?;
    validate_borrow_rate_fee(UFixValue64::new(1_000, -4))?;
    Ok(())
  }

  #[test]
  fn validate_fee_neg_zero() {
    assert_eq!(
      validate_borrow_rate_fee(UFixValue64::new(0, -4)),
      Err(BorrowRateValidation)
    );
  }

  #[test]
  fn validate_fee_neg_high() {
    assert_eq!(
      validate_borrow_rate_fee(UFixValue64::new(1_001, -4)),
      Err(BorrowRateValidation)
    );
  }

  const FLOOR: UFix64<N9> = UFix64::constant(384_620);
  const CEIL: UFix64<N9> = MAX_RATE;

  fn config(floor: UFix64<N9>, ceil: UFix64<N9>) -> BorrowRateCurveConfig {
    BorrowRateCurveConfig::new(floor.into(), ceil.into())
  }

  fn test_config() -> BorrowRateCurveConfig {
    config(FLOOR, CEIL)
  }

  #[test]
  fn rate_neg_below_neutral() -> Result<(), CoreError> {
    let neutral_start = RebalanceMode::Neutral.active_range().start()?;
    let below = CR::finite(UFix64::new(neutral_start.bits - 1));
    assert_eq!(test_config().rate(below), Err(CoreError::InterpOutOfDomain));
    Ok(())
  }

  #[test]
  fn rate_floor_through_neutral() -> Result<(), CoreError> {
    let config = test_config();
    let neutral = RebalanceMode::Neutral.active_range();
    let neutral_mid = CR::finite(UFix64::constant(1_500_000_000));
    assert_eq!(config.rate(CR::finite(neutral.start()?))?, FLOOR);
    assert_eq!(config.rate(neutral_mid)?, FLOOR);
    assert_eq!(config.rate(CR::finite(neutral.end()?))?, FLOOR);
    Ok(())
  }

  #[test]
  fn rate_ramp_within_bounds() -> Result<(), CoreError> {
    let buy_zone_1_start = RebalanceMode::BuyZone1.active_range().start()?;
    let inside = CR::finite(UFix64::new(buy_zone_1_start.bits + 1));
    let rate = test_config().rate(inside)?;
    assert_gt!(rate, FLOOR);
    assert_lt!(rate, CEIL);
    Ok(())
  }

  #[test]
  fn rate_ceil_saturates() -> Result<(), CoreError> {
    let config = test_config();
    let buy_zone_1_end = RebalanceMode::BuyZone1.active_range().end()?;
    let above = CR::finite(UFix64::new(buy_zone_1_end.bits + 1));
    assert_eq!(config.rate(CR::finite(buy_zone_1_end))?, CEIL);
    assert_eq!(config.rate(above)?, CEIL);
    Ok(())
  }

  #[test]
  fn rate_ceil_at_infinite_cr() -> Result<(), CoreError> {
    assert_eq!(test_config().rate(CollateralRatio::Infinite)?, CEIL);
    Ok(())
  }

  #[test]
  fn apply_borrow_rate_7_percent_annual() -> Result<(), CoreError> {
    let neutral_start = RebalanceMode::Neutral.active_range().start()?;
    let collateral = UFix64::<N9>::new(1_000_000_000_000_000);
    let borrow = test_config().apply_borrow_rate(
      collateral,
      CR::finite(neutral_start),
      UFix64::constant(1),
    )?;
    assert_eq!(borrow, UFix64::new(384_620_000_000));
    Ok(())
  }

  #[test]
  fn apply_borrow_rate_multiple_epochs() -> Result<(), CoreError> {
    let neutral_start = RebalanceMode::Neutral.active_range().start()?;
    let collateral = UFix64::<N9>::new(1_234_567_890_123_456);
    let borrow = test_config().apply_borrow_rate(
      collateral,
      CR::finite(neutral_start),
      UFix64::constant(5),
    )?;
    assert_eq!(borrow, UFix64::new(2_374_197_509_495));
    Ok(())
  }

  #[test]
  fn validate_pos() -> Result<(), CoreError> {
    config(FLOOR, CEIL).validate()?;
    config(FLOOR, FLOOR).validate()?;
    Ok(())
  }

  #[test]
  fn validate_neg_zero_floor() {
    assert_eq!(
      config(UFix64::zero(), CEIL).validate(),
      Err(BorrowRateValidation)
    );
  }

  #[test]
  fn validate_neg_floor_above_ceil() {
    let above = UFix64::new(FLOOR.bits + 1);
    assert_eq!(config(above, FLOOR).validate(), Err(BorrowRateValidation));
  }

  #[test]
  fn validate_neg_high_ceil() {
    let above = UFix64::new(CEIL.bits + 1);
    assert_eq!(config(FLOOR, above).validate(), Err(BorrowRateValidation));
  }
}
