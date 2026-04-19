use std::ops::{Bound, RangeBounds};

use anchor_lang::prelude::Result;
use fix::prelude::*;

use crate::error::CoreError::StablecoinMintThresholdInvalid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RebalanceMode {
  Depeg,
  SellZone2,
  SellZone1,
  Neutral,
  BuyZone1,
  BuyZone2,
}

/// Half-open `[start, end)` CR range. `BuyZone2`'s end is `UFix64::MAX`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CrRange {
  pub start: UFix64<N9>,
  pub end: UFix64<N9>,
}

impl CrRange {
  #[must_use]
  pub const fn new(start: UFix64<N9>, end: UFix64<N9>) -> CrRange {
    CrRange { start, end }
  }
}

impl RangeBounds<UFix64<N9>> for CrRange {
  fn start_bound(&self) -> Bound<&UFix64<N9>> {
    Bound::Included(&self.start)
  }

  fn end_bound(&self) -> Bound<&UFix64<N9>> {
    Bound::Excluded(&self.end)
  }
}

impl RebalanceMode {
  #[must_use]
  pub const fn active_range(&self) -> CrRange {
    match self {
      RebalanceMode::Depeg => {
        CrRange::new(UFix64::constant(0), UFix64::constant(1_000_000_000))
      }
      RebalanceMode::SellZone2 => CrRange::new(
        UFix64::constant(1_000_000_000),
        UFix64::constant(1_200_000_000),
      ),
      RebalanceMode::SellZone1 => CrRange::new(
        UFix64::constant(1_200_000_000),
        UFix64::constant(1_350_000_000),
      ),
      RebalanceMode::Neutral => CrRange::new(
        UFix64::constant(1_350_000_000),
        UFix64::constant(1_650_000_000),
      ),
      RebalanceMode::BuyZone1 => CrRange::new(
        UFix64::constant(1_650_000_000),
        UFix64::constant(1_750_000_000),
      ),
      RebalanceMode::BuyZone2 => CrRange::new(
        UFix64::constant(1_750_000_000),
        UFix64::constant(u64::MAX),
      ),
    }
  }

  #[must_use]
  pub fn from_cr(cr: UFix64<N9>) -> RebalanceMode {
    [
      RebalanceMode::Depeg,
      RebalanceMode::SellZone2,
      RebalanceMode::SellZone1,
      RebalanceMode::Neutral,
      RebalanceMode::BuyZone1,
    ]
    .into_iter()
    .find(|mode| mode.active_range().contains(&cr))
    .unwrap_or(RebalanceMode::BuyZone2)
  }
}

/// Checks that the given stablecoin mint threshold is within the Neutral
/// rebalance zone.
pub fn validate_stablecoin_mint_threshold(
  stablecoin_mint_threshold: UFixValue64,
) -> Result<UFixValue64> {
  RebalanceMode::Neutral
    .active_range()
    .contains(&stablecoin_mint_threshold.try_into()?)
    .then_some(stablecoin_mint_threshold)
    .ok_or(StablecoinMintThresholdInvalid.into())
}

#[cfg(test)]
mod tests {
  use RebalanceMode::*;

  use super::*;

  const ALL: [RebalanceMode; 6] =
    [Depeg, SellZone2, SellZone1, Neutral, BuyZone1, BuyZone2];

  #[test]
  fn mode_ordering() {
    assert!(Depeg < SellZone2);
    assert!(SellZone2 < SellZone1);
    assert!(SellZone1 < Neutral);
    assert!(Neutral < BuyZone1);
    assert!(BuyZone1 < BuyZone2);
  }

  #[test]
  fn ranges_are_contiguous() {
    ALL
      .iter()
      .zip(ALL.iter().skip(1))
      .for_each(|(lower, upper)| {
        assert_eq!(
          lower.active_range().end,
          upper.active_range().start,
          "{lower:?} -> {upper:?}",
        );
      });
    assert_eq!(Depeg.active_range().start, UFix64::zero());
    assert_eq!(BuyZone2.active_range().end, UFix64::new(u64::MAX));
  }

  #[test]
  fn from_cr_start_inclusive() {
    ALL.iter().for_each(|mode| {
      assert_eq!(RebalanceMode::from_cr(mode.active_range().start), *mode);
    });
  }

  #[test]
  fn from_cr_end_exclusive() {
    ALL.iter().for_each(|mode| {
      let end = mode.active_range().end;
      let just_below = UFix64::new(end.bits - 1);
      assert_ne!(RebalanceMode::from_cr(end), *mode);
      assert_eq!(RebalanceMode::from_cr(just_below), *mode);
    });
  }

  #[test]
  fn from_cr_extremes() {
    assert_eq!(RebalanceMode::from_cr(UFix64::zero()), Depeg);
    assert_eq!(RebalanceMode::from_cr(UFix64::new(u64::MAX)), BuyZone2);
  }

  #[test]
  fn active_range_is_half_open() {
    let r = SellZone1.active_range();
    assert_eq!(r.start, UFix64::constant(1_200_000_000));
    assert_eq!(r.end, UFix64::constant(1_350_000_000));
    assert!(r.contains(&UFix64::constant(1_200_000_000)));
    assert!(r.contains(&UFix64::constant(1_349_999_999)));
    assert!(!r.contains(&UFix64::constant(1_350_000_000)));
  }
}
