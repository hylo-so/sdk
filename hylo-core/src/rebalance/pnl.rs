use std::cmp::Ordering::{Equal, Greater, Less};

use fix::prelude::*;

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
  use super::*;

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
