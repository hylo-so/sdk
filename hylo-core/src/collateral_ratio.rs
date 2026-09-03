use fix::prelude::*;

use crate::error::CoreError;
use crate::error::CoreError::CollateralRatio as CollateralRatioOverflow;

/// Ratio of collateral value to stablecoin supply.
///
/// ```txt
///         total_collateral * usd_collateral_price
/// CR  =  -----------------------------------------
///                    amount_stablecoin
/// ```
///
/// `Infinite` when the denominator is zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollateralRatio {
  Finite(UFix64<N9>),
  Infinite,
}

pub type CR = CollateralRatio;

impl CollateralRatio {
  /// Computes the ratio from collateral value and stablecoin supply.
  /// Yields [`CollateralRatio::Infinite`] when the supply is zero.
  ///
  /// # Errors
  /// * Overflow computing the ratio
  pub fn new(
    total_collateral: UFix64<N9>,
    usd_collateral_price: UFix64<N9>,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<CollateralRatio, CoreError> {
    if amount_stablecoin == UFix64::zero() {
      Ok(CollateralRatio::Infinite)
    } else {
      amount_stablecoin
        .checked_convert::<N9>()
        .and_then(|stablecoin| {
          total_collateral.mul_div_floor(usd_collateral_price, stablecoin)
        })
        .map(CollateralRatio::Finite)
        .ok_or(CollateralRatioOverflow)
    }
  }

  /// Constructs a finite ratio.
  #[must_use]
  pub const fn finite(cr: UFix64<N9>) -> CollateralRatio {
    CollateralRatio::Finite(cr)
  }

  /// Yields the ratio, discarding `Infinite`.
  #[must_use]
  pub fn as_finite(self) -> Option<UFix64<N9>> {
    match self {
      CollateralRatio::Finite(cr) => Some(cr),
      CollateralRatio::Infinite => None,
    }
  }

  /// Converts to rebalance price curve coordinates.
  ///
  /// Saturates at `i64::MAX`, above every curve domain.
  #[must_use]
  pub fn price_curve_x(self) -> IFix64<N9> {
    match self {
      CollateralRatio::Infinite => IFix64::new(i64::MAX),
      CollateralRatio::Finite(cr) => {
        cr.narrow::<i64>().unwrap_or(IFix64::new(i64::MAX))
      }
    }
  }

  /// Converts to fee curve coordinates, truncating to `N5`.
  ///
  /// Saturates at `i64::MAX`, above every curve domain.
  #[must_use]
  pub fn fee_curve_x(self) -> IFix64<N5> {
    match self {
      CollateralRatio::Infinite => IFix64::new(i64::MAX),
      CollateralRatio::Finite(cr) => cr
        .convert::<N5>()
        .narrow::<i64>()
        .unwrap_or(IFix64::new(i64::MAX)),
    }
  }

  /// Tests a finite lower bound, which `Infinite` always satisfies.
  #[must_use]
  pub fn at_least(self, bound: UFix64<N9>) -> bool {
    self >= CR::finite(bound)
  }
}

impl From<CollateralRatio> for UFix64<N9> {
  fn from(cr: CollateralRatio) -> UFix64<N9> {
    cr.as_finite().unwrap_or(UFix64::new(u64::MAX))
  }
}

impl From<UFix64<N9>> for CollateralRatio {
  fn from(cr: UFix64<N9>) -> CollateralRatio {
    if cr == UFix64::new(u64::MAX) {
      CollateralRatio::Infinite
    } else {
      CR::finite(cr)
    }
  }
}

#[cfg(test)]
mod tests {
  use more_asserts::assert_lt;

  use super::*;

  const PRICE: UFix64<N9> = UFix64::constant(200_000_000_000);
  const COLLATERAL: UFix64<N9> = UFix64::constant(10_000_000_000);

  #[test]
  fn new_infinite_at_zero_supply() -> Result<(), CoreError> {
    let cr = CollateralRatio::new(COLLATERAL, PRICE, UFix64::zero())?;
    assert_eq!(cr, CollateralRatio::Infinite);
    Ok(())
  }

  #[test]
  fn new_finite_ratio() -> Result<(), CoreError> {
    let supply = UFix64::<N6>::constant(1_000_000_000);
    let cr = CollateralRatio::new(COLLATERAL, PRICE, supply)?;
    assert_eq!(cr, CR::finite(UFix64::constant(2_000_000_000)));
    Ok(())
  }

  #[test]
  fn new_below_par() -> Result<(), CoreError> {
    let total_collateral = UFix64::<N9>::new(8_217_712_567_008);
    let price = UFix64::<N9>::new(137_704_920_000);
    let supply = UFix64::<N6>::new(1_150_380_112_112);
    let cr = CollateralRatio::new(total_collateral, price, supply)?;
    assert_eq!(cr, CR::finite(UFix64::new(983_691_772)));
    Ok(())
  }

  #[test]
  fn new_above_par() -> Result<(), CoreError> {
    let total_collateral = UFix64::<N9>::new(976_123_127_719);
    let price = UFix64::<N9>::new(137_704_920_000);
    let supply = UFix64::<N6>::new(97_411_342_200);
    let cr = CollateralRatio::new(total_collateral, price, supply)?;
    assert_eq!(cr, CR::finite(UFix64::new(1_379_890_207)));
    Ok(())
  }

  #[test]
  fn infinite_orders_above_every_finite() {
    let max = CR::finite(UFix64::new(u64::MAX));
    assert_lt!(max, CollateralRatio::Infinite);
    assert_lt!(CR::finite(UFix64::zero()), max);
  }

  #[test]
  fn infinite_saturates_curve_space() {
    let infinite = CollateralRatio::Infinite;
    assert_eq!(infinite.price_curve_x(), IFix64::<N9>::new(i64::MAX));
    assert_eq!(infinite.fee_curve_x(), IFix64::<N5>::new(i64::MAX));
  }

  #[test]
  fn finite_beyond_signed_range_saturates() {
    let cr = CR::finite(UFix64::new(u64::MAX));
    assert_eq!(cr.price_curve_x(), IFix64::new(i64::MAX));
    assert_lt!(cr.fee_curve_x(), IFix64::new(i64::MAX));
  }

  #[test]
  fn finite_in_range_converts_exactly() {
    let cr = CR::finite(UFix64::constant(1_500_000_000));
    assert_eq!(cr.price_curve_x(), IFix64::constant(1_500_000_000));
    assert_eq!(cr.fee_curve_x(), IFix64::constant(150_000));
  }

  #[test]
  fn round_trip_infinite() {
    let raw: UFix64<N9> = CollateralRatio::Infinite.into();
    assert_eq!(raw, UFix64::new(u64::MAX));
    assert_eq!(CollateralRatio::from(raw), CollateralRatio::Infinite);
  }

  #[test]
  fn round_trip_finite() {
    let cr = CR::finite(UFix64::constant(1_500_000_000));
    let raw: UFix64<N9> = cr.into();
    assert_eq!(raw, UFix64::constant(1_500_000_000));
    assert_eq!(CollateralRatio::from(raw), cr);
  }

  #[test]
  fn at_least_finite_bound() {
    let bound = UFix64::<N9>::constant(1_350_000_000);
    assert!(CollateralRatio::Infinite.at_least(bound));
    assert!(CR::finite(bound).at_least(bound));
    assert!(!CR::finite(UFix64::constant(1_200_000_000)).at_least(bound));
  }
}
