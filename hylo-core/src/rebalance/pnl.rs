use std::cmp::Ordering::{Equal, Greater, Less};

use anchor_lang::prelude::*;
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError::{RebalancePnlCacheNet, RebalancePnlCacheUpdate};

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

  /// Computes `PnL` from protocol virtual stablecoin perspective.
  #[must_use]
  pub fn from_stablecoin(
    stablecoin_value_in: UFix64<N6>,
    stablecoin_value_out: UFix64<N6>,
  ) -> Option<RebalancePnl> {
    let input = stablecoin_value_in.checked_convert()?;
    let output = stablecoin_value_out.checked_convert()?;
    match input.cmp(&output) {
      Less => {
        let delta = output.checked_sub(&input)?;
        Some(RebalancePnl::Loss(delta))
      }
      Greater => {
        let delta = input.checked_sub(&output)?;
        Some(RebalancePnl::Profit(delta))
      }
      Equal => Some(RebalancePnl::NoChange),
    }
  }
}

/// Two register counter tracking rebalancing `PnL`.
#[derive(
  Debug,
  Clone,
  Copy,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  Serialize,
  Deserialize,
)]
pub struct RebalancePnlCache {
  profit: UFixValue64,
  loss: UFixValue64,
}

impl Default for RebalancePnlCache {
  fn default() -> Self {
    let zero = UFix64::<N9>::zero();
    RebalancePnlCache {
      profit: zero.into(),
      loss: zero.into(),
    }
  }
}

impl RebalancePnlCache {
  #[must_use]
  pub fn new() -> RebalancePnlCache {
    RebalancePnlCache::default()
  }

  pub fn profit(&self) -> Result<UFix64<N9>> {
    self.profit.try_into()
  }

  pub fn loss(&self) -> Result<UFix64<N9>> {
    self.loss.try_into()
  }

  fn apply_profit(&mut self, profit: UFix64<N9>) -> Result<()> {
    let current = self.profit()?;
    let updated = current
      .checked_add(&profit)
      .ok_or(RebalancePnlCacheUpdate)?;
    self.profit = updated.into();
    Ok(())
  }

  fn apply_loss(&mut self, loss: UFix64<N9>) -> Result<()> {
    let current = self.loss()?;
    let updated = current.checked_add(&loss).ok_or(RebalancePnlCacheUpdate)?;
    self.loss = updated.into();
    Ok(())
  }

  pub fn update_pnl(&mut self, rebalance_pnl: RebalancePnl) -> Result<()> {
    match rebalance_pnl {
      RebalancePnl::Profit(amount) => self.apply_profit(amount),
      RebalancePnl::Loss(amount) => self.apply_loss(amount),
      RebalancePnl::NoChange => Ok(()),
    }
  }

  pub fn net_pnl(&self) -> Result<RebalancePnl> {
    let profit = self.profit()?;
    let loss = self.loss()?;
    match profit.cmp(&loss) {
      Less => {
        let delta = loss.checked_sub(&profit).ok_or(RebalancePnlCacheNet)?;
        Ok(RebalancePnl::Loss(delta))
      }
      Greater => {
        let delta = profit.checked_sub(&loss).ok_or(RebalancePnlCacheNet)?;
        Ok(RebalancePnl::Profit(delta))
      }
      Equal => Ok(RebalancePnl::NoChange),
    }
  }

  pub fn clear(&mut self) {
    *self = RebalancePnlCache::new();
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

  #[test]
  fn accumulates_to_net_profit() -> Result<()> {
    let mut cache = RebalancePnlCache::default();
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(137)))?;
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(241)))?;
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(89)))?;
    cache.update_pnl(RebalancePnl::Loss(UFix64::new(76)))?;
    assert_eq!(cache.net_pnl()?, RebalancePnl::Profit(UFix64::new(391)));
    Ok(())
  }

  #[test]
  fn accumulates_to_net_loss() -> Result<()> {
    let mut cache = RebalancePnlCache::default();
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(53)))?;
    cache.update_pnl(RebalancePnl::Loss(UFix64::new(137)))?;
    cache.update_pnl(RebalancePnl::Loss(UFix64::new(89)))?;
    assert_eq!(cache.net_pnl()?, RebalancePnl::Loss(UFix64::new(173)));
    Ok(())
  }

  #[test]
  fn equal_registers_net_to_no_change() -> Result<()> {
    let mut cache = RebalancePnlCache::default();
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(137)))?;
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(89)))?;
    cache.update_pnl(RebalancePnl::Loss(UFix64::new(226)))?;
    assert_eq!(cache.net_pnl()?, RebalancePnl::NoChange);
    Ok(())
  }

  #[test]
  fn no_change_update_is_noop() -> Result<()> {
    let mut cache = RebalancePnlCache::default();
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(241)))?;
    cache.update_pnl(RebalancePnl::NoChange)?;
    cache.update_pnl(RebalancePnl::Loss(UFix64::new(76)))?;
    assert_eq!(cache.profit()?, UFix64::new(241));
    assert_eq!(cache.loss()?, UFix64::new(76));
    Ok(())
  }

  #[test]
  fn clear_resets_after_updates() -> Result<()> {
    let mut cache = RebalancePnlCache::default();
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(241)))?;
    cache.update_pnl(RebalancePnl::Loss(UFix64::new(89)))?;
    cache.clear();
    assert_eq!(cache.profit()?, UFix64::zero());
    assert_eq!(cache.loss()?, UFix64::zero());
    assert_eq!(cache.net_pnl()?, RebalancePnl::NoChange);
    Ok(())
  }

  #[test]
  fn overflow_returns_err() -> Result<()> {
    let mut cache = RebalancePnlCache::default();
    cache.update_pnl(RebalancePnl::Profit(UFix64::new(u64::MAX)))?;
    let result = cache.update_pnl(RebalancePnl::Profit(UFix64::new(1)));
    assert_eq!(result.err(), Some(RebalancePnlCacheUpdate.into()));
    Ok(())
  }
}
