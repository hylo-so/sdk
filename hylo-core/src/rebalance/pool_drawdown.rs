use anchor_lang::prelude::*;
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::virtual_stablecoin::VirtualStablecoin;

/// Outstanding hyUSD debt owed to the earn pool after a Depeg absorption.
#[derive(
  Debug,
  Clone,
  Copy,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  Serialize,
  Deserialize,
  PartialEq,
  Eq,
)]
pub struct PoolDrawdown {
  ledger: VirtualStablecoin,
}

impl Default for PoolDrawdown {
  fn default() -> PoolDrawdown {
    let ledger = VirtualStablecoin::default();
    PoolDrawdown { ledger }
  }
}

impl PoolDrawdown {
  /// Remaining debt to be paid.
  ///
  /// # Errors
  /// * Underlying conversion
  pub fn outstanding(&self) -> Result<UFix64<N6>> {
    self.ledger.supply()
  }

  /// Increment the debt drawdown.
  ///
  /// # Errors
  /// * Underlying arithmetic
  pub fn drawdown(&mut self, amount: UFix64<N6>) -> Result<()> {
    self.ledger.mint(amount)
  }

  /// Burn down debt.
  ///
  /// # Errors
  /// * Underlying arithmetic
  pub fn repay(&mut self, amount: UFix64<N6>) -> Result<()> {
    self.ledger.burn(amount)
  }

  /// Checks that debt is entirely zeroed.
  #[must_use]
  pub fn is_repaid(&self) -> bool {
    *self == PoolDrawdown::default()
  }
}
