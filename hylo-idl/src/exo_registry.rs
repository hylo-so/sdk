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
  use anchor_lang::prelude::Pubkey;

  use crate::router::accounts::ExoRegistry;
  use crate::router::types::ExoEntry;

  fn entry(byte: u8) -> ExoEntry {
    ExoEntry {
      collateral_mint: Pubkey::new_from_array([byte; 32]),
      levercoin_mint: Pubkey::new_from_array([byte.wrapping_add(1); 32]),
    }
  }

  fn registry(current_size: u8) -> ExoRegistry {
    let entries =
      std::array::from_fn(|index| entry(u8::try_from(index).unwrap()));
    ExoRegistry {
      current_size,
      bump: 0,
      _pad: [0; 6],
      entries,
    }
  }

  #[test]
  fn entries_truncate_to_current_size() {
    let registry = registry(2);
    let collateral_mints = registry
      .registered_entries()
      .unwrap()
      .iter()
      .map(|entry| entry.collateral_mint)
      .collect::<Vec<_>>();
    assert_eq!(
      collateral_mints,
      [entry(0).collateral_mint, entry(1).collateral_mint]
    );
  }

  #[test]
  fn empty_registry_has_no_entries() {
    let registry = registry(0);
    assert!(registry.registered_entries().unwrap().is_empty());
  }

  #[test]
  fn current_size_beyond_capacity_errors() {
    let capacity = registry(0).entries.len();
    let registry = registry(u8::try_from(capacity).unwrap() + 1);
    assert!(registry.registered_entries().is_err());
  }
}
