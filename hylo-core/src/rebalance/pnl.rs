use std::cmp::Ordering::{Equal, Greater, Less};

use anchor_lang::prelude::*;
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError::{RebalancePnlCacheNet, RebalancePnlCacheUpdate};

/// Profit or loss ensuing from a rebalancing trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebalancePnl {
  Profit(UFix64<N6>),
  Loss(UFix64<N6>),
  NoChange,
}

impl RebalancePnl {
  /// Computes `PnL` from protocol virtual stablecoin perspective.
  #[must_use]
  pub fn from_stablecoin_flow(
    stablecoin_value_in: UFix64<N6>,
    stablecoin_value_out: UFix64<N6>,
  ) -> Option<RebalancePnl> {
    match stablecoin_value_in.cmp(&stablecoin_value_out) {
      Less => {
        let delta = stablecoin_value_out.checked_sub(&stablecoin_value_in)?;
        Some(RebalancePnl::Loss(delta))
      }
      Greater => {
        let delta = stablecoin_value_in.checked_sub(&stablecoin_value_out)?;
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
    let zero = UFix64::<N6>::zero();
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

  pub fn profit(&self) -> Result<UFix64<N6>> {
    self.profit.try_into()
  }

  pub fn loss(&self) -> Result<UFix64<N6>> {
    self.loss.try_into()
  }

  fn apply_profit(&mut self, profit: UFix64<N6>) -> Result<()> {
    let current = self.profit()?;
    let updated = current
      .checked_add(&profit)
      .ok_or(RebalancePnlCacheUpdate)?;
    self.profit = updated.into();
    Ok(())
  }

  fn apply_loss(&mut self, loss: UFix64<N6>) -> Result<()> {
    let current = self.loss()?;
    let updated = current.checked_add(&loss).ok_or(RebalancePnlCacheUpdate)?;
    self.loss = updated.into();
    Ok(())
  }

  /// Applies the current swap's `PnL` value to the cache.
  pub fn update_pnl(&mut self, rebalance_pnl: RebalancePnl) -> Result<()> {
    match rebalance_pnl {
      RebalancePnl::Profit(amount) => self.apply_profit(amount),
      RebalancePnl::Loss(amount) => self.apply_loss(amount),
      RebalancePnl::NoChange => Ok(()),
    }
  }

  /// Computes net of profit and loss, reflected as [`RebalancePnl`].
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

  /// Resets `PnL` cache to zero.
  pub fn clear(&mut self) {
    *self = RebalancePnlCache::new();
  }

  /// Checks if cache shows an unchanged net `PnL`.
  #[must_use]
  pub fn is_settled(&self) -> bool {
    self
      .net_pnl()
      .is_ok_and(|pnl| matches!(pnl, RebalancePnl::NoChange))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn from_stablecoin_flow_profit() {
    assert_eq!(
      RebalancePnl::from_stablecoin_flow(UFix64::new(421), UFix64::new(158)),
      Some(RebalancePnl::Profit(UFix64::new(263))),
    );
  }

  #[test]
  fn from_stablecoin_flow_loss() {
    assert_eq!(
      RebalancePnl::from_stablecoin_flow(UFix64::new(84), UFix64::new(237)),
      Some(RebalancePnl::Loss(UFix64::new(153))),
    );
  }

  #[test]
  fn from_stablecoin_flow_no_change() {
    assert_eq!(
      RebalancePnl::from_stablecoin_flow(UFix64::new(314), UFix64::new(314)),
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
