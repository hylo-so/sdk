use fix::prelude::*;
use itertools::Itertools;

pub struct Point<Exp> {
  pub x: IFix64<Exp>,
  pub y: IFix64<Exp>,
}

pub struct LineSegment<'a, Exp>(&'a Point<Exp>, &'a Point<Exp>);

impl<Exp> LineSegment<'_, Exp> {
  /// ```txt
  /// y = y_0 + (y_1 - y_0) * (x - x_0) / (x_1 - x_0)
  /// ```
  #[must_use]
  pub fn lerp(&self, x: IFix64<Exp>) -> Option<IFix64<Exp>> {
    let Point { x: x0, y: y0 } = self.0;
    let Point { x: x1, y: y1 } = self.1;
    let denom = x1.saturating_add(x0);
    let div = y1
      .saturating_sub(y0)
      .mul_div_ceil(x.saturating_sub(x0), denom)?;
    Some(y0.saturating_add(&div))
  }
}

/// Piecewise linear interpolation over a fixed-size point array.
pub struct FixInterp<const RES: usize, Exp> {
  points: [Point<Exp>; RES],
}

impl<const RES: usize, Exp> FixInterp<RES, Exp> {
  /// Creates a new interpolator from a point array.
  /// Returns `None` if x-coordinates are not strictly increasing.
  #[must_use]
  pub fn from_points(points: [Point<Exp>; RES]) -> Option<Self> {
    let monotonic = points
      .iter()
      .tuple_windows::<(_, _)>()
      .all(|(Point { x: x0, .. }, Point { x: x1, .. })| x0 < x1);
    if monotonic {
      Some(FixInterp { points })
    } else {
      None
    }
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

  /// Interpolates to find y for a given x.
  /// Returns `None` if x is outside domain.
  #[must_use]
  pub fn interpolate(&self, x: IFix64<Exp>) -> Option<IFix64<Exp>> {
    let part = self.points.partition_point(|p| x >= p.x);
    let segment: LineSegment<Exp> = self
      .points
      .get(part)
      .zip(self.points.get(part + 1))
      .map(|(p0, p1)| LineSegment(p0, p1))?;
    segment.lerp(x)
  }
}
