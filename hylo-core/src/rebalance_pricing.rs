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

use crate::error::CoreError;
use crate::interp::{FixInterp, Point};
use crate::pyth::OraclePrice;

/// Confidence interval multipliers for rebalance price curve construction.
#[derive(
  Copy, Clone, Debug, PartialEq, InitSpace, AnchorSerialize, AnchorDeserialize,
)]
pub struct RebalanceCurveConfig {
  floor_mult: UFixValue64,
  ceil_mult: UFixValue64,
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

  /// Checks both multipliers parse and are nonzero.
  ///
  /// # Errors
  /// * Multiplier has incorrect precision or is zero
  pub fn validate(self) -> Result<Self> {
    let valid =
      self.floor_mult()? > UFix64::zero() && self.ceil_mult()? > UFix64::zero();
    valid
      .then_some(self)
      .ok_or(CoreError::RebalanceCurveConfigValidation.into())
  }
}

// CR domain boundaries.
const CR_1_20: IFix64<N9> = IFix64::constant(1_200_000_000);
const CR_1_35: IFix64<N9> = IFix64::constant(1_350_000_000);
const CR_1_65: IFix64<N9> = IFix64::constant(1_650_000_000);
const CR_1_75: IFix64<N9> = IFix64::constant(1_750_000_000);

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

/// Interpolated rebalance price controller.
/// Implementors define boundary behavior via [`price_inner`].
pub trait RebalancePriceController {
  /// Reference to the underlying interpolator.
  fn curve(&self) -> &FixInterp<2, N9>;

  /// Compute price for CR from underlying curve with boundary handling.
  ///
  /// # Errors
  /// * Domain, arithmetic, or route-inactive errors.
  fn price_inner(&self, cr: IFix64<N9>) -> Result<IFix64<N9>>;

  /// Collateral price at the given CR.
  ///
  /// # Errors
  /// * CR conversion
  /// * Domain
  /// * Route inactive
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
  ) -> Result<SellPriceCurve> {
    let (floor, ceil) = spot
      .checked_sub(&scale_ci(conf, config.floor_mult()?)?)
      .zip(spot.checked_add(&scale_ci(conf, config.ceil_mult()?)?))
      .ok_or(CoreError::RebalancePriceConstruction)?;
    let curve = FixInterp::from_points([
      Point {
        x: CR_1_20,
        y: narrow_price(floor)?,
      },
      Point {
        x: CR_1_35,
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

  fn price_inner(&self, cr: IFix64<N9>) -> Result<IFix64<N9>> {
    let interp = self.curve();
    if cr < interp.x_min() {
      Ok(interp.y_min())
    } else if cr > interp.x_max() {
      Err(CoreError::RebalanceSellInactive.into())
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
  ) -> Result<BuyPriceCurve> {
    let (floor, ceil) = spot
      .checked_sub(&scale_ci(conf, config.floor_mult()?)?)
      .zip(spot.checked_add(&scale_ci(conf, config.ceil_mult()?)?))
      .ok_or(CoreError::RebalancePriceConstruction)?;
    let curve = FixInterp::from_points([
      Point {
        x: CR_1_65,
        y: narrow_price(floor)?,
      },
      Point {
        x: CR_1_75,
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

  fn price_inner(&self, cr: IFix64<N9>) -> Result<IFix64<N9>> {
    let interp = self.curve();
    if cr < interp.x_min() {
      Err(CoreError::RebalanceBuyInactive.into())
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
  fn sell_constructs() -> anyhow::Result<()> {
    SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    Ok(())
  }

  #[test]
  fn buy_constructs() -> anyhow::Result<()> {
    BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    Ok(())
  }

  #[test]
  fn sell_rejects_negative_ci() {
    let huge_ci = OraclePrice {
      conf: ORACLE.spot,
      ..ORACLE
    };
    let res = SellPriceCurve::new(huge_ci, &SELL_CONFIG);
    assert_eq!(
      res.err(),
      Some(CoreError::RebalancePriceConstruction.into())
    );
  }

  #[test]
  fn buy_rejects_negative_ci() {
    let huge_ci = OraclePrice {
      conf: ORACLE.spot,
      ..ORACLE
    };
    let res = BuyPriceCurve::new(huge_ci, &BUY_CONFIG);
    assert_eq!(
      res.err(),
      Some(CoreError::RebalancePriceConstruction.into())
    );
  }

  #[test]
  fn sell_flat_below_domain() -> anyhow::Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    assert_eq!(curve.price(UCR_1_00)?, curve.price(UCR_1_15)?);
    Ok(())
  }

  #[test]
  fn sell_inactive_above_domain() -> anyhow::Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    assert_eq!(
      curve.price(UCR_1_40).err(),
      Some(CoreError::RebalanceSellInactive.into())
    );
    Ok(())
  }

  #[test]
  fn sell_endpoints() -> anyhow::Result<()> {
    let curve = SellPriceCurve::new(ORACLE, &SELL_CONFIG)?;
    let at_floor = curve.price(UCR_1_20)?;
    let at_ceil = curve.price(UCR_1_35)?;
    assert_lt!(at_floor, at_ceil);
    assert_eq!(at_floor, curve.price(UCR_1_00)?);
    Ok(())
  }

  #[test]
  fn buy_inactive_below_domain() -> anyhow::Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    assert_eq!(
      curve.price(UCR_1_60).err(),
      Some(CoreError::RebalanceBuyInactive.into())
    );
    Ok(())
  }

  #[test]
  fn buy_flat_above_domain() -> anyhow::Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    assert_eq!(curve.price(UCR_1_80)?, curve.price(UCR_2_50)?);
    Ok(())
  }

  #[test]
  fn buy_endpoints() -> anyhow::Result<()> {
    let curve = BuyPriceCurve::new(ORACLE, &BUY_CONFIG)?;
    let at_floor = curve.price(UCR_1_65)?;
    let at_ceil = curve.price(UCR_1_75)?;
    assert_lt!(at_floor, at_ceil);
    assert_eq!(at_ceil, curve.price(UCR_2_50)?);
    Ok(())
  }

  fn sell_cr() -> BoxedStrategy<UFix64<N9>> {
    (0u64..1_350_000_000).prop_map(UFix64::new).boxed()
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
      if let Ok(curve) = SellPriceCurve::new(oracle, &SELL_CONFIG) {
        curve
          .price(cr)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
      }
    }

    #[test]
    fn buy_price_valid(
      cr in buy_cr(),
      spot in oracle_spot(),
      conf in oracle_ci(),
    ) {
      let oracle = OraclePrice { spot, conf };
      if let Ok(curve) = BuyPriceCurve::new(oracle, &BUY_CONFIG) {
        curve
          .price(cr)
          .map_err(|e| TestCaseError::fail(format!("{e}")))?;
      }
    }
  }
}
