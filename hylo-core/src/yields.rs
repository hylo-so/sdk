use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::error::CoreError::YieldHarvestConfigValidation;
use crate::fees::controller::FeeExtract;

/// 1000 bps (10%)
const MAX_FEE: UFix64<N4> = UFix64::constant(1000);

/// 5x
const MAX_CEIL_MULT: UFix64<N4> = UFix64::constant(50_000);

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

  /// Baseline multiple
  #[must_use]
  pub const fn floor_mult(&self) -> UFix64<N4> {
    UFix64::one()
  }

  /// Percentage of accrued yield to qualify for harvest
  pub fn ceil_mult(&self) -> Result<UFix64<N4>, CoreError> {
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

  /// Ensures `fee <= MAX_FEE` and `ceil_mult` in `[1, MAX_CEIL_MULT]`.
  ///
  /// # Errors
  /// * Bound violation
  pub fn validate(&self) -> Result<YieldHarvestConfig, CoreError> {
    let fee: UFix64<N4> = self.fee.try_into()?;
    let ceil_mult: UFix64<N4> = self.ceil_mult.try_into()?;
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
