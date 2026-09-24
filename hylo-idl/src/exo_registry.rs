use anyhow::{Context, Result};

use crate::router::accounts::ExoRegistry;
use crate::router::types::ExoEntry;

impl ExoRegistry {
  /// Registered entries.
  ///
  /// # Errors
  /// * `current_size` exceeds capacity
  pub fn registered_entries(&self) -> Result<&[ExoEntry]> {
    self
      .entries
      .get(..usize::from(self.current_size))
      .context("Exo registry length exceeds capacity")
  }
}

#[cfg(test)]
mod tests {
  use std::array::from_fn;

  use anchor_lang::prelude::Pubkey;
  use anyhow::Result;

  use crate::pda;
  use crate::router::accounts::ExoRegistry;
  use crate::router::types::ExoEntry;
  use crate::tokens::{Exo, TokenMint, CBBTC, ZEC};

  fn entry<E: Exo>() -> ExoEntry {
    ExoEntry {
      collateral_mint: E::MINT,
      levercoin_mint: pda::exo_levercoin_mint(E::MINT),
    }
  }

  fn registry(current_size: u8) -> ExoRegistry {
    let registered = [entry::<CBBTC>(), entry::<ZEC>()];
    let entries = from_fn(|slot| registered[slot % registered.len()]);
    ExoRegistry {
      current_size,
      bump: 0,
      _pad: [0; 6],
      entries,
    }
  }

  #[test]
  fn entries_truncate_to_current_size() -> Result<()> {
    let collateral_mints = registry(2)
      .registered_entries()?
      .iter()
      .map(|entry| entry.collateral_mint)
      .collect::<Vec<Pubkey>>();
    assert_eq!(collateral_mints, [CBBTC::MINT, ZEC::MINT]);
    Ok(())
  }

  #[test]
  fn empty_registry_has_no_entries() -> Result<()> {
    assert!(registry(0).registered_entries()?.is_empty());
    Ok(())
  }

  #[test]
  fn current_size_beyond_capacity_errors() {
    assert!(registry(u8::MAX).registered_entries().is_err());
  }
}
