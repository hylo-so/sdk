//! Exo registry decoding and account key derivation.

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;
use anyhow::{Context, Result};
use hylo_core::error::CoreError;
use hylo_core::idl::router::accounts::ExoRegistry;
use hylo_core::idl::router::types::ExoEntry;
use hylo_core::pyth::{PythFeed, PythOracle};
use hylo_idl::tokens::{
  exo_role, ExoRole, TokenMint, CBBTC, HYPE, ONYC, PST, WETH, ZEC,
};
use hylo_idl::{pda, with_exo_pairs};

use crate::protocol_state::exo_pair::{exo_pair_oracle, EXO_PAIR_ACCOUNTS};

pub(crate) const EXO_ENTRY_ACCOUNTS: usize = 4;

macro_rules! exo_pyth_feed_dispatch {
  ($(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?) => {
    /// Collateral/USD Pyth feed for an exo collateral mint known at runtime.
    ///
    /// # Errors
    /// * Mint backs no exo pair
    #[deprecated(since = "2.7.6", note = "read the oracle from `ExoRoute`")]
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
pub fn find_exo_entry<'a>(
  entries: impl IntoIterator<Item = &'a ExoEntry>,
  mint_a: Pubkey,
  mint_b: Pubkey,
) -> Option<&'a ExoEntry> {
  entries
    .into_iter()
    .find(|entry| exo_entry_owns_either(entry, mint_a, mint_b))
}

fn exo_entry_owns_either(
  entry: &ExoEntry,
  mint_a: Pubkey,
  mint_b: Pubkey,
) -> bool {
  let mints = [mint_a, mint_b];
  mints.contains(&entry.collateral_mint)
    || mints.contains(&entry.levercoin_mint)
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

/// Derives the account pubkeys of one registry entry.
#[must_use]
pub fn exo_entry_pubkeys(entry: &ExoEntry) -> [Pubkey; EXO_ENTRY_ACCOUNTS] {
  [
    pda::exo_pair(entry.collateral_mint),
    pda::exo_vault(entry.collateral_mint),
    entry.levercoin_mint,
    entry.collateral_mint,
  ]
}

/// [`EXO_PAIR_ACCOUNTS`] pubkeys per registry entry, in `ExoPairAccounts`
/// field order.
///
/// # Errors
/// * Collateral mint has no feed
#[deprecated(since = "2.7.6", note = "use `ExoRoute::pubkeys`")]
#[allow(deprecated)]
pub fn exo_pubkeys_from_entries(
  entries: &[ExoEntry],
) -> Result<Vec<Pubkey>, CoreError> {
  entries.iter().try_fold(
    Vec::with_capacity(entries.len() * EXO_PAIR_ACCOUNTS),
    |mut keys, entry| {
      let feed = exo_pyth_feed_by_mint(entry.collateral_mint)?;
      let entry_keys: [Pubkey; EXO_PAIR_ACCOUNTS] = [
        pda::exo_pair(entry.collateral_mint),
        pda::exo_vault(entry.collateral_mint),
        entry.levercoin_mint,
        entry.collateral_mint,
        feed.address,
      ];
      keys.extend(entry_keys);
      Ok(keys)
    },
  )
}

/// Exo registry entry and its pair oracle.
#[derive(Clone)]
pub struct ExoRoute {
  pub entry: ExoEntry,
  pub oracle: Option<Pubkey>,
}

impl ExoRoute {
  /// Reads the oracle from the exo pair account of `entry`.
  ///
  /// # Errors
  /// * Account deserialization
  pub fn new(entry: ExoEntry, exo_pair: Option<&Account>) -> Result<ExoRoute> {
    let oracle = exo_pair.map(exo_pair_oracle).transpose()?;
    Ok(ExoRoute { entry, oracle })
  }

  /// Tests whether the entry's collateral or levercoin is `mint_a` or
  /// `mint_b`.
  #[must_use]
  pub fn owns_either(&self, mint_a: Pubkey, mint_b: Pubkey) -> bool {
    exo_entry_owns_either(&self.entry, mint_a, mint_b)
  }

  /// Derives the pubkeys of the entry accounts and the oracle.
  pub fn pubkeys(&self) -> impl Iterator<Item = Pubkey> {
    exo_entry_pubkeys(&self.entry)
      .into_iter()
      .chain(self.oracle)
  }
}
