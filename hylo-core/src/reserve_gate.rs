use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::error::CoreError::{InsufficientLiquidity, ReserveGateInvalid};

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

impl ReserveGate {
  fn new(reserve_per_tvl: UFixValue64, tvl_cap: UFixValue64) -> ReserveGate {
    ReserveGate {
      reserve_per_tvl,
      tvl_cap,
    }
  }

  /// Reserve ratio must be in `[0, 1]`. A zero TVL cap leaves TVL uncapped.
  pub fn validated(
    reserve_per_tvl: UFixValue64,
    tvl_cap: UFixValue64,
  ) -> Result<ReserveGate, CoreError> {
    let gate = ReserveGate::new(reserve_per_tvl, tvl_cap);
    let _ = gate.tvl_cap()?;
    (gate.reserve_per_tvl()? <= UFix64::one())
      .then_some(gate)
      .ok_or(ReserveGateInvalid)
  }

  /// Lifts serialized reserve ratio to `UFix64`.
  pub fn reserve_per_tvl(&self) -> Result<UFix64<N9>, CoreError> {
    Ok(self.reserve_per_tvl.try_into()?)
  }

  /// Lifts serialized TVL cap to `UFix64`.
  pub fn tvl_cap(&self) -> Result<UFix64<N9>, CoreError> {
    Ok(self.tvl_cap.try_into()?)
  }

  /// Clamps TVL to the cap, leaving it uncapped when the cap is zero.
  fn capped_tvl(&self, tvl: UFix64<N9>) -> Result<UFix64<N9>, CoreError> {
    let tvl_cap = self.tvl_cap()?;
    let capped = if tvl_cap == UFix64::zero() {
      tvl
    } else {
      tvl.min(tvl_cap)
    };
    Ok(capped)
  }

  /// Computes the USDC reserve required against a pair's TVL.
  pub fn required_reserve(
    &self,
    tvl: UFix64<N9>,
  ) -> Result<UFix64<N6>, CoreError> {
    self
      .capped_tvl(tvl)?
      .mul_ceil(self.reserve_per_tvl()?)
      .and_then(UFix64::checked_convert_ceil::<N6>)
      .ok_or(InsufficientLiquidity)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn caps_tvl_before_computing_reserve() -> Result<(), CoreError> {
    let gate = ReserveGate::validated(
      UFixValue64::new(50_000_000, -9),
      UFixValue64::new(10_000_000_000_000_000, -9),
    )?;
    let tvl = UFix64::<N9>::new(50_000_000_000_000_000);

    assert_eq!(
      gate.required_reserve(tvl)?,
      UFix64::<N6>::new(500_000_000_000),
    );
    Ok(())
  }

  #[test]
  fn zero_cap_leaves_tvl_uncapped() -> Result<(), CoreError> {
    let gate = ReserveGate::validated(
      UFixValue64::new(50_000_000, -9),
      UFixValue64::new(0, -9),
    )?;
    let tvl = UFix64::<N9>::new(50_000_000_000_000_000);

    assert_eq!(
      gate.required_reserve(tvl)?,
      UFix64::<N6>::new(2_500_000_000_000),
    );
    Ok(())
  }

  #[test]
  fn rejects_a_reserve_ratio_above_one() {
    assert_eq!(
      ReserveGate::validated(
        UFixValue64::new(1_000_000_001, -9),
        UFixValue64::new(0, -9),
      ),
      Err(ReserveGateInvalid),
    );
  }
}
