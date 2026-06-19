//! Oracle-derived collateral rebalancing price curves.
//!
//! Computes the price at which collateral trades against USDC based on
//! the protocol's current collateral ratio (CR) and oracle price range.
//!
//! Two independent curves:
//! * **Sell side** (low CR, 1.0–1.35): protocol sells collateral for USDC
//! * **Buy side** (high CR, 1.65–2.0+): protocol buys collateral with USDC

use anchor_lang::prelude::*;
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::fees::interp::{FixInterp, Point};
use crate::pyth::OraclePrice;
use crate::rebalance::mode::RebalanceMode;

// Confidence multiplier boundaries
const MIN_CONF_MULT: UFix64<N2> = UFix64::constant(0);
const MAX_CONF_FLOOR_MULT: UFix64<N2> = UFix64::constant(1_000);
const MAX_CONF_CEIL_MULT: UFix64<N2> = UFix64::constant(1_000);

// Percent deviation boundaries
const MIN_DEVIATION_PCT: UFix64<N9> = UFix64::constant(0);
const MAX_DEVIATION_PCT: UFix64<N9> = UFix64::constant(20_000_000);

// Checks deviation tolerance against boundaries.
pub fn validate_deviation_tolerance(dev: UFixValue64) -> Result<UFixValue64> {
  let deviation: UFix64<N9> = dev.try_into()?;
  (MIN_DEVIATION_PCT..=MAX_DEVIATION_PCT)
    .contains(&deviation)
    .then_some(dev)
    .ok_or(CoreError::RebalanceDeviationValidation.into())
}

/// Confidence interval multipliers for rebalance price curve construction.
#[derive(
  Copy,
  Clone,
  Debug,
  PartialEq,
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Serialize,
  Deserialize,
)]
pub struct RebalanceCurveConfig {
  pub floor_mult: UFixValue64,
  pub ceil_mult: UFixValue64,
}

impl RebalanceCurveConfig {
  #[must_use]
  pub fn new(
    floor_mult: UFixValue64,
    ceil_mult: UFixValue64,
  ) -> RebalanceCurveConfig {
    RebalanceCurveConfig {
      floor_mult,
      ceil_mult,
    }
  }

  /// Converts floor CI multiplier to `UFix64`.
  ///
  /// # Errors
  /// * Conversion fails
  pub fn floor_mult(&self) -> Result<UFix64<N2>> {
    self.floor_mult.try_into()
  }

  /// Converts ceil CI multiplier to `UFix64`.
  ///
  /// # Errors
  /// * Conversion fails
  pub fn ceil_mult(&self) -> Result<UFix64<N2>> {
    self.ceil_mult.try_into()
  }

  /// Checks validity of CI multipliers.
  ///
  /// # Errors
  /// * Incorrect precision or failed validation
  pub fn validate(self) -> Result<Self> {
    let floor_ok =
      (MIN_CONF_MULT..=MAX_CONF_FLOOR_MULT).contains(&self.floor_mult()?);
    let ceil_ok =
      (MIN_CONF_MULT..=MAX_CONF_CEIL_MULT).contains(&self.ceil_mult()?);
    (floor_ok && ceil_ok)
      .then_some(self)
      .ok_or(CoreError::RebalanceCurveConfigValidation.into())
  }
}

/// Convert unsigned CR to signed for curve lookup.
///
/// # Errors
/// * Conversion overflow
fn narrow_cr(cr: UFix64<N9>) -> Result<IFix64<N9>> {
  cr.narrow::<i64>()
    .ok_or(CoreError::RebalancePriceConversion.into())
}

/// Convert unsigned oracle price to signed for curve storage.
///
/// # Errors
/// * Conversion overflow
fn narrow_price(price: UFix64<N9>) -> Result<IFix64<N9>> {
  price
    .narrow::<i64>()
    .ok_or(CoreError::RebalancePriceConversion.into())
}

/// Scales confidence interval by multiplier.
///
/// # Errors
/// * Arithmetic overflow
fn scale_ci(ci: UFix64<N9>, mult: UFix64<N2>) -> Result<UFix64<N9>> {
  ci.mul_div_ceil(mult, UFix64::<N2>::one())
    .ok_or(CoreError::RebalancePriceConstruction.into())
}

/// Clamps `projected_price` into `spot_price ± spot_price * tolerance`.
///
/// # Errors
/// * Arithmetic overflow computing the tolerance band.
fn clamp_to_tolerance(
  spot_price: UFix64<N9>,
  projected_price: UFix64<N9>,
  tolerance: UFix64<N9>,
) -> Result<UFix64<N9>> {
  clamp_to_tolerance_inner(spot_price, projected_price, tolerance)
    .ok_or(CoreError::RebalanceDeviationArithmetic.into())
}

fn clamp_to_tolerance_inner(
  spot_price: UFix64<N9>,
  projected_price: UFix64<N9>,
  tolerance: UFix64<N9>,
) -> Option<UFix64<N9>> {
  let max_delta = spot_price.mul_div_ceil(tolerance, UFix64::<N9>::one())?;
  if spot_price.abs_diff(&projected_price) <= max_delta {
    Some(projected_price)
  } else if projected_price < spot_price {
    spot_price.checked_sub(&max_delta)
  } else {
    spot_price.checked_add(&max_delta)
  }
}

/// Interpolated rebalance price controller.
/// Implementors define boundary behavior via
/// [`RebalancePriceController::price_inner`].
pub trait RebalancePriceController {
  /// Reference to the underlying interpolator.
  fn curve(&self) -> &FixInterp<2, N9>;

  /// Whether the given CR falls within the active domain.
  fn is_active(&self, ucr: UFix64<N9>) -> bool;

  /// Compute price for CR from underlying curve with boundary handling.
  ///
  /// # Errors
  /// * Domain or arithmetic errors.
  fn price_inner(&self, cr: IFix64<N9>) -> Result<IFix64<N9>>;

  /// Collateral price at the given CR.
  ///
  /// # Errors
  /// * CR conversion, domain, or arithmetic
  fn price(&self, ucr: UFix64<N9>) -> Result<UFix64<N9>> {
    let cr = narrow_cr(ucr)?;
    self
      .price_inner(cr)?
      .narrow()
      .ok_or(CoreError::RebalancePriceConversion.into())
  }

  /// Validate curve invariants after construction.
  ///
  /// # Errors
  /// * Invariant violation (e.g. non-positive prices, floor >= ceil).
  fn validate(self) -> Result<Self>
  where
    Self: Sized;
}

/// Sell side rebalance pricing curve.
/// Active when CR is low (below 1.35).
#[derive(Debug, Clone)]
pub struct SellPriceCurve {
  curve: FixInterp<2, N9>,
}

impl SellPriceCurve {
  /// Construct sell side price curve.
  ///
  /// # Errors
  /// * Arithmetic underflow/overflow
  /// * Conversion overflow
  pub fn new(
    OraclePrice { spot, conf }: OraclePrice,
    config: &RebalanceCurveConfig,
    deviation_tolerance: UFix64<N9>,
  ) -> Result<SellPriceCurve> {
    let (raw_floor, raw_ceil) = spot
      .checked_sub(&scale_ci(conf, config.floor_mult()?)?)
      .zip(spot.checked_add(&scale_ci(conf, config.ceil_mult()?)?))
      .ok_or(CoreError::RebalancePriceConstruction)?;
    let floor = clamp_to_tolerance(spot, raw_floor, deviation_tolerance)?;
    let ceil = clamp_to_tolerance(spot, raw_ceil, deviation_tolerance)?;
    let sell_zone_1 = RebalanceMode::SellZone1.active_range();
    let curve = FixInterp::from_points([
      Point {
        x: narrow_cr(sell_zone_1.start()?)?,
        y: narrow_price(floor)?,
      },
      Point {
        x: narrow_cr(sell_zone_1.end()?)?,
        y: narrow_price(ceil)?,
      },
    ])?;
    SellPriceCurve { curve }.validate()
  }
}

impl RebalancePriceController for SellPriceCurve {
  fn curve(&self) -> &FixInterp<2, N9> {
    &self.curve
  }

  fn is_active(&self, ucr: UFix64<N9>) -> bool {
    (RebalanceMode::SellZone2..RebalanceMode::Neutral)
      .contains(&RebalanceMode::from_cr(ucr))
  }

  fn price_inner(&self, cr: IFix64<N9>) -> Result<IFix64<N9>> {
    let interp = self.curve();
    if cr < interp.x_min() {
      Ok(interp.y_min())
    } else if cr > interp.x_max() {
      Err(CoreError::RebalanceOutOfDomain.into())
    } else {
      interp.interpolate(cr)
    }
  }

  fn validate(self) -> Result<SellPriceCurve> {
    let interp = self.curve();
    (interp.y_min() > IFix64::zero() && interp.y_min() < interp.y_max())
      .then_some(self)
      .ok_or(CoreError::RebalancePriceConstruction.into())
  }
}

/// Buy-side rebalance pricing curve.
/// Active when CR is high (above 1.65).
#[derive(Debug, Clone)]
pub struct BuyPriceCurve {
  curve: FixInterp<2, N9>,
}

impl BuyPriceCurve {
  /// Construct buy side price curve.
  ///
  /// # Errors
  /// * Arithmetic underflow/overflow
  /// * Precision conversion
  pub fn new(
    OraclePrice { spot, conf }: OraclePrice,
    config: &RebalanceCurveConfig,
    deviation_tolerance: UFix64<N9>,
  ) -> Result<BuyPriceCurve> {
    let (raw_floor, raw_ceil) = spot
      .checked_sub(&scale_ci(conf, config.floor_mult()?)?)
      .zip(spot.checked_add(&scale_ci(conf, config.ceil_mult()?)?))
      .ok_or(CoreError::RebalancePriceConstruction)?;
    let floor = clamp_to_tolerance(spot, raw_floor, deviation_tolerance)?;
    let ceil = clamp_to_tolerance(spot, raw_ceil, deviation_tolerance)?;
    let buy_zone_1 = RebalanceMode::BuyZone1.active_range();
    let curve = FixInterp::from_points([
      Point {
        x: narrow_cr(buy_zone_1.start()?)?,
        y: narrow_price(floor)?,
      },
      Point {
        x: narrow_cr(buy_zone_1.end()?)?,
        y: narrow_price(ceil)?,
      },
    ])?;
    BuyPriceCurve { curve }.validate()
  }
}

impl RebalancePriceController for BuyPriceCurve {
  fn curve(&self) -> &FixInterp<2, N9> {
    &self.curve
  }

  fn is_active(&self, ucr: UFix64<N9>) -> bool {
    RebalanceMode::from_cr(ucr) > RebalanceMode::Neutral
  }

  fn price_inner(&self, cr: IFix64<N9>) -> Result<IFix64<N9>> {
    let interp = self.curve();
    if cr < interp.x_min() {
      Err(CoreError::RebalanceOutOfDomain.into())
    } else if cr > interp.x_max() {
      Ok(interp.y_max())
    } else {
      interp.interpolate(cr)
    }
  }

  fn validate(self) -> Result<BuyPriceCurve> {
    let interp = self.curve();
    (interp.y_min() > IFix64::zero() && interp.y_min() < interp.y_max())
      .then_some(self)
      .ok_or(CoreError::RebalancePriceConstruction.into())
  }
}

#[cfg(test)]
mod tests {
  use more_asserts::*;
  use proptest::prelude::*;

  use super::*;
  use crate::error::CoreError;
  use crate::pyth::OraclePrice;

  const ORACLE: OraclePrice = OraclePrice {
    spot: UFix64::constant(146_401_109_370),
    conf: UFix64::constant(94_635_820),
  };

  const DEVIATION_5_PCT: UFix64<N9> = UFix64::constant(50_000_000);

  const SELL_CONFIG: RebalanceCurveConfig = RebalanceCurveConfig {
    floor_mult: UFixValue64 { bits: 200, exp: -2 },
    ceil_mult: UFixValue64 { bits: 100, exp: -2 },
  };

  const BUY_CONFIG: RebalanceCurveConfig = RebalanceCurveConfig {
    floor_mult: UFixValue64 { bits: 100, exp: -2 },
    ceil_mult: UFixValue64 { bits: 100, exp: -2 },
  };

  const UCR_1_00: UFix64<N9> = UFix64::constant(1_000_000_000);
  const UCR_1_15: UFix64<N9> = UFix64::constant(1_150_000_000);
  const UCR_1_20: UFix64<N9> = UFix64::constant(1_200_000_000);
  const UCR_1_35: UFix64<N9> = UFix64::constant(1_350_000_000);
  const UCR_1_40: UFix64<N9> = UFix64::constant(1_400_000_000);
  const UCR_1_60: UFix64<N9> = UFix64::constant(1_600_000_000);
  const UCR_1_65: UFix64<N9> = UFix64::constant(1_650_000_000);
  const UCR_1_75: UFix64<N9> = UFix64::constant(1_750_000_000);
  const UCR_1_80: UFix64<N9> = UFix64::constant(1_800_000_000);
  const UCR_2_50: UFix64<N9> = UFix64::constant(2_500_000_000);

  #[test]
  fn sell_constructs() -> Result<()> {
    SellPriceCurve::new(ORACLE, &SELL_CONFIG, DEVIATION_5_PCT)?;
    Ok(())
  }

  #[test]
  fn buy_constructs() -> Result<()> {
    BuyPriceCurve::new(ORACLE, &BUY_CONFIG, DEVIATION_5_PCT)?;
    Ok(())
  }

  #[test]
  fn sell_clamps_to_tolerance() -> Result<()> {
    let wide_ci = OraclePrice {
      conf: UFix64::constant(14_000_000_000),
      ..ORACLE
    };
    let curve = SellPriceCurve::new(wide_ci, &SELL_CONFIG, DEVIATION_5_PCT)?;
    let max_delta = ORACLE
      .spot
      .mul_div_ceil(DEVIATION_5_PCT, UFix64::<N9>::one())
      .ok_or(CoreError::RebalanceDeviationArithmetic)?;
    let expected_floor: IFix64<N9> = ORACLE
      .spot
      .checked_sub(&max_delta)
      .ok_or(CoreError::RebalanceDeviationArithmetic)?
      .narrow()
      .ok_or(CoreError::RebalancePriceConversion)?;
    let expected_ceil: IFix64<N9> = ORACLE
      .spot
      .checked_add(&max_delta)
      .ok_or(CoreError::RebalanceDeviationArithmetic)?
      .narrow()
      .ok_or(CoreError::RebalancePriceConversion)?;
    assert_eq!(curve.curve().y_min(), expected_floor);
    assert_eq!(curve.curve().y_max(), expected_ceil);
    Ok(())
  }

  #[test]
  fn buy_clamps_to_tolerance() -> Result<()> {
    let wide_ci = OraclePrice {
      conf: UFix64::constant(14_000_000_000),
      ..ORACLE
    };
    let curve = BuyPriceCurve::new(wide_ci, &BUY_CONFIG, DEVIATION_5_PCT)?;
    let max_delta = ORACLE
      .spot
      .mul_div_ceil(DEVIATION_5_PCT, UFix64::<N9>::one())
      .ok_or(CoreError::RebalanceDeviationArithmetic)?;
    let expected_floor: IFix64<N9> = ORACLE
      .spot
      .checked_sub(&max_delta)
      .ok_or(CoreError::RebalanceDeviationArithmetic)?
      .narrow()
      .ok_or(CoreError::RebalancePriceConversion)?;
    let expected_ceil: IFix64<N9> = ORACLE
      .spot
      .checked_add(&max_delta)
      .ok_or(CoreError::RebalanceDeviationArithmetic)?
      .narrow()
      .ok_or(CoreError::RebalancePriceConversion)?;
    assert_eq!(curve.curve().y_min(), expected_floor);
    assert_eq!(curve.curve().y_max(), expected_ceil);
    Ok(())
  }

  #[test]
  fn sell_flat_below_domain() -> Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG, DEVIATION_5_PCT)?;
    assert_eq!(curve.price(UCR_1_00)?, curve.price(UCR_1_15)?);
    Ok(())
  }

  #[test]
  fn sell_inactive_above_domain() -> Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG, DEVIATION_5_PCT)?;
    assert_eq!(
      curve.price(UCR_1_40).err(),
      Some(CoreError::RebalanceOutOfDomain.into())
    );
    Ok(())
  }

  #[test]
  fn sell_endpoints() -> Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG, DEVIATION_5_PCT)?;
    let at_floor = curve.price(UCR_1_20)?;
    let at_ceil = curve.price(UCR_1_35)?;
    assert_lt!(at_floor, at_ceil);
    assert_eq!(at_floor, curve.price(UCR_1_00)?);
    Ok(())
  }

  #[test]
  fn buy_inactive_below_domain() -> Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG, DEVIATION_5_PCT)?;
    assert_eq!(
      curve.price(UCR_1_60).err(),
      Some(CoreError::RebalanceOutOfDomain.into())
    );
    Ok(())
  }

  #[test]
  fn buy_flat_above_domain() -> Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG, DEVIATION_5_PCT)?;
    assert_eq!(curve.price(UCR_1_80)?, curve.price(UCR_2_50)?);
    Ok(())
  }

  #[test]
  fn buy_endpoints() -> Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG, DEVIATION_5_PCT)?;
    let at_floor = curve.price(UCR_1_65)?;
    let at_ceil = curve.price(UCR_1_75)?;
    assert_lt!(at_floor, at_ceil);
    assert_eq!(at_ceil, curve.price(UCR_2_50)?);
    Ok(())
  }

  fn sell_cr() -> BoxedStrategy<UFix64<N9>> {
    (1_000_000_000u64..1_350_000_000)
      .prop_map(UFix64::new)
      .boxed()
  }

  fn buy_cr() -> BoxedStrategy<UFix64<N9>> {
    (1_650_000_000u64..4_000_000_000)
      .prop_map(UFix64::new)
      .boxed()
  }

  fn oracle_spot() -> BoxedStrategy<UFix64<N9>> {
    (10_000_000_000u64..1_000_000_000_000)
      .prop_map(UFix64::new)
      .boxed()
  }

  fn oracle_ci() -> BoxedStrategy<UFix64<N9>> {
    (10_000u64..500_000_000).prop_map(UFix64::new).boxed()
  }

  proptest! {
    #[test]
    fn sell_price_valid(
      cr in sell_cr(),
      spot in oracle_spot(),
      conf in oracle_ci(),
    ) {
      let oracle = OraclePrice { spot, conf };
      if let Ok(curve) = SellPriceCurve::new(oracle, &SELL_CONFIG, DEVIATION_5_PCT) {
        let price = curve
          .price(cr)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
        if let (Some(floor), Some(ceil)) = (
          curve.curve().y_min().narrow(),
          curve.curve().y_max().narrow(),
        ) {
          prop_assert!(price >= floor && price <= ceil);
        } else {
          Err(TestCaseError::fail("floor/ceil narrow"))?;
        }
      }
    }

    #[test]
    fn buy_price_valid(
      cr in buy_cr(),
      spot in oracle_spot(),
      conf in oracle_ci(),
    ) {
      let oracle = OraclePrice { spot, conf };
      if let Ok(curve) = BuyPriceCurve::new(oracle, &BUY_CONFIG, DEVIATION_5_PCT) {
        let price = curve
          .price(cr)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
        if let (Some(floor), Some(ceil)) = (
          curve.curve().y_min().narrow(),
          curve.curve().y_max().narrow(),
        ) {
          prop_assert!(price >= floor && price <= ceil);
        } else {
          Err(TestCaseError::fail("floor/ceil narrow"))?;
        }
      }
    }

    /// Sell-side loss per unit traded never exceeds `spot * tolerance`.
    #[test]
    fn sell_curve_loss_bounded(
      cr in sell_cr(),
      spot in oracle_spot(),
      conf in oracle_ci(),
    ) {
      let oracle = OraclePrice { spot, conf };
      if let Ok(curve) = SellPriceCurve::new(oracle, &SELL_CONFIG, DEVIATION_5_PCT) {
        let max_delta = spot
          .mul_div_ceil(DEVIATION_5_PCT, UFix64::<N9>::one())
          .ok_or(TestCaseError::fail("max_delta"))?;
        let price = curve
          .price(cr)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert!(spot.abs_diff(&price) <= max_delta);
      }
    }

    /// Buy-side overpayment per unit traded never exceeds `spot * deviation`.
    #[test]
    fn buy_curve_overpayment_bounded(
      cr in buy_cr(),
      spot in oracle_spot(),
      conf in oracle_ci(),
    ) {
      let oracle = OraclePrice { spot, conf };
      if let Ok(curve) = BuyPriceCurve::new(oracle, &BUY_CONFIG, DEVIATION_5_PCT) {
        let max_delta = spot
          .mul_div_ceil(DEVIATION_5_PCT, UFix64::<N9>::one())
          .ok_or(TestCaseError::fail("max_delta"))?;
        let price = curve
          .price(cr)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert!(spot.abs_diff(&price) <= max_delta);
      }
    }
  }
}

#[cfg(kani)]
mod proofs {
  use fix::prelude::*;

  use super::{clamp_to_tolerance_inner, MAX_DEVIATION_PCT};
  use crate::kani_generators::narrow_ufix64;

  fn tolerance_bps() -> UFix64<N9> {
    let t: UFix64<N9> = narrow_ufix64();
    kani::assume(t > UFix64::zero() && t <= MAX_DEVIATION_PCT);
    t
  }

  /// `|spot - clamp_to_tolerance(spot, _, tol)| <= spot * tol`.
  #[kani::proof]
  fn clamp_band_membership() {
    let spot: UFix64<N9> = narrow_ufix64();
    let projected: UFix64<N9> = narrow_ufix64();
    let tol = tolerance_bps();
    let clamped = clamp_to_tolerance_inner(spot, projected, tol);
    let max_delta = spot.mul_div_ceil(tol, UFix64::<N9>::one());
    clamped
      .zip(max_delta)
      .map(|(c, delta)| assert!(spot.abs_diff(&c) <= delta));
  }
}
