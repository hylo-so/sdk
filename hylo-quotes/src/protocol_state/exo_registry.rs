//! EXO registry decoding and account key derivation.

use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;
use anyhow::{anyhow, ensure, Context, Result};
use hylo_core::error::CoreError;
use hylo_core::idl::exchange::accounts::ExoPair;
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

/// Checks that the pair's oracle matches the feed the SDK fetches for its
/// collateral.
///
/// # Errors
/// * Collateral mint has no feed
/// * Oracle address or feed id differs from the SDK feed
pub fn validate_exo_pair_oracle(exo_pair: &ExoPair) -> Result<()> {
  let feed = exo_pyth_feed_by_mint(exo_pair.collateral_mint)?;
  ensure!(
    exo_pair.oracle == feed.address,
    "EXO pair oracle {} does not match SDK feed {}",
    exo_pair.oracle,
    feed.address,
  );
  ensure!(
    exo_pair.oracle_feed_id == feed.feed_id,
    "EXO pair feed id does not match SDK feed for {}",
    exo_pair.collateral_mint,
  );
  Ok(())
}

/// Decodes the registry from raw account data.
///
/// # Errors
/// * Discriminator mismatch
/// * Payload size mismatch
pub fn read_exo_registry(data: &[u8]) -> Result<ExoRegistry> {
  let payload = data
    .strip_prefix(ExoRegistry::DISCRIMINATOR)
    .context("EXO registry discriminator mismatch")?;
  bytemuck::try_pod_read_unaligned(payload)
    .map_err(|error| anyhow!("EXO registry deserialization: {error}"))
}

/// Registered entries.
///
/// # Errors
/// * `current_size` exceeds capacity
pub fn exo_registry_entries(registry: &ExoRegistry) -> Result<&[ExoEntry]> {
  registry
    .entries
    .get(..usize::from(registry.current_size))
    .context("EXO registry length exceeds capacity")
}

/// Pubkeys for every registry entry in fetch order:
///
/// * All pair groups first: exo pair, vault, levercoin mint, collateral mint
/// * Then all collateral/USD Pyth feeds, one per entry
///
/// # Errors
/// * Collateral mint has no feed
pub fn exo_pubkeys_from_entries(
  entries: &[ExoEntry],
) -> Result<Vec<Pubkey>, CoreError> {
  let feed_keys = entries
    .iter()
    .map(|entry| {
      exo_pyth_feed_by_mint(entry.collateral_mint).map(|feed| feed.address)
    })
    .collect::<Result<Vec<_>, _>>()?;
  let pair_keys = entries.iter().flat_map(|entry| {
    [
      pda::exo_pair(entry.collateral_mint),
      pda::exo_vault(entry.collateral_mint),
      entry.levercoin_mint,
      entry.collateral_mint,
    ]
  });
  Ok(pair_keys.chain(feed_keys).collect())
}
