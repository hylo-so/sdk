//! Differentiation rules and float guards for marginal rate math.
//!
//! Division hazards are contained by type: every divisor enters as
//! [`StrictlyPositiveFinite`] via [`positive`], and rates leave
//! through [`positive_rate`].

use fix::prelude::*;
use fix::typed_floats::StrictlyPositiveFinite;
use fix::typenum::Integer;

use crate::error::CoreError;

/// Converts an unsigned fixed-point value to a strictly positive
/// finite float, the only form accepted as a divisor here.
///
/// # Errors
/// * Value is zero (degenerate protocol state)
pub fn positive<Exp: Integer>(
  value: UFix64<Exp>,
) -> Result<StrictlyPositiveFinite, CoreError> {
  value
    .to_positive_f64()
    .ok_or(CoreError::MarginalRateInvalid)
}

/// Validates a rate as strictly positive and finite — the only form a
/// marginal rate may leave this module in.
///
/// # Errors
/// * Non-finite or non-positive rate
pub fn positive_rate(rate: f64) -> Result<f64, CoreError> {
  StrictlyPositiveFinite::try_from(rate)
    .map(|checked| checked.get())
    .map_err(|_| CoreError::MarginalRateInvalid)
}

/// Derivative of a scaled ratio of two moving quantities:
///
/// ```txt
/// d(numerator * scale / denominator)
///
///   (d_numerator * denominator - numerator * d_denominator) * scale
/// = ---------------------------------------------------------------
///                          denominator^2
/// ```
///
/// `numerator` and `denominator` are evaluated at the quoted size;
/// `scale` is constant in the differentiation variable.
#[must_use]
pub fn quotient_rule(
  numerator: StrictlyPositiveFinite,
  d_numerator: f64,
  denominator: StrictlyPositiveFinite,
  d_denominator: f64,
  scale: StrictlyPositiveFinite,
) -> f64 {
  let denominator = denominator.get();
  (d_numerator * denominator - numerator.get() * d_denominator) * scale.get()
    / (denominator * denominator)
}

/// Derivative of a curve applied to a moving input:
///
/// ```txt
/// d(curve(inner(x))) = curve_slope * d_inner
/// ```
///
/// Zero `curve_slope` short-circuits to zero: on a flat curve region
/// the `d_inner` factor is irrelevant and may be infinite.
#[must_use]
pub fn chain_rule(curve_slope: f64, d_inner: f64) -> f64 {
  if curve_slope == 0.0 {
    0.0
  } else {
    curve_slope * d_inner
  }
}
