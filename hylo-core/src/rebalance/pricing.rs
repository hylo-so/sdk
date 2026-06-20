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

// Percent deviation boundaries
const MIN_DEVIATION_PCT: UFix64<N9> = UFix64::constant(1);
const MAX_DEVIATION_PCT: UFix64<N9> = UFix64::constant(20_000_000);

/// Floor/ceil deviation percentages for rebalance price curve construction.
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
  pub floor_pct: UFixValue64,
  pub ceil_pct: UFixValue64,
}

impl RebalanceCurveConfig {
  #[must_use]
  pub fn new(
    floor_pct: UFixValue64,
    ceil_pct: UFixValue64,
  ) -> RebalanceCurveConfig {
    RebalanceCurveConfig {
      floor_pct,
      ceil_pct,
    }
  }

  /// Converts floor percentage discount to `UFix64`.
  ///
  /// # Errors
  /// * Conversion fails
  pub fn floor_pct(&self) -> Result<UFix64<N9>> {
    self.floor_pct.try_into()
  }

  /// Converts ceil percentage premium to `UFix64`.
  ///
  /// # Errors
  /// * Conversion fails
  pub fn ceil_pct(&self) -> Result<UFix64<N9>> {
    self.ceil_pct.try_into()
  }

  /// Checks validity of floor/ceil percentages.
  ///
  /// # Errors
  /// * Incorrect precision or failed validation
  pub fn validate(self) -> Result<Self> {
    let ok = |pct| (MIN_DEVIATION_PCT..=MAX_DEVIATION_PCT).contains(&pct);
    (ok(self.floor_pct()?) && ok(self.ceil_pct()?))
      .then_some(self)
      .ok_or(CoreError::RebalanceCurveConfigValidation.into())
  }
}

/// Complement of a percentage: `1 − pct`.
///
/// # Errors
/// * Arithmetic underflow
fn complement(pct: UFix64<N9>) -> Result<UFix64<N9>> {
  UFix64::<N9>::one()
    .checked_sub(&pct)
    .ok_or(CoreError::RebalancePercentArithmetic.into())
}

/// Markup of a percentage: `1 + pct`.
///
/// # Errors
/// * Arithmetic overflow
fn markup(pct: UFix64<N9>) -> Result<UFix64<N9>> {
  UFix64::<N9>::one()
    .checked_add(&pct)
    .ok_or(CoreError::RebalancePercentArithmetic.into())
}

/// Convert unsigned CR to signed for curve lookup.
///
/// # Errors
/// * Conversion overflow
fn narrow(cr: UFix64<N9>) -> Result<IFix64<N9>> {
  cr.narrow::<i64>()
    .ok_or(CoreError::RebalancePriceConversion.into())
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
    let cr = narrow(ucr)?;
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
    OraclePrice { spot, .. }: OraclePrice,
    config: &RebalanceCurveConfig,
  ) -> Result<SellPriceCurve> {
    let floor_mult = config.floor_pct().and_then(complement)?;
    let ceil_mult = config.ceil_pct().and_then(markup)?;
    let (floor, ceil) = spot
      .mul_div_floor(floor_mult, UFix64::one())
      .zip(spot.mul_div_ceil(ceil_mult, UFix64::one()))
      .ok_or(CoreError::RebalancePriceConstruction)?;
    let sell_zone_1 = RebalanceMode::SellZone1.active_range();
    let curve = FixInterp::from_points([
      Point {
        x: narrow(sell_zone_1.start()?)?,
        y: narrow(floor)?,
      },
      Point {
        x: narrow(sell_zone_1.end()?)?,
        y: narrow(ceil)?,
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
    OraclePrice { spot, .. }: OraclePrice,
    config: &RebalanceCurveConfig,
  ) -> Result<BuyPriceCurve> {
    let floor_mult = config.floor_pct().and_then(complement)?;
    let ceil_mult = config.ceil_pct().and_then(markup)?;
    let (floor, ceil) = spot
      .mul_div_floor(floor_mult, UFix64::one())
      .zip(spot.mul_div_ceil(ceil_mult, UFix64::one()))
      .ok_or(CoreError::RebalancePriceConstruction)?;
    let buy_zone_1 = RebalanceMode::BuyZone1.active_range();
    let curve = FixInterp::from_points([
      Point {
        x: narrow(buy_zone_1.start()?)?,
        y: narrow(floor)?,
      },
      Point {
        x: narrow(buy_zone_1.end()?)?,
        y: narrow(ceil)?,
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

  const SELL_CONFIG: RebalanceCurveConfig = RebalanceCurveConfig {
    floor_pct: UFixValue64 {
      bits: 10_000_000,
      exp: -9,
    },
    ceil_pct: UFixValue64 {
      bits: 5_000_000,
      exp: -9,
    },
  };

  const BUY_CONFIG: RebalanceCurveConfig = RebalanceCurveConfig {
    floor_pct: UFixValue64 {
      bits: 5_000_000,
      exp: -9,
    },
    ceil_pct: UFixValue64 {
      bits: 10_000_000,
      exp: -9,
    },
  };

  const UCR_1_00: UFix64<N9> = UFix64::constant(1_000_000_000);
  const UCR_1_15: UFix64<N9> = UFix64::constant(1_150_000_000);
  const UCR_1_20: UFix64<N9> = UFix64::constant(1_200_000_000);
  const UCR_1_275: UFix64<N9> = UFix64::constant(1_275_000_000);
  const UCR_1_35: UFix64<N9> = UFix64::constant(1_350_000_000);
  const UCR_1_40: UFix64<N9> = UFix64::constant(1_400_000_000);
  const UCR_1_60: UFix64<N9> = UFix64::constant(1_600_000_000);
  const UCR_1_65: UFix64<N9> = UFix64::constant(1_650_000_000);
  const UCR_1_70: UFix64<N9> = UFix64::constant(1_700_000_000);
  const UCR_1_75: UFix64<N9> = UFix64::constant(1_750_000_000);
  const UCR_1_80: UFix64<N9> = UFix64::constant(1_800_000_000);
  const UCR_2_50: UFix64<N9> = UFix64::constant(2_500_000_000);

  #[test]
  fn sell_constructs() -> Result<()> {
    SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    Ok(())
  }

  #[test]
  fn buy_constructs() -> Result<()> {
    BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    Ok(())
  }

  #[test]
  fn sell_flat_below_domain() -> Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    assert_eq!(curve.price(UCR_1_00)?, curve.price(UCR_1_15)?);
    Ok(())
  }

  #[test]
  fn sell_inactive_above_domain() -> Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    assert_eq!(
      curve.price(UCR_1_40).err(),
      Some(CoreError::RebalanceOutOfDomain.into())
    );
    Ok(())
  }

  #[test]
  fn sell_endpoints() -> Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    let at_floor = curve.price(UCR_1_20)?;
    let at_ceil = curve.price(UCR_1_35)?;
    assert_lt!(at_floor, at_ceil);
    assert_eq!(at_floor, curve.price(UCR_1_00)?);
    Ok(())
  }

  #[test]
  fn buy_inactive_below_domain() -> Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    assert_eq!(
      curve.price(UCR_1_60).err(),
      Some(CoreError::RebalanceOutOfDomain.into())
    );
    Ok(())
  }

  #[test]
  fn buy_flat_above_domain() -> Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    assert_eq!(curve.price(UCR_1_80)?, curve.price(UCR_2_50)?);
    Ok(())
  }

  #[test]
  fn buy_endpoints() -> Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    let at_floor = curve.price(UCR_1_65)?;
    let at_ceil = curve.price(UCR_1_75)?;
    assert_lt!(at_floor, at_ceil);
    assert_eq!(at_ceil, curve.price(UCR_2_50)?);
    Ok(())
  }

  #[test]
  fn sell_endpoint_values() -> Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    assert_eq!(curve.price(UCR_1_20)?, UFix64::constant(144_937_098_276));
    assert_eq!(curve.price(UCR_1_35)?, UFix64::constant(147_133_114_917));
    Ok(())
  }

  #[test]
  fn buy_endpoint_values() -> Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    assert_eq!(curve.price(UCR_1_65)?, UFix64::constant(145_669_103_823));
    assert_eq!(curve.price(UCR_1_75)?, UFix64::constant(147_865_120_464));
    Ok(())
  }

  #[test]
  fn sell_midpoint_value() -> Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    assert_eq!(curve.price(UCR_1_275)?, UFix64::constant(146_035_106_597));
    Ok(())
  }

  #[test]
  fn buy_midpoint_value() -> Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    assert_eq!(curve.price(UCR_1_70)?, UFix64::constant(146_767_112_144));
    Ok(())
  }

  #[test]
  fn validate_accepts_in_range() -> Result<()> {
    assert_eq!(SELL_CONFIG.validate()?, SELL_CONFIG);
    assert_eq!(BUY_CONFIG.validate()?, BUY_CONFIG);
    Ok(())
  }

  #[test]
  fn validate_rejects_above_max_deviation() {
    let over_max = UFixValue64 {
      bits: 20_000_001,
      exp: -9,
    };
    let err = Some(CoreError::RebalanceCurveConfigValidation.into());
    let high_floor = RebalanceCurveConfig {
      floor_pct: over_max,
      ceil_pct: SELL_CONFIG.ceil_pct,
    };
    let high_ceil = RebalanceCurveConfig {
      floor_pct: SELL_CONFIG.floor_pct,
      ceil_pct: over_max,
    };
    assert_eq!(high_floor.validate().err(), err);
    assert_eq!(high_ceil.validate().err(), err);
  }

  #[test]
  fn validate_rejects_zero_band() {
    let config = RebalanceCurveConfig {
      floor_pct: UFixValue64 { bits: 0, exp: -9 },
      ceil_pct: UFixValue64 { bits: 0, exp: -9 },
    };
    assert_eq!(
      config.validate().err(),
      Some(CoreError::RebalanceCurveConfigValidation.into())
    );
  }

  #[test]
  fn floor_pct_above_one_underflows() {
    let config = RebalanceCurveConfig {
      floor_pct: UFixValue64 {
        bits: 1_010_000_000,
        exp: -9,
      },
      ceil_pct: SELL_CONFIG.ceil_pct,
    };
    assert_eq!(
      SellPriceCurve::new(ORACLE, &config).err(),
      Some(CoreError::RebalancePercentArithmetic.into())
    );
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

  fn spot_band(
    spot: UFix64<N9>,
    config: &RebalanceCurveConfig,
  ) -> Result<(UFix64<N9>, UFix64<N9>)> {
    let floor = spot
      .mul_div_floor(complement(config.floor_pct()?)?, UFix64::one())
      .ok_or(CoreError::RebalancePriceConstruction)?;
    let ceil = spot
      .mul_div_ceil(markup(config.ceil_pct()?)?, UFix64::one())
      .ok_or(CoreError::RebalancePriceConstruction)?;
    Ok((floor, ceil))
  }

  proptest! {
    #[test]
    fn sell_price_in_spot_band(
      cr in sell_cr(),
      spot in oracle_spot(),
      conf in oracle_ci(),
    ) {
      let oracle = OraclePrice { spot, conf };
      if let Ok(curve) = SellPriceCurve::new(oracle, &SELL_CONFIG) {
        let (floor, ceil) = spot_band(spot, &SELL_CONFIG)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
        let price = curve
          .price(cr)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert_eq!(curve.price(UCR_1_20)?, floor);
        prop_assert_eq!(curve.price(UCR_1_35)?, ceil);
        prop_assert!(price >= floor && price <= ceil);
      }
    }

    #[test]
    fn buy_price_in_spot_band(
      cr in buy_cr(),
      spot in oracle_spot(),
      conf in oracle_ci(),
    ) {
      let oracle = OraclePrice { spot, conf };
      if let Ok(curve) = BuyPriceCurve::new(oracle, &BUY_CONFIG) {
        let (floor, ceil) = spot_band(spot, &BUY_CONFIG)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
        let price = curve
          .price(cr)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert_eq!(curve.price(UCR_1_65)?, floor);
        prop_assert_eq!(curve.price(UCR_1_75)?, ceil);
        prop_assert!(price >= floor && price <= ceil);
      }
    }
  }
}
