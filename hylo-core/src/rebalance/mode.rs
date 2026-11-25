use std::fmt::{Display, Formatter};
use std::ops::{Bound, RangeBounds};

use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;

use crate::collateral_ratio::{CollateralRatio, CR};
use crate::error::CoreError;
use crate::error::CoreError::RangeUnexpectedBound;

#[derive(
  Debug,
  Clone,
  Copy,
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

#[derive(
  Debug,
  Clone,
  Copy,
  PartialEq,
  Eq,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  Default,
)]
pub enum RebalanceTolerance {
  #[default]
  Wide,
  Narrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl RebalanceTolerance {
  const fn neutral_start(self) -> UFix64<N9> {
    match self {
      RebalanceTolerance::Wide => UFix64::constant(1_350_000_000),
      RebalanceTolerance::Narrow => UFix64::constant(1_450_000_000),
    }
  }

  const fn neutral_end(self) -> UFix64<N9> {
    match self {
      RebalanceTolerance::Wide => UFix64::constant(1_650_000_000),
      RebalanceTolerance::Narrow => UFix64::constant(1_550_000_000),
    }
  }

  #[must_use]
  pub const fn active_range(self, mode: RebalanceMode) -> CrRange {
    match mode {
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
        Bound::Excluded(self.neutral_start()),
      ),
      RebalanceMode::Neutral => CrRange::new(
        Bound::Included(self.neutral_start()),
        Bound::Excluded(self.neutral_end()),
      ),
      RebalanceMode::BuyZone1 => CrRange::new(
        Bound::Included(self.neutral_end()),
        Bound::Excluded(UFix64::constant(1_750_000_000)),
      ),
      RebalanceMode::BuyZone2 => CrRange::new(
        Bound::Included(UFix64::constant(1_750_000_000)),
        Bound::Unbounded,
      ),
    }
  }

  #[must_use]
  pub fn mode(self, cr: CollateralRatio) -> RebalanceMode {
    match cr {
      CR::Infinite => RebalanceMode::BuyZone2,
      CR::Finite(cr) => [
        RebalanceMode::Depeg,
        RebalanceMode::SellZone2,
        RebalanceMode::SellZone1,
        RebalanceMode::Neutral,
        RebalanceMode::BuyZone1,
      ]
      .into_iter()
      .find(|mode| self.active_range(*mode).contains(&cr))
      .unwrap_or(RebalanceMode::BuyZone2),
    }
  }
}

#[cfg(test)]
mod tests {
  use RebalanceMode::*;

  use super::*;
  use crate::collateral_ratio::CR;

  const TOLERANCES: [RebalanceTolerance; 2] =
    [RebalanceTolerance::Wide, RebalanceTolerance::Narrow];

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
    TOLERANCES.iter().for_each(|tolerance| {
      RebalanceMode::ALL
        .iter()
        .zip(RebalanceMode::ALL.iter().skip(1))
        .for_each(|(lower, upper)| {
          assert_eq!(
            tolerance.active_range(*lower).end(),
            tolerance.active_range(*upper).start(),
            "{tolerance:?}: {lower:?} -> {upper:?}",
          );
        });
      assert_eq!(tolerance.active_range(Depeg).start(), Ok(UFix64::zero()));
      assert_eq!(
        tolerance.active_range(BuyZone2).end(),
        Err(RangeUnexpectedBound),
      );
    });
  }

  #[test]
  fn mode_start_inclusive() {
    TOLERANCES.iter().for_each(|tolerance| {
      RebalanceMode::ALL.iter().for_each(|mode| {
        assert_eq!(
          tolerance
            .active_range(*mode)
            .start()
            .map(|cr| tolerance.mode(CR::Finite(cr))),
          Ok(*mode),
        );
      });
    });
  }

  #[test]
  fn mode_end_exclusive() {
    TOLERANCES.iter().for_each(|tolerance| {
      RebalanceMode::ALL.iter().for_each(|mode| {
        if let Ok(end) = tolerance.active_range(*mode).end() {
          let just_below = UFix64::new(end.bits - 1);
          assert_ne!(tolerance.mode(CR::Finite(end)), *mode);
          assert_eq!(tolerance.mode(CR::Finite(just_below)), *mode);
        }
      });
    });
  }

  #[test]
  fn mode_extremes() {
    TOLERANCES.iter().for_each(|tolerance| {
      assert_eq!(tolerance.mode(CR::Finite(UFix64::zero())), Depeg);
      assert_eq!(tolerance.mode(CR::Infinite), BuyZone2);
    });
  }

  #[test]
  fn narrow_moves_neutral_edges() {
    let narrow = RebalanceTolerance::Narrow;
    let cr = |bits| CR::Finite(UFix64::constant(bits));
    assert_eq!(narrow.mode(cr(1_449_999_999)), SellZone1);
    assert_eq!(narrow.mode(cr(1_450_000_000)), Neutral);
    assert_eq!(narrow.mode(cr(1_549_999_999)), Neutral);
    assert_eq!(narrow.mode(cr(1_550_000_000)), BuyZone1);
  }

  #[test]
  fn active_range_is_half_open() {
    let r = RebalanceTolerance::Wide.active_range(SellZone1);
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

  use crate::collateral_ratio::CR;
  use crate::kani_generators::any_ufix64;
  use crate::rebalance::mode::{RebalanceMode, RebalanceTolerance};

  /// `mode(cr)` returns a mode whose `active_range` contains `cr`.
  #[kani::proof]
  fn mode_contains_input() {
    let tolerance = if kani::any() {
      RebalanceTolerance::Wide
    } else {
      RebalanceTolerance::Narrow
    };
    let cr: UFix64<N9> = any_ufix64();
    let mode = tolerance.mode(CR::Finite(cr));
    assert!(tolerance.active_range(mode).contains(&cr));
  }

  /// Exactly one mode's `active_range` contains any given `cr` (partition).
  #[kani::proof]
  fn mode_zones_disjoint() {
    let tolerance = if kani::any() {
      RebalanceTolerance::Wide
    } else {
      RebalanceTolerance::Narrow
    };
    let cr: UFix64<N9> = any_ufix64();
    let count = RebalanceMode::ALL
      .iter()
      .filter(|m| tolerance.active_range(**m).contains(&cr))
      .count();
    assert_eq!(count, 1);
  }
}
