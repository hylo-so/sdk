use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::borrow_rate::{buy_zone_curve, saturating_apply_curve};
use crate::collateral_ratio::CollateralRatio;
use crate::error::CoreError;
use crate::error::CoreError::{
  YieldHarvestConfigValidation, YieldHarvestMultiplier,
};
use crate::fees::controller::FeeExtract;

/// 1000 bps (10%)
const MAX_FEE: UFix64<N4> = UFix64::constant(1000);

/// 5x
const MAX_CEIL_MULT: UFix64<N9> = UFix64::constant(5_000_000_000);

/// Captures yield harvest configuration as two basis point values:
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
pub struct YieldHarvestConfig {
  pub ceil_mult: UFixValue64,
  pub fee: UFixValue64,
}

impl YieldHarvestConfig {
  pub fn init(
    &mut self,
    ceil_mult: UFixValue64,
    fee: UFixValue64,
  ) -> Result<(), CoreError> {
    self.ceil_mult = ceil_mult;
    self.fee = fee;
    Ok(())
  }

  /// Multiplier through the neutral zone.
  #[must_use]
  pub const fn floor_mult(&self) -> UFix64<N9> {
    UFix64::one()
  }

  /// Multiplier at and above the end of buy zone 1.
  ///
  /// # Errors
  /// * Invalid multiplier data
  pub fn ceil_mult(&self) -> Result<UFix64<N9>, CoreError> {
    Ok(self.ceil_mult.try_into()?)
  }

  /// Percentage of harvest allocation to divert to treasury
  pub fn fee(&self) -> Result<UFix64<N4>, CoreError> {
    Ok(self.fee.try_into()?)
  }

  /// Applies configuration to the given amount of stablecoin to harvest.
  pub fn apply_fee(
    &self,
    stablecoin: UFix64<N6>,
  ) -> Result<FeeExtract<N6>, CoreError> {
    let fee = self.fee()?;
    let extract = FeeExtract::new(fee, stablecoin)?;
    Ok(extract)
  }

  /// Multiplier at the given CR.
  ///
  /// # Errors
  /// * CR below the curve domain
  /// * Conversion or arithmetic
  pub fn multiple(&self, cr: CollateralRatio) -> Result<UFix64<N9>, CoreError> {
    let curve = buy_zone_curve(self.floor_mult(), self.ceil_mult()?)?;
    saturating_apply_curve(&curve, cr)
  }

  /// Scales `amount` by the multiplier at the given CR.
  ///
  /// # Errors
  /// * CR below the curve domain
  /// * Arithmetic overflow
  pub fn apply_multiple(
    &self,
    amount: UFix64<N9>,
    cr: CollateralRatio,
  ) -> Result<UFix64<N9>, CoreError> {
    let multiple = self.multiple(cr)?;
    amount
      .mul_div_floor(multiple, UFix64::one())
      .ok_or(YieldHarvestMultiplier)
  }

  /// Ensures `fee <= MAX_FEE` and `ceil_mult` in `[1, MAX_CEIL_MULT]`.
  ///
  /// # Errors
  /// * Bound violation
  pub fn validate(&self) -> Result<YieldHarvestConfig, CoreError> {
    let fee: UFix64<N4> = self.fee.try_into()?;
    let ceil_mult = self.ceil_mult()?;
    (fee <= MAX_FEE && (UFix64::one()..=MAX_CEIL_MULT).contains(&ceil_mult))
      .then_some(*self)
      .ok_or(YieldHarvestConfigValidation)
  }
}

/// Records epoch harvest information for off-chain consumers.
#[derive(
  Copy,
  Clone,
  Debug,
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Serialize,
  Deserialize,
)]
pub struct HarvestCache {
  pub epoch: u64,
  pub stability_pool_cap: UFixValue64,
  pub stablecoin_to_pool: UFixValue64,
}

impl HarvestCache {
  pub fn init(&mut self, epoch: u64) -> Result<(), CoreError> {
    self.epoch = epoch;
    self.stability_pool_cap = UFix64::<N6>::zero().into();
    self.stablecoin_to_pool = UFix64::<N6>::zero().into();
    Ok(())
  }

  pub fn update(
    &mut self,
    stability_pool_cap: UFix64<N6>,
    stablecoin_to_pool: UFix64<N6>,
    epoch: u64,
  ) -> Result<(), CoreError> {
    self.epoch = epoch;
    self.stability_pool_cap = stability_pool_cap.into();
    self.stablecoin_to_pool = stablecoin_to_pool.into();
    Ok(())
  }

  /// Returns true if the cache is stale (harvest needed for current epoch).
  #[must_use]
  pub fn is_stale(&self, current_epoch: u64) -> bool {
    self.epoch < current_epoch
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::collateral_ratio::CR;
  use crate::rebalance::mode::RebalanceMode;

  const ONE: UFix64<N9> = UFix64::constant(1_000_000_000);
  const TWO: UFix64<N9> = UFix64::constant(2_000_000_000);
  const FEE: UFix64<N4> = UFix64::constant(500);

  fn config(ceil_mult: UFix64<N9>) -> YieldHarvestConfig {
    YieldHarvestConfig {
      ceil_mult: ceil_mult.into(),
      fee: FEE.into(),
    }
  }

  #[test]
  fn validate_pos() -> Result<(), CoreError> {
    config(ONE).validate()?;
    config(MAX_CEIL_MULT).validate()?;
    Ok(())
  }

  #[test]
  fn validate_neg_bounds() {
    let below = UFix64::new(ONE.bits - 1);
    let above = UFix64::new(MAX_CEIL_MULT.bits + 1);
    assert_eq!(config(below).validate(), Err(YieldHarvestConfigValidation));
    assert_eq!(config(above).validate(), Err(YieldHarvestConfigValidation));
  }

  #[test]
  fn multiple_shape() -> Result<(), CoreError> {
    let config = config(TWO);
    let neutral_start = RebalanceMode::Neutral.active_range().start()?;
    let buy_zone_1_end = RebalanceMode::BuyZone1.active_range().end()?;
    let below = CR::Finite(UFix64::new(neutral_start.bits - 1));
    assert_eq!(config.multiple(below), Err(CoreError::InterpOutOfDomain));
    assert_eq!(config.multiple(CR::Finite(neutral_start))?, ONE);
    assert_eq!(config.multiple(CR::Finite(buy_zone_1_end))?, TWO);
    assert_eq!(config.multiple(CR::Infinite)?, TWO);
    Ok(())
  }

  #[test]
  fn apply_multiple_at_ceil() -> Result<(), CoreError> {
    let sol_in = UFix64::<N9>::new(1_234_567_890_123);
    let sol_out = config(TWO).apply_multiple(sol_in, CR::Infinite)?;
    assert_eq!(sol_out, UFix64::new(2_469_135_780_246));
    Ok(())
  }
}
