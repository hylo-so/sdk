use std::fmt::{Display, Formatter};
use std::ops::{Bound, RangeBounds};

use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;

use crate::error::CoreError;
use crate::error::CoreError::{
  RangeUnexpectedBound, StablecoinMintThresholdInvalid,
};

#[derive(
  Debug,
  Copy,
  Clone,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
)]
pub enum RebalanceMode {
  Depeg,
  SellZone2,
  SellZone1,
  Neutral,
  BuyZone1,
  BuyZone2,
}

impl Display for RebalanceMode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      RebalanceMode::Depeg => f.write_str("Depeg"),
      RebalanceMode::SellZone2 => f.write_str("SellZone2"),
      RebalanceMode::SellZone1 => f.write_str("SellZone1"),
      RebalanceMode::Neutral => f.write_str("Neutral"),
      RebalanceMode::BuyZone1 => f.write_str("BuyZone1"),
      RebalanceMode::BuyZone2 => f.write_str("BuyZone2"),
    }
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CrRange {
  start: Bound<UFix64<N9>>,
  end: Bound<UFix64<N9>>,
}

impl CrRange {
  #[must_use]
  pub const fn new(
    start: Bound<UFix64<N9>>,
    end: Bound<UFix64<N9>>,
  ) -> CrRange {
    CrRange { start, end }
  }

  pub fn start(&self) -> Result<UFix64<N9>, CoreError> {
    match self.start {
      Bound::Included(start) => Ok(start),
      Bound::Excluded(_) | Bound::Unbounded => Err(RangeUnexpectedBound),
    }
  }

  pub fn end(&self) -> Result<UFix64<N9>, CoreError> {
    match self.end {
      Bound::Excluded(end) => Ok(end),
      Bound::Included(_) | Bound::Unbounded => Err(RangeUnexpectedBound),
    }
  }
}

impl RangeBounds<UFix64<N9>> for CrRange {
  fn start_bound(&self) -> Bound<&UFix64<N9>> {
    self.start.as_ref()
  }

  fn end_bound(&self) -> Bound<&UFix64<N9>> {
    self.end.as_ref()
  }
}

impl RebalanceMode {
  pub const ALL: [RebalanceMode; 6] = [
    RebalanceMode::Depeg,
    RebalanceMode::SellZone2,
    RebalanceMode::SellZone1,
    RebalanceMode::Neutral,
    RebalanceMode::BuyZone1,
    RebalanceMode::BuyZone2,
  ];

  #[must_use]
  pub const fn active_range(&self) -> CrRange {
    match self {
      RebalanceMode::Depeg => CrRange::new(
        Bound::Included(UFix64::constant(0)),
        Bound::Excluded(UFix64::constant(1_000_000_000)),
      ),
      RebalanceMode::SellZone2 => CrRange::new(
        Bound::Included(UFix64::constant(1_000_000_000)),
        Bound::Excluded(UFix64::constant(1_200_000_000)),
      ),
      RebalanceMode::SellZone1 => CrRange::new(
        Bound::Included(UFix64::constant(1_200_000_000)),
        Bound::Excluded(UFix64::constant(1_350_000_000)),
      ),
      RebalanceMode::Neutral => CrRange::new(
        Bound::Included(UFix64::constant(1_350_000_000)),
        Bound::Excluded(UFix64::constant(1_650_000_000)),
      ),
      RebalanceMode::BuyZone1 => CrRange::new(
        Bound::Included(UFix64::constant(1_650_000_000)),
        Bound::Excluded(UFix64::constant(1_750_000_000)),
      ),
      RebalanceMode::BuyZone2 => CrRange::new(
        Bound::Included(UFix64::constant(1_750_000_000)),
        Bound::Unbounded,
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
) -> Result<UFixValue64, CoreError> {
  RebalanceMode::Neutral
    .active_range()
    .contains(&stablecoin_mint_threshold.try_into()?)
    .then_some(stablecoin_mint_threshold)
    .ok_or(StablecoinMintThresholdInvalid)
}

#[cfg(test)]
mod tests {
  use RebalanceMode::*;

  use super::*;

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
    RebalanceMode::ALL
      .iter()
      .zip(RebalanceMode::ALL.iter().skip(1))
      .for_each(|(lower, upper)| {
        assert_eq!(
          lower.active_range().end(),
          upper.active_range().start(),
          "{lower:?} -> {upper:?}",
        );
      });
    assert_eq!(Depeg.active_range().start(), Ok(UFix64::zero()));
    assert_eq!(BuyZone2.active_range().end(), Err(RangeUnexpectedBound));
  }

  #[test]
  fn from_cr_start_inclusive() {
    RebalanceMode::ALL.iter().for_each(|mode| {
      assert_eq!(
        mode.active_range().start().map(RebalanceMode::from_cr),
        Ok(*mode),
      );
    });
  }

  #[test]
  fn from_cr_end_exclusive() {
    RebalanceMode::ALL.iter().for_each(|mode| {
      if let Ok(end) = mode.active_range().end() {
        let just_below = UFix64::new(end.bits - 1);
        assert_ne!(RebalanceMode::from_cr(end), *mode);
        assert_eq!(RebalanceMode::from_cr(just_below), *mode);
      }
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
    assert_eq!(r.start(), Ok(UFix64::constant(1_200_000_000)));
    assert_eq!(r.end(), Ok(UFix64::constant(1_350_000_000)));
    assert!(r.contains(&UFix64::constant(1_200_000_000)));
    assert!(r.contains(&UFix64::constant(1_349_999_999)));
    assert!(!r.contains(&UFix64::constant(1_350_000_000)));
  }
}

#[cfg(kani)]
mod proofs {
  use std::ops::RangeBounds;

  use fix::prelude::*;

  use crate::kani_generators::any_ufix64;
  use crate::rebalance::mode::RebalanceMode;

  /// `from_cr(cr)` returns a mode whose `active_range` contains `cr`.
  #[kani::proof]
  fn from_cr_mode_contains_input() {
    let cr: UFix64<N9> = any_ufix64();
    let mode = RebalanceMode::from_cr(cr);
    assert!(mode.active_range().contains(&cr));
  }

  /// Exactly one mode's `active_range` contains any given `cr` (partition).
  #[kani::proof]
  fn mode_zones_disjoint() {
    let cr: UFix64<N9> = any_ufix64();
    let count = RebalanceMode::ALL
      .iter()
      .filter(|m| m.active_range().contains(&cr))
      .count();
    assert_eq!(count, 1);
  }
}
