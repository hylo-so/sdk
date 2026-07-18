//! Marginal rate math for offchain quoting.
//!
//! Every operation has the shape `f(x) = x * R(cr(x))` for an effective
//! rate `R` over the projected post-trade collateral ratio. Its
//! derivative is
//!
//! ```txt
//! f'(x) = R(cr(x)) + x * R'(cr(x)) * cr'(x)
//! ```
//!
//! Floating point: the quotient rule squares the stablecoin supply,
//! which overflows 64-bit fixed-point. Division hazards are contained
//! by type: every divisor enters as [`StrictlyPositiveFinite`] via
//! [`positive`], and the final rate leaves through the same check in
//! [`marginal_rate`].

use fix::prelude::*;
use fix::typenum::Integer;
use typed_floats::StrictlyPositiveFinite;

use crate::error::CoreError;

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn ufix_to_f64<Exp: Integer>(value: UFix64<Exp>) -> f64 {
  (value.bits as f64) * 10f64.powi(Exp::to_i32())
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn ifix_to_f64<Exp: Integer>(value: IFix64<Exp>) -> f64 {
  (value.bits as f64) * 10f64.powi(Exp::to_i32())
}

/// Converts an unsigned fixed-point value to a strictly positive
/// finite float, the only form accepted as a divisor here.
///
/// # Errors
/// * Value is zero (degenerate protocol state)
pub fn positive<Exp: Integer>(
  value: UFix64<Exp>,
) -> Result<StrictlyPositiveFinite, CoreError> {
  StrictlyPositiveFinite::try_from(ufix_to_f64(value))
    .map_err(|_| CoreError::MarginalRateInvalid)
}

/// Derivative `cr'(x)` of the projected collateral ratio
/// `cr(x) = C(x) * p / S(x)` by the quotient rule:
///
/// ```txt
///          (C' * S - C * S') * p
/// cr'(x) = ---------------------
///                   S^2
/// ```
///
/// `collateral` and `supply` are the post-trade `C(x)` and `S(x)`.
#[must_use]
pub fn cr_impact(
  collateral: StrictlyPositiveFinite,
  supply: StrictlyPositiveFinite,
  collateral_price: StrictlyPositiveFinite,
  d_collateral: f64,
  d_supply: f64,
) -> f64 {
  let supply = supply.get();
  (d_collateral * supply - collateral.get() * d_supply) * collateral_price.get()
    / (supply * supply)
}

/// Marginal rate `f'(x) = R + x * R' * cr'(x)` at `amount_in`.
///
/// Zero `rate_slope` short-circuits to `rate`: with a flat rate curve
/// the `cr'(x)` factor is irrelevant and may be infinite (zero
/// projected supply). The result must be strictly positive and finite —
/// Titan's venue contract for a valid quote — so any `NaN`/`inf` that
/// propagated through the arithmetic is rejected here.
///
/// # Errors
/// * Non-finite or non-positive result
pub fn marginal_rate(
  amount_in: f64,
  rate: f64,
  rate_slope: f64,
  cr_impact: f64,
) -> Result<f64, CoreError> {
  let value = if rate_slope == 0.0 {
    rate
  } else {
    rate + amount_in * rate_slope * cr_impact
  };
  positive_rate(value)
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
