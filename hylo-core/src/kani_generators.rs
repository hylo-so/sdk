use std::ops::RangeBounds;

use fix::prelude::*;
use fix::typenum::Integer;

use crate::borrow_rate::MAX_RATE;
use crate::pyth::PriceRange;
use crate::rebalance::mode::{RebalanceMode, RebalanceScheme};

#[must_use]
pub fn any_ufix64<Exp: Integer>() -> UFix64<Exp> {
  UFix64::new(kani::any())
}

/// Symbolic `UFix64<Exp>` bounded to 40 raw bits (`< 2^40` or `1.1e12`).
///
/// Use when the proof needs realistic-magnitude inputs and the arithmetic
/// chain is shallow enough to stay tractable.
#[must_use]
pub fn wide_ufix64<Exp: Integer>() -> UFix64<Exp> {
  let v: UFix64<Exp> = any_ufix64();
  kani::assume(v.bits < (1u64 << 40));
  v
}

/// Symbolic `UFix64<Exp>` bounded to 16 raw bits (`< 65536`).
///
/// Use when wider bit widths are intractable. Magnitude doesn't change
/// algebraic properties.
#[must_use]
pub fn narrow_ufix64<Exp: Integer>() -> UFix64<Exp> {
  let v: UFix64<Exp> = any_ufix64();
  kani::assume(v.bits < (1u64 << 16));
  v
}

#[must_use]
pub fn narrow_price_range<Exp: Integer>() -> Option<PriceRange<Exp>> {
  let price: UFix64<Exp> = narrow_ufix64();
  let conf: UFix64<Exp> = UFix64::new(kani::any());
  kani::assume(conf.bits <= 100);
  let lower = price.checked_sub(&conf)?;
  let upper = price.checked_add(&conf)?;
  Some(PriceRange::new(lower, upper))
}

/// Symbolic CR within the widest borrow rate curve domain.
#[must_use]
pub fn borrow_rate_curve_x() -> IFix64<N9> {
  let x: IFix64<N9> = IFix64::new(kani::any());
  kani::assume(x.narrow::<u64>().is_some_and(|cr| {
    [RebalanceMode::Neutral, RebalanceMode::BuyZone1]
      .iter()
      .any(|mode| RebalanceScheme::Wide.active_range(*mode).contains(&cr))
  }));
  x
}

/// Symbolic rate within the valid borrow rate range.
#[must_use]
pub fn borrow_rate_curve_y() -> IFix64<N9> {
  let y: IFix64<N9> = IFix64::new(kani::any());
  kani::assume(y.narrow::<u64>().is_some_and(|rate| rate <= MAX_RATE));
  y
}

#[must_use]
pub fn tolerance() -> UFix64<N4> {
  let v: UFix64<N4> = any_ufix64();
  kani::assume(v <= UFix64::one());
  v
}

#[must_use]
pub fn wide_price_range<Exp: Integer>() -> PriceRange<Exp> {
  let lower: UFix64<Exp> = wide_ufix64();
  let upper: UFix64<Exp> = wide_ufix64();
  kani::assume(lower <= upper);
  PriceRange::new(lower, upper)
}
