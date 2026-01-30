//! Piecewise linear interpolation for onchain fee curves.

use anchor_lang::prelude::*;
use fix::prelude::*;
use fix::typenum::Integer;
use itertools::Itertools;

use crate::error::CoreError;

/// Serializable version of [`Point`] for account storage.
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub struct PointValue {
  pub x: i64,
  pub y: i64,
}

impl<Exp: Integer> From<PointValue> for Point<Exp> {
  fn from(PointValue { x, y }: PointValue) -> Self {
    Point {
      x: IFix64::new(x),
      y: IFix64::new(y),
    }
  }
}

impl<Exp: Integer> From<Point<Exp>> for PointValue {
  fn from(Point { x, y }: Point<Exp>) -> Self {
    PointValue {
      x: x.bits,
      y: y.bits,
    }
  }
}

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
    let div = y1
      .checked_sub(y0)?
      .mul_div_ceil(x.checked_sub(x0)?, denom)?;
    y0.checked_add(&div)
  }
}

/// Piecewise linear interpolation over a fixed-size point array.
#[derive(Debug)]
pub struct FixInterp<const RES: usize, Exp: Integer> {
  points: [Point<Exp>; RES],
}

impl<const RES: usize, Exp: Integer> FixInterp<RES, Exp> {
  /// Creates a new interpolator from a point array.
  ///
  /// # Errors
  /// * Minimum of 2 points resolution
  /// * Monotonically increasing x values
  pub fn from_points(points: [Point<Exp>; RES]) -> Result<Self> {
    (RES >= 2)
      .then_some(())
      .ok_or(CoreError::InterpInsufficientPoints)?;
    points
      .iter()
      .tuple_windows::<(_, _)>()
      .all(|(p0, p1)| p0.x < p1.x)
      .then_some(FixInterp { points })
      .ok_or(CoreError::InterpPointsNotMonotonic.into())
  }

  /// Loads an interpolator from serialized point values.
  ///
  /// # Errors
  /// * See [`FixInterp::from_points`].
  pub fn from_values(values: [PointValue; RES]) -> Result<Self> {
    FixInterp::from_points(values.map(Point::from))
  }

  /// Returns the minimum x value in the domain.
  #[must_use]
  pub fn x_min(&self) -> IFix64<Exp> {
    self.points[0].x
  }

  /// Returns the maximum x value in the domain.
  #[must_use]
  pub fn x_max(&self) -> IFix64<Exp> {
    self.points[RES - 1].x
  }

  /// Returns the minimum y value in the range.
  #[must_use]
  pub fn y_min(&self) -> IFix64<Exp> {
    self.points[0].y
  }

  /// Returns the maximum y value in the range.
  #[must_use]
  pub fn y_max(&self) -> IFix64<Exp> {
    self.points[RES - 1].y
  }

  /// Interpolates to find y for a given x.
  ///
  /// # Errors
  ///
  /// * Input x is outside the valid domain.
  /// * Arithmetic overflow during calculation.
  pub fn interpolate(&self, x: IFix64<Exp>) -> Result<IFix64<Exp>> {
    (x >= self.x_min() && x <= self.x_max())
      .then_some(())
      .ok_or(CoreError::InterpOutOfDomain)?;
    let part = self.points.partition_point(|p| p.x < x).max(1);
    self
      .points
      .get(part - 1)
      .zip(self.points.get(part))
      .map(|(p0, p1)| LineSegment(p0, p1))
      .and_then(|seg| seg.lerp(x))
      .ok_or(CoreError::InterpArithmetic.into())
  }
}

#[cfg(test)]
mod tests {
  use std::fs::File;
  use std::io::Write;

  use super::*;
  use crate::error::CoreError;
  use crate::fee_curves::{MINT_FEE_EXP_DECAY, REDEEM_FEE_LN};

  #[test]
  fn from_points_insufficient_points() {
    let result = FixInterp::from_points([Point::<N5>::from_ints(0, 0)]);
    assert_eq!(
      result.err(),
      Some(CoreError::InterpInsufficientPoints.into())
    );
  }

  #[test]
  fn from_points_non_monotonic() {
    let points = [Point::<N5>::from_ints(100, 10), Point::from_ints(50, 20)];
    let result = FixInterp::from_points(points);
    assert_eq!(
      result.err(),
      Some(CoreError::InterpPointsNotMonotonic.into())
    );
  }

  #[test]
  fn from_points_valid_curves() -> anyhow::Result<()> {
    FixInterp::from_points(*MINT_FEE_EXP_DECAY)?;
    FixInterp::from_points(*REDEEM_FEE_LN)?;
    Ok(())
  }

  #[test]
  fn interpolate_below_domain() -> anyhow::Result<()> {
    let interp = FixInterp::from_points(*MINT_FEE_EXP_DECAY)?;
    let x = IFix64::<N5>::constant(129_999);
    assert_eq!(
      interp.interpolate(x).err(),
      Some(CoreError::InterpOutOfDomain.into())
    );
    Ok(())
  }

  #[test]
  fn interpolate_above_domain() -> anyhow::Result<()> {
    let interp = FixInterp::from_points(*MINT_FEE_EXP_DECAY)?;
    let x = IFix64::<N5>::constant(300_001);
    assert_eq!(
      interp.interpolate(x).err(),
      Some(CoreError::InterpOutOfDomain.into())
    );
    Ok(())
  }

  #[test]
  fn interpolate_exact_first_point() -> anyhow::Result<()> {
    let interp = FixInterp::from_points(*MINT_FEE_EXP_DECAY)?;
    let x = IFix64::<N5>::constant(130_000);
    let y = interp.interpolate(x)?;
    assert_eq!(y, IFix64::constant(5000));
    Ok(())
  }

  #[test]
  fn interpolate_exact_last_point() -> anyhow::Result<()> {
    let interp = FixInterp::from_points(*MINT_FEE_EXP_DECAY)?;
    let x = IFix64::<N5>::constant(300_000);
    let y = interp.interpolate(x)?;
    assert_eq!(y, IFix64::constant(108));
    Ok(())
  }

  #[test]
  fn interpolate_exact_interior_point() -> anyhow::Result<()> {
    let interp = FixInterp::from_points(*REDEEM_FEE_LN)?;
    let x = IFix64::<N5>::constant(151_000);
    let y = interp.interpolate(x)?;
    assert_eq!(y, IFix64::constant(203));
    Ok(())
  }

  #[test]
  fn interpolate_midpoint() -> anyhow::Result<()> {
    let interp = FixInterp::from_points(*REDEEM_FEE_LN)?;
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
      Some(CoreError::InterpOutOfDomain.into())
    );
    assert_eq!(
      interp.interpolate(IFix64::constant(101)).err(),
      Some(CoreError::InterpOutOfDomain.into())
    );
    Ok(())
  }

  #[test]
  //#[ignore = "offline use not for CI"]
  fn dump_mint_fee_curve() -> anyhow::Result<()> {
    let interp = FixInterp::from_points(*MINT_FEE_EXP_DECAY)?;
    let mut f = File::create("mint_fee_curve.csv")?;
    writeln!(f, "cr,fee")?;
    (130_000..=300_000).try_for_each(|ix| -> anyhow::Result<()> {
      let x = IFix64::<N5>::constant(ix);
      let y = interp.interpolate(x)?;
      writeln!(f, "{}e-5,{}e-5", x.bits, y.bits)?;
      Ok(())
    })
  }

  #[test]
  //#[ignore = "offline use not for CI"]
  fn dump_redeem_fee_curve() -> anyhow::Result<()> {
    let interp = FixInterp::from_points(*REDEEM_FEE_LN)?;
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
