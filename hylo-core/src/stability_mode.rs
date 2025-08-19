use crate::error::CoreError::StabilityValidation;
use crate::stability_mode::StabilityMode::{Depeg, Mode1, Mode2, Normal};

use anchor_lang::prelude::*;
use fix::prelude::*;

use std::fmt::Display;

/// Mode of operation based on the protocol's current collateral ratio.
/// See whitepaper for more.
#[derive(
  Copy, Clone, AnchorSerialize, AnchorDeserialize, PartialEq, PartialOrd,
)]
pub enum StabilityMode {
  Normal,
  Mode1,
  Mode2,
  Depeg,
}

impl Display for StabilityMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Normal => f.write_str("Normal"),
      Mode1 => f.write_str("Mode1"),
      Mode2 => f.write_str("Mode2"),
      Depeg => f.write_str("Depeg"),
    }
  }
}

#[derive(Copy, Clone)]
pub struct StabilityController {
  pub stability_threshold_1: UFix64<N2>,
  pub stability_threshold_2: UFix64<N2>,
}

impl StabilityController {
  /// Parses stability thresholds into controller.
  pub fn new(
    stability_threshold_1: UFix64<N2>,
    stability_threshold_2: UFix64<N2>,
  ) -> Result<StabilityController> {
    let controller = StabilityController {
      stability_threshold_1,
      stability_threshold_2,
    };
    controller.validate()?;
    Ok(controller)
  }

  /// Determines which mode the protocol is in from the current collateral ratio
  /// and configured stability thresholds.
  pub fn stability_mode(
    &self,
    collateral_ratio: UFix64<N9>,
  ) -> Result<StabilityMode> {
    Ok(
      if collateral_ratio >= self.stability_threshold_1.convert() {
        Normal
      } else if collateral_ratio >= self.stability_threshold_2.convert() {
        Mode1
      } else if collateral_ratio >= UFix64::one() {
        Mode2
      } else {
        Depeg
      },
    )
  }

  /// Like [`next_stability_threshold`] but in reverse order.
  /// Yields the previously higher threshold.
  #[must_use]
  pub fn prev_stability_threshold(
    &self,
    mode: StabilityMode,
  ) -> Option<UFix64<N2>> {
    match mode {
      Normal => None,
      Mode1 => Some(self.stability_threshold_1),
      Mode2 => Some(self.stability_threshold_2),
      Depeg => Some(UFix64::one()),
    }
  }

  /// Given the current stability mode, returns the next lower CR threshold.
  /// Should be used when computing the maximum mintable stablecoin.
  /// When stablecoin is depegged, returns None.
  #[must_use]
  pub fn next_stability_threshold(
    &self,
    mode: StabilityMode,
  ) -> Option<UFix64<N2>> {
    match mode {
      Normal => Some(self.stability_threshold_1),
      Mode1 => Some(self.stability_threshold_2),
      Mode2 => Some(UFix64::one()),
      Depeg => None,
    }
  }

  /// Lowest tolerable threshold.
  #[must_use]
  pub fn min_stability_threshold(&self) -> UFix64<N2> {
    self.stability_threshold_2
  }

  /// Ensures stability thresholds:
  ///   - Are greater than 1.0
  ///   - Have 2 decimal places `X.XX`
  ///   - Mode 1 threshold is greater than mode 2 threshold
  pub fn validate(&self) -> Result<()> {
    let t1 = self.stability_threshold_1;
    let t2 = self.stability_threshold_2;
    let one = UFix64::one();
    if t1 > t2 && t1 > one && t2 > one {
      Ok(())
    } else {
      Err(StabilityValidation.into())
    }
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn stability_mode_ord() {
    use super::StabilityMode::*;
    assert!(Normal < Mode1);
    assert!(Mode1 < Mode2);
    assert!(Mode2 < Depeg);
  }
}
