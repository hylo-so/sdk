use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::{
  YieldHarvestAllocation, YieldHarvestConfigValidation,
};
use crate::fee_controller::FeeExtract;

/// Captures yield harvest configuration as two basis point values:
#[derive(Copy, Clone, InitSpace, AnchorSerialize, AnchorDeserialize)]
pub struct YieldHarvestConfig {
  pub allocation: UFixValue64,
  pub fee: UFixValue64,
}

impl YieldHarvestConfig {
  pub fn init(
    &mut self,
    allocation: UFixValue64,
    fee: UFixValue64,
  ) -> Result<()> {
    self.allocation = allocation;
    self.fee = fee;
    Ok(())
  }

  /// Percentage of accrued yield to qualify for harvest
  pub fn allocation(&self) -> Result<UFix64<N4>> {
    self.allocation.try_into()
  }

  /// Percentage of harvest allocation to divert to treasury
  pub fn fee(&self) -> Result<UFix64<N4>> {
    self.fee.try_into()
  }

  /// Multiplies allocation bps by amount of harvestable stablecoin.
  pub fn apply_allocation(&self, stablecoin: UFix64<N6>) -> Result<UFix64<N6>> {
    let allocation = self.allocation()?;
    stablecoin
      .mul_div_floor(allocation, UFix64::one())
      .ok_or(YieldHarvestAllocation.into())
  }

  /// Applies configuration to the given amount of stablecoin to harvest.
  pub fn apply_fee(&self, stablecoin: UFix64<N6>) -> Result<FeeExtract<N6>> {
    let fee = self.fee()?;
    let extract = FeeExtract::new(fee, stablecoin)?;
    Ok(extract)
  }

  /// Yield harvest fee and allocation parse to bps and are less than or equal
  /// to 100%
  pub fn validate(&self) -> Result<Self> {
    let fee: UFix64<N4> = self.fee.try_into()?;
    let allocation: UFix64<N4> = self.allocation.try_into()?;
    let one = UFix64::new(10000);
    let zero = UFix64::zero();
    if fee > zero && fee <= one && allocation > zero && allocation <= one {
      Ok(*self)
    } else {
      Err(YieldHarvestConfigValidation.into())
    }
  }
}

/// Records epoch yield harvest information for off-chain consumers.
#[derive(Copy, Clone, InitSpace, AnchorSerialize, AnchorDeserialize)]
pub struct YieldHarvestCache {
  pub epoch: u64,
  pub stability_pool_cap: UFixValue64,
  pub stablecoin_yield_to_pool: UFixValue64,
}

impl YieldHarvestCache {
  pub fn init(&mut self, epoch: u64) -> Result<()> {
    self.epoch = epoch;
    self.stability_pool_cap = UFix64::<N6>::zero().into();
    self.stablecoin_yield_to_pool = UFix64::<N6>::zero().into();
    Ok(())
  }

  pub fn update(
    &mut self,
    stability_pool_cap: UFix64<N6>,
    stablecoin_yield_to_pool: UFix64<N6>,
    epoch: u64,
  ) -> Result<()> {
    self.epoch = epoch;
    self.stability_pool_cap = stability_pool_cap.into();
    self.stablecoin_yield_to_pool = stablecoin_yield_to_pool.into();
    Ok(())
  }
}
