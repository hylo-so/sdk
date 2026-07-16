//! Piecewise linear interpolation for fee curves.

use std::ops::RangeInclusive;

use fix::prelude::*;
use fix::typenum::Integer;
use itertools::Itertools;

use crate::error::CoreError;

/// Fixed-point Cartesian coordinate.
#[derive(Debug, Clone, Copy)]
pub struct Point<Exp: Integer> {
  pub x: IFix64<Exp>,
  pub y: IFix64<Exp>,
}

impl<Exp: Integer> Point<Exp> {
  #[must_use]
  pub const fn from_ints(x: i64, y: i64) -> Point<Exp> {
    Point {
      x: IFix64::constant(x),
      y: IFix64::constant(y),
    }
  }
}

/// Line segment between two points for linear interpolation.
pub struct LineSegment<'a, Exp: Integer>(&'a Point<Exp>, &'a Point<Exp>);

impl<Exp: Integer> LineSegment<'_, Exp> {
  /// Linear interpolation to find an approximate `y` for the given `x`.
  ///
  /// ```txt
  /// y = y_0 + (y_1 - y_0) * (x - x_0) / (x_1 - x_0)
  /// ```
  #[must_use]
  pub fn lerp(&self, x: IFix64<Exp>) -> Option<IFix64<Exp>> {
    let Point { x: x0, y: y0 } = self.0;
    let Point { x: x1, y: y1 } = self.1;
    let denom = x1.checked_sub(x0)?;
    (denom != IFix64::zero())
      .then_some(denom)
      .and_then(|d| y1.checked_sub(y0)?.mul_div_ceil(x.checked_sub(x0)?, d))
      .and_then(|div| y0.checked_add(&div))
  }

  /// Finds approximate `x` for the given `y`.
  /// This is the inverse of [`lerp`](Self::lerp).
  ///
  /// ```txt
  ///           (y - y_0) * (x_1 - x_0)
  /// x = x_0 + -----------------------
  ///                  y_1 - y_0
  /// ```
  #[must_use]
  pub fn inverse_lerp(&self, y: IFix64<Exp>) -> Option<IFix64<Exp>> {
    let Point { x: x0, y: y0 } = self.0;
    let Point { x: x1, y: y1 } = self.1;
    let denom = y1.checked_sub(y0)?;
    (denom != IFix64::zero())
      .then_some(denom)
      .and_then(|d| x1.checked_sub(x0)?.mul_div_floor(y.checked_sub(y0)?, d))
      .and_then(|div| x0.checked_add(&div))
  }

  /// Slope of this segment.
  ///
  /// ```txt
  ///     y_1 - y_0
  /// m = ---------
  ///     x_1 - x_0
  /// ```
  #[must_use]
  pub fn slope(&self) -> Option<IFix64<Exp>>
  where
    IFix64<Exp>: FixExt,
  {
    let Point { x: x0, y: y0 } = self.0;
    let Point { x: x1, y: y1 } = self.1;
    let rise = y1.checked_sub(y0)?;
    let run = x1.checked_sub(x0)?;
    rise.mul_div_ceil(IFix64::<Exp>::one(), run)
  }
}

/// Piecewise linear interpolation over a fixed-size point array.
#[derive(Debug, Clone)]
pub struct FixInterp<const RES: usize, Exp: Integer> {
  points: [Point<Exp>; RES],
}

/// Errors if `value` falls outside `bounds`.
fn check_bounds<Exp: Integer>(
  bounds: RangeInclusive<IFix64<Exp>>,
  value: IFix64<Exp>,
) -> Result<(), CoreError> {
  bounds
    .contains(&value)
    .then_some(())
    .ok_or(CoreError::InterpOutOfDomain)
}

impl<const RES: usize, Exp: Integer> FixInterp<RES, Exp> {
  /// Creates a new interpolator from a point array.
  ///
  /// # Errors
  /// * Minimum of 2 points resolution
  /// * Monotonically increasing x values
  pub fn from_points(points: [Point<Exp>; RES]) -> Result<Self, CoreError> {
    (RES >= 2)
      .then_some(())
      .ok_or(CoreError::InterpInsufficientPoints)?;
    points
      .iter()
      .tuple_windows::<(_, _)>()
      .all(|(p0, p1)| p0.x < p1.x)
      .then_some(FixInterp { points })
      .ok_or(CoreError::InterpPointsNotMonotonic)
  }

  /// Constructs interpolator with no validations.
  #[must_use]
  pub const fn from_points_unchecked(points: [Point<Exp>; RES]) -> Self {
    FixInterp { points }
  }

  /// Returns the minimum x value in the domain.
  #[must_use]
  pub const fn x_min(&self) -> IFix64<Exp> {
    self.points[0].x
  }

  /// Returns the maximum x value in the domain.
  #[must_use]
  pub const fn x_max(&self) -> IFix64<Exp> {
    self.points[RES - 1].x
  }

  /// Returns the domain as an inclusive range.
  #[must_use]
  pub fn domain(&self) -> RangeInclusive<IFix64<Exp>> {
    self.x_min()..=self.x_max()
  }

  /// Returns the minimum y value in the range.
  #[must_use]
  pub const fn y_min(&self) -> IFix64<Exp> {
    self.points[0].y
  }

  /// Returns the maximum y value in the range.
  #[must_use]
  pub const fn y_max(&self) -> IFix64<Exp> {
    self.points[RES - 1].y
  }

  /// Returns the range as an inclusive range.
  #[must_use]
  pub fn range(&self) -> RangeInclusive<IFix64<Exp>> {
    self.y_min()..=self.y_max()
  }

  /// Interpolates to find y for a given x.
  ///
  /// # Errors
  ///
  /// * Input x is outside the valid domain.
  /// * Arithmetic overflow during calculation.
  pub fn interpolate(&self, x: IFix64<Exp>) -> Result<IFix64<Exp>, CoreError> {
    check_bounds(self.domain(), x)?;
    self
      .segment_at(|p| p.x < x)
      .and_then(|seg| seg.lerp(x))
      .ok_or(CoreError::InterpArithmetic)
  }

  /// Inverse of [`interpolate`](Self::interpolate): the approximate `x`
  /// for the given `y`.
  ///
  /// # Errors
  /// * `y` is outside the valid range
  /// * Arithmetic overflow
  pub fn inverse_interpolate(
    &self,
    y: IFix64<Exp>,
  ) -> Result<IFix64<Exp>, CoreError> {
    check_bounds(self.range(), y)?;
    self
      .segment_at(|p| p.y < y)
      .and_then(|seg| seg.inverse_lerp(y))
      .ok_or(CoreError::InterpArithmetic)
  }

  /// Derivative of curve at `x`, or the slope of its containing line segment.
  /// At a breakpoint, takes the segment to its left.
  ///
  /// # Errors
  /// * `x` is outside the valid domain
  /// * Arithmetic overflow
  pub fn derivative(&self, x: IFix64<Exp>) -> Result<IFix64<Exp>, CoreError>
  where
    IFix64<Exp>: FixExt,
  {
    check_bounds(self.domain(), x)?;
    self
      .segment_at(|p| p.x < x)
      .and_then(|seg| seg.slope())
      .ok_or(CoreError::InterpArithmetic)
  }

  /// Segment whose endpoints straddle the partition by `below`.
  fn segment_at(
    &self,
    below: impl Fn(&Point<Exp>) -> bool,
  ) -> Option<LineSegment<'_, Exp>> {
    let part = self.points.partition_point(below).max(1);
    self
      .points
      .get(part - 1)
      .zip(self.points.get(part))
      .map(|(p0, p1)| LineSegment(p0, p1))
  }
}

#[cfg(test)]
mod tests {
  use std::fs::File;
  use std::io::Write;

  use itertools::Itertools;

  use super::*;
  use crate::error::CoreError;
  use crate::fees::curves::{
    mint_fee_curve, redeem_fee_curve, MINT_FEE_INV, REDEEM_FEE_LN,
  };

  fn assert_segments_preserve_endpoints(points: &[Point<N5>]) {
    points
      .iter()
      .tuple_windows::<(_, _)>()
      .for_each(|(p0, p1)| {
        let seg = LineSegment(p0, p1);
        assert_eq!(seg.lerp(p0.x), Some(p0.y));
        assert_eq!(seg.lerp(p1.x), Some(p1.y));
      });
  }

  #[test]
  fn mint_curve_continuous_at_breakpoints() {
    assert_segments_preserve_endpoints(MINT_FEE_INV);
  }

  #[test]
  fn redeem_curve_continuous_at_breakpoints() {
    assert_segments_preserve_endpoints(REDEEM_FEE_LN);
  }

  #[test]
  fn from_points_insufficient_points() {
    let result = FixInterp::from_points([Point::<N5>::from_ints(0, 0)]);
    assert_eq!(result.err(), Some(CoreError::InterpInsufficientPoints));
  }

  #[test]
  fn from_points_non_monotonic() {
    let points = [Point::<N5>::from_ints(100, 10), Point::from_ints(50, 20)];
    let result = FixInterp::from_points(points);
    assert_eq!(result.err(), Some(CoreError::InterpPointsNotMonotonic));
  }

  #[test]
  fn from_points_valid_curves() -> anyhow::Result<()> {
    mint_fee_curve()?;
    redeem_fee_curve()?;
    Ok(())
  }

  #[test]
  fn interpolate_below_domain() -> anyhow::Result<()> {
    let interp = mint_fee_curve()?;
    let x = IFix64::<N5>::constant(149_999);
    assert_eq!(
      interp.interpolate(x).err(),
      Some(CoreError::InterpOutOfDomain)
    );
    Ok(())
  }

  #[test]
  fn interpolate_above_domain() -> anyhow::Result<()> {
    let interp = mint_fee_curve()?;
    let x = IFix64::<N5>::constant(170_001);
    assert_eq!(
      interp.interpolate(x).err(),
      Some(CoreError::InterpOutOfDomain)
    );
    Ok(())
  }

  #[test]
  fn interpolate_exact_first_point() -> anyhow::Result<()> {
    let interp = mint_fee_curve()?;
    let x = IFix64::<N5>::constant(150_000);
    let y = interp.interpolate(x)?;
    assert_eq!(y, IFix64::constant(200));
    Ok(())
  }

  #[test]
  fn interpolate_exact_last_point() -> anyhow::Result<()> {
    let interp = mint_fee_curve()?;
    let x = IFix64::<N5>::constant(170_000);
    let y = interp.interpolate(x)?;
    assert_eq!(y, IFix64::constant(5));
    Ok(())
  }

  #[test]
  fn interpolate_exact_interior_point() -> anyhow::Result<()> {
    let interp = redeem_fee_curve()?;
    let x = IFix64::<N5>::constant(150_000);
    let y = interp.interpolate(x)?;
    assert_eq!(y, IFix64::constant(200));
    Ok(())
  }

  #[test]
  fn interpolate_midpoint() -> anyhow::Result<()> {
    let interp = redeem_fee_curve()?;
    let x = IFix64::<N5>::constant(131_000);
    let y = interp.interpolate(x)?;
    assert_eq!(y, IFix64::constant(23));
    Ok(())
  }

  #[test]
  fn interpolate_two_point_curve() -> anyhow::Result<()> {
    let points = [Point::<N5>::from_ints(0, 100), Point::from_ints(100, 200)];
    let interp = FixInterp::from_points(points)?;

    assert_eq!(
      interp.interpolate(IFix64::constant(0))?,
      IFix64::constant(100)
    );
    assert_eq!(
      interp.interpolate(IFix64::constant(100))?,
      IFix64::constant(200)
    );
    assert_eq!(
      interp.interpolate(IFix64::constant(50))?,
      IFix64::constant(150)
    );

    assert_eq!(
      interp.interpolate(IFix64::constant(-1)).err(),
      Some(CoreError::InterpOutOfDomain)
    );
    assert_eq!(
      interp.interpolate(IFix64::constant(101)).err(),
      Some(CoreError::InterpOutOfDomain)
    );
    Ok(())
  }

  #[test]
  fn inverse_lerp_recovers_endpoints() {
    let p0 = Point::<N5>::from_ints(0, 100);
    let p1 = Point::<N5>::from_ints(100, 200);
    let seg = LineSegment(&p0, &p1);
    assert_eq!(
      seg.inverse_lerp(IFix64::constant(100)),
      Some(IFix64::constant(0))
    );
    assert_eq!(
      seg.inverse_lerp(IFix64::constant(200)),
      Some(IFix64::constant(100))
    );
  }

  #[test]
  fn lerp_inverse_lerp_roundtrip() {
    let p0 = Point::<N5>::from_ints(0, 100);
    let p1 = Point::<N5>::from_ints(100, 200);
    let seg = LineSegment(&p0, &p1);
    [0, 25, 50, 75, 100].into_iter().for_each(|xi| {
      let x = IFix64::<N5>::constant(xi);
      assert_eq!(seg.lerp(x).and_then(|y| seg.inverse_lerp(y)), Some(x));
    });
  }

  #[test]
  #[ignore = "offline use not for CI"]
  fn dump_mint_fee_curve() -> anyhow::Result<()> {
    let interp = mint_fee_curve()?;
    let mut f = File::create("mint_fee_curve.csv")?;
    writeln!(f, "cr,fee")?;
    (150_000..=170_000).try_for_each(|ix| -> anyhow::Result<()> {
      let x = IFix64::<N5>::constant(ix);
      let y = interp.interpolate(x)?;
      writeln!(f, "{}e-5,{}e-5", x.bits, y.bits)?;
      Ok(())
    })
  }

  #[test]
  #[ignore = "offline use not for CI"]
  fn dump_redeem_fee_curve() -> anyhow::Result<()> {
    let interp = redeem_fee_curve()?;
    let mut f = File::create("redeem_fee_curve.csv")?;
    writeln!(f, "cr,fee")?;
    (130_000..=300_000).try_for_each(|ix| -> anyhow::Result<()> {
      let x = IFix64::<N5>::constant(ix);
      let y = interp.interpolate(x)?;
      writeln!(f, "{}e-5,{}e-5", x.bits, y.bits)?;
      Ok(())
    })
  }
}

#[cfg(kani)]
mod proofs {
  use fix::prelude::*;

  use crate::fees::interp::{LineSegment, Point};
  use crate::kani_generators::{deployed_curve_x, deployed_curve_y};

  fn curve_coord() -> IFix64<N5> {
    let bits = kani::any_where(|b: &i64| *b >= 0 && *b < (1i64 << 8));
    IFix64::new(bits)
  }

  /// `lerp` at either endpoint returns that endpoint's `y`.
  #[kani::proof]
  fn lerp_preserves_endpoints() {
    let x0: IFix64<N5> = curve_coord();
    let y0: IFix64<N5> = curve_coord();
    let x1: IFix64<N5> = curve_coord();
    let y1: IFix64<N5> = curve_coord();
    kani::assume(x0 < x1);
    let p0 = Point { x: x0, y: y0 };
    let p1 = Point { x: x1, y: y1 };
    let seg = LineSegment(&p0, &p1);
    let pick: bool = kani::any();
    let (x, expected) = if pick { (x0, y0) } else { (x1, y1) };
    assert_eq!(seg.lerp(x), Some(expected));
  }

  /// `lerp` is total on segments within the deployed fee curve domain.
  #[kani::proof]
  fn lerp_safe_on_deployed_curve_domain() {
    let x0 = deployed_curve_x();
    let x1 = deployed_curve_x();
    kani::assume(x0 < x1);
    let y0 = deployed_curve_y();
    let y1 = deployed_curve_y();
    let x = deployed_curve_x();
    kani::assume(x >= x0 && x <= x1);
    let p0 = Point::<N5> { x: x0, y: y0 };
    let p1 = Point::<N5> { x: x1, y: y1 };
    let seg = LineSegment(&p0, &p1);
    assert!(seg.lerp(x).is_some());
  }
}
