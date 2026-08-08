use fix::prelude::*;

use crate::error::CoreError;
use crate::error::CoreError::{InvalidParTolerance, ParToleranceExceeded};

/// $0.001
pub const MAX_PAR_TOLERANCE: UFix64<N9> = UFix64::constant(1_000_000);

/// Maximum distance from par at which an asset settles at face value.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ParTolerance {
  tolerance: UFix64<N9>,
}

impl ParTolerance {
  pub fn new(tolerance: UFixValue64) -> Result<ParTolerance, CoreError> {
    let tolerance = tolerance.try_into()?;
    ParTolerance::validate_tolerance(tolerance)?;
    Ok(ParTolerance { tolerance })
  }

  /// Par tolerance must be in `(0, MAX_PAR_TOLERANCE]`.
  pub fn validate_tolerance(tolerance: UFix64<N9>) -> Result<(), CoreError> {
    if tolerance > UFix64::zero() && tolerance <= MAX_PAR_TOLERANCE {
      Ok(())
    } else {
      Err(InvalidParTolerance)
    }
  }

  /// Checks `|1 - spot|` against the tolerance.
  pub fn validate_spot(&self, spot: UFix64<N9>) -> Result<(), CoreError> {
    if spot.abs_diff(&UFix64::one()) <= self.tolerance {
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
    let par_tolerance = ParTolerance::new(UFixValue64::new(500_000, -9))?;
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
    let zero = ParTolerance::new(UFixValue64::new(0, -9));
    let over = ParTolerance::new(UFixValue64::new(1_000_001, -9));
    assert_eq!(zero.err(), Some(InvalidParTolerance));
    assert_eq!(over.err(), Some(InvalidParTolerance));
  }
}
