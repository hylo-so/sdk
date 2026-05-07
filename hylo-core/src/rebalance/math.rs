use std::cmp::Ordering::{Equal, Greater, Less};

use fix::prelude::*;

/// Max sellable collateral until exo pair CR rises to target.
///
/// ```text
///   target_cr * virtual_stablecoin - collateral_usd_price * total_collateral
///   ────────────────────────────────────────────────────────────────────────
///                  collateral_usd_price * (target_cr - 1)
/// ```
#[must_use]
pub fn max_sellable_collateral(
  target_cr: UFix64<N9>,
  virtual_stablecoin: UFix64<N6>,
  collateral_usd_price: UFix64<N9>,
  total_collateral: UFix64<N9>,
) -> Option<UFix64<N9>> {
  let virtual_stablecoin = virtual_stablecoin.checked_convert::<N9>()?;
  let num_1 = target_cr.mul_div_floor(virtual_stablecoin, UFix64::one())?;
  let num_2 =
    collateral_usd_price.mul_div_ceil(total_collateral, UFix64::one())?;
  let num = num_1.checked_sub(&num_2)?;
  let denom_2 = target_cr.checked_sub(&UFix64::one())?;
  let denom = collateral_usd_price.mul_div_ceil(denom_2, UFix64::one())?;
  num.mul_div_floor(UFix64::one(), denom)
}

/// Max buyable collateral until exo pair CR falls to the target.
///
/// ```text
///   collateral_usd_price * total_collateral - target_cr * virtual_stablecoin
///   ────────────────────────────────────────────────────────────────────────
///                  collateral_usd_price * (target_cr - 1)
/// ```
#[must_use]
pub fn max_buyable_collateral(
  target_cr: UFix64<N9>,
  virtual_stablecoin: UFix64<N6>,
  collateral_usd_price: UFix64<N9>,
  total_collateral: UFix64<N9>,
) -> Option<UFix64<N9>> {
  let virtual_stablecoin = virtual_stablecoin.checked_convert::<N9>()?;
  let num_1 =
    collateral_usd_price.mul_div_floor(total_collateral, UFix64::one())?;
  let num_2 = target_cr.mul_div_ceil(virtual_stablecoin, UFix64::one())?;
  let num = num_1.checked_sub(&num_2)?;
  let denom_2 = target_cr.checked_sub(&UFix64::one())?;
  let denom = collateral_usd_price.mul_div_ceil(denom_2, UFix64::one())?;
  num.mul_div_floor(UFix64::one(), denom)
}

/// Profit or loss ensuing from a rebalancing trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebalancePnl {
  Profit(UFix64<N9>),
  Loss(UFix64<N9>),
  NoChange,
}

impl RebalancePnl {
  /// Determines `PnL` during sell-side rebalancing.
  #[must_use]
  pub fn sell_side(
    spot_price: UFix64<N9>,
    rebalance_price: UFix64<N9>,
    collateral_amount: UFix64<N9>,
  ) -> Option<RebalancePnl> {
    match rebalance_price.cmp(&spot_price) {
      Less => {
        let delta = spot_price.checked_sub(&rebalance_price)?;
        let loss = delta.mul_div_ceil(collateral_amount, UFix64::one())?;
        Some(RebalancePnl::Loss(loss))
      }
      Greater => {
        let delta = rebalance_price.checked_sub(&spot_price)?;
        let profit = delta.mul_div_floor(collateral_amount, UFix64::one())?;
        Some(RebalancePnl::Profit(profit))
      }
      Equal => Some(RebalancePnl::NoChange),
    }
  }

  /// Determines `PnL` during buy-side rebalancing.
  #[must_use]
  pub fn buy_side(
    spot_price: UFix64<N9>,
    rebalance_price: UFix64<N9>,
    collateral_amount: UFix64<N9>,
  ) -> Option<RebalancePnl> {
    match rebalance_price.cmp(&spot_price) {
      Less => {
        let delta = spot_price.checked_sub(&rebalance_price)?;
        let profit = delta.mul_div_floor(collateral_amount, UFix64::one())?;
        Some(RebalancePnl::Profit(profit))
      }
      Greater => {
        let delta = rebalance_price.checked_sub(&spot_price)?;
        let loss = delta.mul_div_ceil(collateral_amount, UFix64::one())?;
        Some(RebalancePnl::Loss(loss))
      }
      Equal => Some(RebalancePnl::NoChange),
    }
  }
}

#[cfg(test)]
mod tests {
  use anyhow::{Context, Result};
  use proptest::prelude::*;

  use super::*;

  const TARGET_CR: UFix64<N9> = UFix64::constant(1_500_000_000);

  fn spot_price() -> BoxedStrategy<UFix64<N9>> {
    (10_000_000_000u64..500_000_000_000)
      .prop_map(UFix64::new)
      .boxed()
  }

  fn collateral() -> BoxedStrategy<UFix64<N9>> {
    (1_000_000_000u64..1_000_000_000_000)
      .prop_map(UFix64::new)
      .boxed()
  }

  fn sell_side_cr() -> BoxedStrategy<UFix64<N9>> {
    (1_000_000_000u64..1_490_000_000)
      .prop_map(UFix64::new)
      .boxed()
  }

  fn buy_side_cr() -> BoxedStrategy<UFix64<N9>> {
    (1_510_000_000u64..4_000_000_000)
      .prop_map(UFix64::new)
      .boxed()
  }

  proptest! {
    #[test]
    fn sell_side_sanity(
      price in spot_price(),
      total in collateral(),
      cr in sell_side_cr(),
    ) {
      let stablecoin: UFix64<N6> = total
        .mul_div_ceil(price, cr)
        .map(UFix64::convert)
        .ok_or(TestCaseError::fail("derive stablecoin"))?;

      let result = max_sellable_collateral(
        TARGET_CR, stablecoin, price, total,
      );

      let sellable = result
        .ok_or(TestCaseError::fail("expected Some"))?;
      prop_assert!(sellable > UFix64::zero());
      prop_assert!(sellable <= total);
    }

    #[test]
    fn buy_side_sanity(
      price in spot_price(),
      total in collateral(),
      cr in buy_side_cr(),
    ) {
      let stablecoin: UFix64<N6> = total
        .mul_div_floor(price, cr)
        .map(UFix64::convert)
        .ok_or(TestCaseError::fail("derive stablecoin"))?;

      let result = max_buyable_collateral(
        TARGET_CR, stablecoin, price, total,
      );

      let buyable = result
        .ok_or(TestCaseError::fail("expected Some"))?;
      prop_assert!(buyable > UFix64::zero());
    }
  }

  #[test]
  fn sell_wrong_direction() {
    let price = UFix64::<N9>::new(100_000_000_000);
    let total = UFix64::<N9>::new(100_000_000_000);
    let stablecoin = UFix64::<N6>::new(5_000_000_000);
    assert_eq!(
      max_sellable_collateral(TARGET_CR, stablecoin, price, total),
      None,
    );
  }

  #[test]
  fn buy_wrong_direction() {
    let price = UFix64::<N9>::new(100_000_000_000);
    let total = UFix64::<N9>::new(100_000_000_000);
    let stablecoin = UFix64::<N6>::new(8_333_333_333);
    assert_eq!(
      max_buyable_collateral(TARGET_CR, stablecoin, price, total),
      None,
    );
  }

  #[test]
  fn sell_side_known_value() -> Result<()> {
    let price = UFix64::<N9>::new(100_000_000_000);
    let total = UFix64::<N9>::new(100_000_000_000);
    let stablecoin = UFix64::<N6>::new(8_000_000_000);
    let sellable = max_sellable_collateral(TARGET_CR, stablecoin, price, total)
      .context("max_sellable_collateral")?;
    let forty = UFix64::<N9>::new(40_000_000_000);
    assert_eq!(sellable, forty);
    Ok(())
  }

  #[test]
  fn buy_side_known_value() -> Result<()> {
    let price = UFix64::<N9>::new(100_000_000_000);
    let total = UFix64::<N9>::new(100_000_000_000);
    let stablecoin = UFix64::<N6>::new(5_000_000_000);
    let buyable = max_buyable_collateral(TARGET_CR, stablecoin, price, total)
      .context("max_buyable_collateral")?;
    let fifty = UFix64::<N9>::new(50_000_000_000);
    assert_eq!(buyable, fifty);
    Ok(())
  }

  #[test]
  fn pnl_sell_side_known_values() {
    // Rebalance price one unit below spot. Loss of 2, rounded up from 1.5.
    assert_eq!(
      RebalancePnl::sell_side(
        UFix64::new(100_000_000_000),
        UFix64::new(99_999_999_999),
        UFix64::new(1_500_000_000),
      ),
      Some(RebalancePnl::Loss(UFix64::new(2))),
    );
    // Rebalance price one unit above spot. Profit of 1, rounded down from 1.5.
    assert_eq!(
      RebalancePnl::sell_side(
        UFix64::new(100_000_000_000),
        UFix64::new(100_000_000_001),
        UFix64::new(1_500_000_000),
      ),
      Some(RebalancePnl::Profit(UFix64::new(1))),
    );
    // Rebalance price equals spot.
    assert_eq!(
      RebalancePnl::sell_side(
        UFix64::new(100_000_000_000),
        UFix64::new(100_000_000_000),
        UFix64::new(1_500_000_000),
      ),
      Some(RebalancePnl::NoChange),
    );
  }

  #[test]
  fn pnl_buy_side_known_values() {
    // Rebalance price one unit above spot. Loss of 2, rounded up from 1.5.
    assert_eq!(
      RebalancePnl::buy_side(
        UFix64::new(100_000_000_000),
        UFix64::new(100_000_000_001),
        UFix64::new(1_500_000_000),
      ),
      Some(RebalancePnl::Loss(UFix64::new(2))),
    );
    // Rebalance price one unit below spot. Profit of 1, rounded down from 1.5.
    assert_eq!(
      RebalancePnl::buy_side(
        UFix64::new(100_000_000_000),
        UFix64::new(99_999_999_999),
        UFix64::new(1_500_000_000),
      ),
      Some(RebalancePnl::Profit(UFix64::new(1))),
    );
    // Rebalance price equals spot.
    assert_eq!(
      RebalancePnl::buy_side(
        UFix64::new(100_000_000_000),
        UFix64::new(100_000_000_000),
        UFix64::new(1_500_000_000),
      ),
      Some(RebalancePnl::NoChange),
    );
  }
}
