//! Exo registry decoding and account key derivation.

use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;
use anyhow::{anyhow, Context, Result};
use hylo_core::error::CoreError;
use hylo_core::idl::router::accounts::ExoRegistry;
use hylo_core::idl::router::types::ExoEntry;
use hylo_core::pyth::{PythFeed, PythOracle};
use hylo_idl::tokens::{TokenMint, CBBTC, HYPE, ONYC, PST, WETH, ZEC};
use hylo_idl::{pda, with_exo_pairs};

macro_rules! exo_pyth_feed_dispatch {
  ($(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?) => {
    /// Collateral/USD Pyth feed for an exo collateral mint known at runtime.
    ///
    /// # Errors
    /// * Mint backs no exo pair
    pub fn exo_pyth_feed_by_mint(
      collateral_mint: Pubkey,
    ) -> Result<PythFeed, CoreError> {
      match collateral_mint {
        $(<$exo>::MINT => Ok(<$exo>::FEED),)+
        _ => Err(CoreError::UnknownExoMint),
      }
    }
  };
}

with_exo_pairs!(exo_pyth_feed_dispatch);

/// Decodes the registry from raw account data.
///
/// # Errors
/// * Discriminator mismatch
/// * Payload size mismatch
pub fn read_exo_registry(data: &[u8]) -> Result<ExoRegistry> {
  let payload = data
    .strip_prefix(ExoRegistry::DISCRIMINATOR)
    .context("Exo registry discriminator mismatch")?;
  bytemuck::try_pod_read_unaligned(payload)
    .map_err(|error| anyhow!("Exo registry deserialization: {error}"))
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

/// Five pubkeys per registry entry, in `ExoPairAccounts` field order.
///
/// # Errors
/// * Collateral mint has no feed
pub fn exo_pubkeys_from_entries(
  entries: &[ExoEntry],
) -> Result<Vec<Pubkey>, CoreError> {
  entries
    .iter()
    .map(|entry| {
      let feed = exo_pyth_feed_by_mint(entry.collateral_mint)?;
      Ok([
        pda::exo_pair(entry.collateral_mint),
        pda::exo_vault(entry.collateral_mint),
        entry.levercoin_mint,
        entry.collateral_mint,
        feed.address,
      ])
    })
    .collect::<Result<Vec<_>, CoreError>>()
    .map(|groups| groups.into_iter().flatten().collect())
}
