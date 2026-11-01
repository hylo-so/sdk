//! Exo registry decoding and account key derivation.

use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;
use anyhow::{Context, Result};
use hylo_core::error::CoreError;
use hylo_core::idl::router::accounts::ExoRegistry;
use hylo_core::idl::router::types::ExoEntry;
use hylo_idl::pda;
use hylo_idl::tokens::{exo_role, ExoRole};

use crate::protocol_state::exo_pair::EXO_PAIR_ACCOUNTS;

/// Decodes the registered entries from raw registry account data.
///
/// # Errors
/// * Discriminator mismatch
/// * Payload size mismatch
/// * `current_size` exceeds capacity
pub fn read_exo_entries(data: &[u8]) -> Result<Vec<ExoEntry>> {
  let payload = data
    .strip_prefix(ExoRegistry::DISCRIMINATOR)
    .context("Exo registry discriminator mismatch")?;
  let registry: ExoRegistry = bytemuck::try_pod_read_unaligned(payload)
    .context("Exo registry deserialization")?;
  registry
    .entries
    .get(..usize::from(registry.current_size))
    .map(<[ExoEntry]>::to_vec)
    .context("Exo registry length exceeds capacity")
}

/// Entry whose collateral or levercoin is `mint_a` or `mint_b`.
#[must_use]
pub fn find_exo_entry(
  entries: &[ExoEntry],
  mint_a: Pubkey,
  mint_b: Pubkey,
) -> Option<&ExoEntry> {
  let mints = [mint_a, mint_b];
  entries.iter().find(|entry| {
    mints.contains(&entry.collateral_mint)
      || mints.contains(&entry.levercoin_mint)
  })
}

/// Role of `mint` in `entry`.
///
/// # Errors
/// * Mint is not routable through the entry
pub fn exo_entry_role(
  entry: &ExoEntry,
  mint: Pubkey,
) -> Result<ExoRole, CoreError> {
  exo_role(entry.collateral_mint, entry.levercoin_mint, mint)
    .ok_or(CoreError::UnknownExoMint)
}

/// [`EXO_PAIR_ACCOUNTS`] pubkeys per registry entry, in `ExoPairAccounts`
/// field order.
///
/// # Errors
/// * Arithmetic overflow while sizing the account list
pub fn exo_pubkeys_from_entries(
  entries: &[ExoEntry],
) -> Result<Vec<Pubkey>, CoreError> {
  entries.iter().try_fold(
    Vec::with_capacity(entries.len() * EXO_PAIR_ACCOUNTS),
    |mut keys, entry| {
      let entry_keys: [Pubkey; EXO_PAIR_ACCOUNTS] = [
        pda::exo_pair(entry.collateral_mint),
        pda::exo_vault(entry.collateral_mint),
        entry.levercoin_mint,
        entry.collateral_mint,
        pda::price_aggregate(entry.collateral_mint),
      ];
      keys.extend(entry_keys);
      Ok(keys)
    },
  )
}
