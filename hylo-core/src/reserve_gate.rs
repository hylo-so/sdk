use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Reserve policy applied to collateral-to-stablecoin rebalances.
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
pub struct ReserveGate {
  pub reserve_per_tvl: UFixValue64,
  pub tvl_cap: UFixValue64,
}

impl Default for ReserveGate {
  fn default() -> ReserveGate {
    ReserveGate {
      reserve_per_tvl: UFixValue64::new(0, 0),
      tvl_cap: UFixValue64::new(0, 0),
    }
  }
}

impl ReserveGate {
  /// Creates a validated reserve policy.
  pub fn new(
    reserve_per_tvl: UFixValue64,
    tvl_cap: UFixValue64,
  ) -> Result<ReserveGate, CoreError> {
    let mut config = ReserveGate::default();
    config.update(reserve_per_tvl, tvl_cap)?;
    Ok(config)
  }

  /// Validates and updates the reserve policy.
  pub fn update(
    &mut self,
    reserve_per_tvl: UFixValue64,
    tvl_cap: UFixValue64,
  ) -> Result<(), CoreError> {
    let reserve_per_tvl_value: UFix64<N6> = reserve_per_tvl.try_into()?;
    let _: UFix64<N6> = tvl_cap.try_into()?;

    if reserve_per_tvl_value > UFix64::one() {
      return Err(CoreError::ReserveGateInvalid);
    }

    self.reserve_per_tvl = reserve_per_tvl;
    self.tvl_cap = tvl_cap;
    Ok(())
  }

  pub fn reserve_per_tvl(&self) -> Result<UFix64<N6>, CoreError> {
    Ok(self.reserve_per_tvl.try_into()?)
  }

  pub fn tvl_cap(&self) -> Result<UFix64<N6>, CoreError> {
    Ok(self.tvl_cap.try_into()?)
  }

  /// Computes the USDC reserve required for a collateral pair with this policy.
  pub fn required_reserve(
    &self,
    tvl: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let reserve_per_tvl = self.reserve_per_tvl()?;
    if reserve_per_tvl == UFix64::zero() {
      return Ok(UFix64::zero());
    }

    let tvl_cap = self.tvl_cap()?;
    let capped_tvl = if tvl_cap == UFix64::zero() {
      tvl
    } else {
      tvl.min(tvl_cap)
    };
    capped_tvl
      .mul_floor(reserve_per_tvl)
      .ok_or(CoreError::InsufficientLiquidity)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn caps_tvl_before_computing_reserve() -> Result<(), CoreError> {
    let config = ReserveGate::new(
      UFixValue64::new(50_000, -6),
      UFixValue64::new(10_000_000_000_000, -6),
    )?;
    let tvl = UFix64::<N6>::new(50_000_000_000_000);

    assert_eq!(
      config.required_reserve(tvl)?,
      UFix64::<N6>::new(500_000_000_000),
    );
    Ok(())
  }

  #[test]
  fn rejects_a_reserve_ratio_above_one() {
    assert_eq!(
      ReserveGate::new(
        UFixValue64::new(1_000_001, -6),
        UFixValue64::new(0, -6),
      ),
      Err(CoreError::ReserveGateInvalid),
    );
  }
}
