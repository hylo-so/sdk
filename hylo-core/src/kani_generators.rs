use fix::prelude::*;
use fix::typenum::Integer;

use crate::fees::curves::{MINT_FEE_INV, REDEEM_FEE_LN};
use crate::fees::interp::FixInterp;
use crate::pyth::PriceRange;

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
  kani::assume(conf.bits > 0 && conf.bits <= 100);
  let lower = price.checked_sub(&conf)?;
  let upper = price.checked_add(&conf)?;
  Some(PriceRange::new(lower, upper))
}

/// `PriceRange` centered at $1 with a tight symbolic confidence interval.
#[must_use]
pub fn dollar_centered_price_range() -> Option<PriceRange<N9>> {
  let conf: UFix64<N9> = UFix64::new(kani::any());
  kani::assume(conf > UFix64::zero());
  kani::assume(conf.bits <= 5_000_000);
  let one = UFix64::<N9>::one();
  let lower = one.checked_sub(&conf)?;
  let upper = one.checked_add(&conf)?;
  Some(PriceRange::new(lower, upper))
}

fn deployed_curve_bounds() -> (i64, i64, i64, i64) {
  let mint = FixInterp::from_points_unchecked(*MINT_FEE_INV);
  let redeem = FixInterp::from_points_unchecked(*REDEEM_FEE_LN);
  (
    mint.x_min().bits.min(redeem.x_min().bits),
    mint.x_max().bits.max(redeem.x_max().bits),
    mint.y_min().bits.min(redeem.y_min().bits),
    mint.y_max().bits.max(redeem.y_max().bits),
  )
}

#[must_use]
pub fn deployed_curve_x() -> IFix64<N5> {
  let (x_min, x_max, _, _) = deployed_curve_bounds();
  let bits: i64 = kani::any();
  kani::assume(bits >= x_min && bits <= x_max);
  IFix64::new(bits)
}

#[must_use]
pub fn deployed_curve_y() -> IFix64<N5> {
  let (_, _, y_min, y_max) = deployed_curve_bounds();
  let bits: i64 = kani::any();
  kani::assume(bits >= y_min && bits <= y_max);
  IFix64::new(bits)
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
