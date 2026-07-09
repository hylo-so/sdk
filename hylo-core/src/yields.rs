use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::error::CoreError::{
  YieldHarvestAllocation, YieldHarvestConfigValidation,
};
use crate::fees::controller::FeeExtract;

/// 1000 bps (10%)
const MAX_FEE: UFix64<N4> = UFix64::constant(1000);

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
  pub allocation: UFixValue64,
  pub fee: UFixValue64,
}

impl YieldHarvestConfig {
  pub fn init(
    &mut self,
    allocation: UFixValue64,
    fee: UFixValue64,
  ) -> Result<(), CoreError> {
    self.allocation = allocation;
    self.fee = fee;
    Ok(())
  }

  /// Percentage of accrued yield to qualify for harvest
  pub fn allocation(&self) -> Result<UFix64<N4>, CoreError> {
    Ok(self.allocation.try_into()?)
  }

  /// Percentage of harvest allocation to divert to treasury
  pub fn fee(&self) -> Result<UFix64<N4>, CoreError> {
    Ok(self.fee.try_into()?)
  }

  /// Multiplies allocation bps by amount of harvestable stablecoin.
  pub fn apply_allocation(
    &self,
    stablecoin: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let allocation = self.allocation()?;
    stablecoin
      .mul_div_floor(allocation, UFix64::one())
      .ok_or(YieldHarvestAllocation)
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

  fn is_valid(fee: UFix64<N4>, allocation: UFix64<N4>) -> bool {
    let fee_valid = (UFix64::new(1)..=MAX_FEE).contains(&fee);
    let allocation_valid =
      (UFix64::new(1)..=UFix64::one()).contains(&allocation);
    fee_valid && allocation_valid
  }

  pub fn validate(&self) -> Result<Self, CoreError> {
    let fee: UFix64<N4> = self.fee.try_into()?;
    let allocation: UFix64<N4> = self.allocation.try_into()?;
    if YieldHarvestConfig::is_valid(fee, allocation) {
      Ok(*self)
    } else {
      Err(YieldHarvestConfigValidation)
    }
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
