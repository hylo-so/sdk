use anchor_lang::AccountDeserialize;
use anyhow::{Context, Result};
use hylo_idl::pda;
use hylo_idl::router::accounts::ExoRegistry;
use hylo_idl::router::types::ExoEntry;

use super::RouterClient;
use crate::program_client::ProgramClient;

impl RouterClient {
  /// Fetches the canonical exo registry.
  ///
  /// # Errors
  /// * Failed to fetch the account
  /// * Failed to deserialize account data
  pub async fn exo_registry(&self) -> Result<ExoRegistry> {
    let account = self.program().rpc().get_account(&pda::EXO_REGISTRY).await?;
    Ok(ExoRegistry::try_deserialize(&mut account.data.as_slice())?)
  }
}

/// Registered entries.
///
/// # Errors
/// * `current_size` exceeds capacity
pub fn exo_registry_entries(registry: &ExoRegistry) -> Result<&[ExoEntry]> {
  registry
    .entries
    .get(..usize::from(registry.current_size))
    .context("Exo registry length exceeds capacity")
}

#[cfg(test)]
mod tests {
  use anchor_client::solana_sdk::pubkey::Pubkey;
  use hylo_idl::router::accounts::ExoRegistry;
  use hylo_idl::router::types::ExoEntry;

  use super::exo_registry_entries;

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
    let collateral_mints = exo_registry_entries(&registry)
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
    assert!(exo_registry_entries(&registry).unwrap().is_empty());
  }

  #[test]
  fn current_size_beyond_capacity_errors() {
    let capacity = registry(0).entries.len();
    let registry = registry(u8::try_from(capacity).unwrap() + 1);
    assert!(exo_registry_entries(&registry).is_err());
  }
}
