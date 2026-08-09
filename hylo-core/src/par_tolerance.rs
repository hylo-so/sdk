use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::error::CoreError::{InvalidParTolerance, ParToleranceExceeded};

const MIN_PAR_TOLERANCE: UFix64<N9> = UFix64::constant(10_000);
const MAX_PAR_TOLERANCE: UFix64<N9> = UFix64::constant(1_000_000);

/// Maximum distance from par at which an asset settles at face value.
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
pub struct ParTolerance {
  pub tolerance: UFixValue64,
}

impl ParTolerance {
  fn new(tolerance: UFixValue64) -> ParTolerance {
    ParTolerance { tolerance }
  }

  /// Par tolerance must be in `[MIN, MAX]`.
  pub fn validated(tolerance: UFixValue64) -> Result<ParTolerance, CoreError> {
    if (MIN_PAR_TOLERANCE..=MAX_PAR_TOLERANCE).contains(&tolerance.try_into()?)
    {
      Ok(ParTolerance::new(tolerance))
    } else {
      Err(InvalidParTolerance)
    }
  }

  /// Lifts serialized tolerance to `UFix64`.
  pub fn tolerance(&self) -> Result<UFix64<N9>, CoreError> {
    Ok(self.tolerance.try_into()?)
  }

  /// Checks `|1 - spot|` against the tolerance.
  pub fn validate_spot(&self, spot: UFix64<N9>) -> Result<(), CoreError> {
    if spot.abs_diff(&UFix64::one()) <= self.tolerance()? {
      Ok(())
    } else {
      Err(ParToleranceExceeded)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn band_is_symmetric_and_inclusive() -> Result<(), CoreError> {
    let par_tolerance = ParTolerance::validated(UFixValue64::new(500_000, -9))?;
    par_tolerance.validate_spot(UFix64::new(999_500_000))?;
    par_tolerance.validate_spot(UFix64::new(1_000_500_000))?;
    let under = par_tolerance.validate_spot(UFix64::new(999_499_999));
    let over = par_tolerance.validate_spot(UFix64::new(1_000_500_001));
    assert_eq!(under.err(), Some(ParToleranceExceeded));
    assert_eq!(over.err(), Some(ParToleranceExceeded));
    Ok(())
  }

  #[test]
  fn reject_out_of_range_tolerance() {
    let zero = ParTolerance::validated(UFixValue64::new(0, -9));
    let over = ParTolerance::validated(UFixValue64::new(1_000_001, -9));
    assert_eq!(zero.err(), Some(InvalidParTolerance));
    assert_eq!(over.err(), Some(InvalidParTolerance));
  }
}
