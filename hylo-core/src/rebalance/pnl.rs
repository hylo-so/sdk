use std::cmp::Ordering::{Equal, Greater, Less};

use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

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

impl TryFrom<RebalancePnlValue> for RebalancePnl {
  type Error = CoreError;

  fn try_from(pnl: RebalancePnlValue) -> Result<RebalancePnl, CoreError> {
    match pnl {
      RebalancePnlValue::Profit(profit) => {
        Ok(RebalancePnl::Profit(profit.try_into()?))
      }
      RebalancePnlValue::Loss(loss) => Ok(RebalancePnl::Loss(loss.try_into()?)),
      RebalancePnlValue::NoChange => Ok(RebalancePnl::NoChange),
    }
  }
}

/// Serializable version of [`RebalancePnl`].
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
pub enum RebalancePnlValue {
  Profit(UFixValue64),
  Loss(UFixValue64),
  NoChange,
}

impl From<RebalancePnl> for RebalancePnlValue {
  fn from(pnl: RebalancePnl) -> Self {
    match pnl {
      RebalancePnl::Profit(profit) => RebalancePnlValue::Profit(profit.into()),
      RebalancePnl::Loss(loss) => RebalancePnlValue::Loss(loss.into()),
      RebalancePnl::NoChange => RebalancePnlValue::NoChange,
    }
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
}

#[cfg(kani)]
mod proofs {
  use fix::prelude::*;

  use crate::kani_generators::any_ufix64;
  use crate::rebalance::pnl::RebalancePnl;

  /// `from_stablecoin_flow` never returns `None` for any `(in, out)` pair.
  #[kani::proof]
  fn from_stablecoin_flow_always_some() {
    let in_amount: UFix64<N6> = any_ufix64();
    let out_amount: UFix64<N6> = any_ufix64();
    assert!(RebalancePnl::from_stablecoin_flow(in_amount, out_amount).is_some());
  }
}
