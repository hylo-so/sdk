//! Exo registry decoding and account key derivation.

use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;
use anyhow::{Context, Result};
use hylo_core::error::CoreError;
use hylo_core::idl::router::accounts::ExoRegistry;
use hylo_core::idl::router::types::ExoEntry;
use hylo_core::pyth::{PythFeed, PythOracle};
use hylo_idl::tokens::{
  exo_role, Exo, ExoRole, TokenMint, CBBTC, HYPE, ONYC, PST, WETH, ZEC,
};
use hylo_idl::{pda, with_exo_pairs};

use crate::protocol_state::exo_pair::EXO_PAIR_ACCOUNTS;

/// Pubkeys of one exo pair, in [`ExoPairAccounts`] field order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExoPairPubkeys {
  pub exo_pair: Pubkey,
  pub vault: Pubkey,
  pub levercoin_mint: Pubkey,
  pub collateral_mint: Pubkey,
  pub oracle: Pubkey,
  pub token_program: Pubkey,
}

impl ExoPairPubkeys {
  /// Derives the pair's pubkeys from the collateral's compile time consts.
  #[must_use]
  pub const fn derive<E: Exo + PythOracle>() -> ExoPairPubkeys {
    ExoPairPubkeys {
      exo_pair: pda::exo_pair(E::MINT),
      vault: pda::exo_vault(E::MINT, E::TOKEN_PROGRAM),
      levercoin_mint: pda::exo_levercoin_mint(E::MINT),
      collateral_mint: E::MINT,
      oracle: E::FEED.address,
      token_program: E::TOKEN_PROGRAM,
    }
  }

  /// Account keys in [`ExoPairAccounts`] field order.
  #[must_use]
  pub const fn account_keys(&self) -> [Pubkey; EXO_PAIR_ACCOUNTS] {
    [
      self.exo_pair,
      self.vault,
      self.levercoin_mint,
      self.collateral_mint,
      self.oracle,
    ]
  }
}

macro_rules! exo_mint_dispatch {
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

    impl ExoPairPubkeys {
      /// Pubkeys of the pair backed by a collateral mint known at runtime.
      ///
      /// # Errors
      /// * Mint backs no exo pair
      pub fn by_mint(
        collateral_mint: Pubkey,
      ) -> Result<ExoPairPubkeys, CoreError> {
        match collateral_mint {
          $(<$exo>::MINT => Ok(ExoPairPubkeys::derive::<$exo>()),)+
          _ => Err(CoreError::UnknownExoMint),
        }
      }
    }
  };
}

with_exo_pairs!(exo_mint_dispatch);

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
/// * Collateral mint has no feed
pub fn exo_pubkeys_from_entries(
  entries: &[ExoEntry],
) -> Result<Vec<Pubkey>, CoreError> {
  entries.iter().try_fold(
    Vec::with_capacity(entries.len() * EXO_PAIR_ACCOUNTS),
    |mut keys, entry| {
      let pubkeys = ExoPairPubkeys::by_mint(entry.collateral_mint)?;
      keys.extend(pubkeys.account_keys());
      Ok(keys)
    },
  )
}
