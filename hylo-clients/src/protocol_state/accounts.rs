//! Type-safe collection of protocol state accounts

use anchor_client::solana_sdk::account::Account;
use anchor_lang::{prelude::Pubkey, solana_program::sysvar};
use anyhow::{anyhow, Context, Result};
use hylo_idl::{
  pda,
  tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL},
};
use std::convert::TryFrom;

/// Type-safe collection of protocol state accounts
#[derive(Debug, Clone)]
pub struct ProtocolAccounts {
  /// Hylo protocol account
  pub hylo: Account,

  /// `JitoSOL` LST header
  pub jitosol_header: Account,

  /// `HyloSOL` LST header
  pub hylosol_header: Account,

  /// HYUSD mint account
  pub hyusd_mint: Account,

  /// SHYUSD mint account
  pub shyusd_mint: Account,

  /// XSOL mint account
  pub xsol_mint: Account,

  /// Stability pool configuration
  pub pool_config: Account,

  /// HYUSD stability pool token account
  pub hyusd_pool: Account,

  /// XSOL stability pool token account
  pub xsol_pool: Account,

  /// Pyth SOL/USD price feed
  pub sol_usd_pyth: Account,

  /// Solana clock sysvar
  pub clock: Account,
}

impl ProtocolAccounts {
  /// Get the list of account pubkeys in the order expected by RPC
  ///
  /// This order matches the struct field order for `TryFrom` conversion.
  #[must_use]
  pub fn pubkeys() -> Vec<Pubkey> {
    vec![
      *pda::HYLO,
      pda::lst_header(JITOSOL::MINT),
      pda::lst_header(HYLOSOL::MINT),
      HYUSD::MINT,
      SHYUSD::MINT,
      XSOL::MINT,
      *pda::POOL_CONFIG,
      *pda::HYUSD_POOL,
      *pda::XSOL_POOL,
      hylo_core::pyth::SOL_USD_PYTH_FEED,
      sysvar::clock::ID,
    ]
  }
}

/// Convert from RPC response (pubkeys and accounts) to `ProtocolAccounts`
///
/// Validates that:
/// - The number of pubkeys matches the number of accounts
/// - The pubkeys match the expected protocol accounts in order
/// - All accounts are present (not None)
impl TryFrom<(&[Pubkey], &[Option<Account>])> for ProtocolAccounts {
  type Error = anyhow::Error;

  fn try_from(
    (pubkeys, accounts): (&[Pubkey], &[Option<Account>]),
  ) -> Result<Self> {
    const EXPECTED_COUNT: usize = 11;

    // Validate length
    if pubkeys.len() != accounts.len() {
      return Err(anyhow!(
        "Mismatch: {} pubkeys but {} accounts",
        pubkeys.len(),
        accounts.len()
      ));
    }

    if pubkeys.len() != EXPECTED_COUNT {
      return Err(anyhow!(
        "Expected {} accounts, got {}",
        EXPECTED_COUNT,
        pubkeys.len()
      ));
    }

    // Validate pubkeys match expected
    let expected = Self::pubkeys();
    expected
      .iter()
      .zip(pubkeys.iter())
      .enumerate()
      .try_fold((), |(), (i, (expected_pubkey, actual_pubkey))| {
        if expected_pubkey == actual_pubkey {
          Ok(())
        } else {
          Err(anyhow!(
            "Account {i} mismatch: expected {expected_pubkey}, got {actual_pubkey}"
          ))
        }
      })?;

    // Extract accounts with proper error messages
    Ok(Self {
      hylo: accounts[0]
        .as_ref()
        .context("Hylo account not found")?
        .clone(),

      jitosol_header: accounts[1]
        .as_ref()
        .context("JitoSOL header not found")?
        .clone(),

      hylosol_header: accounts[2]
        .as_ref()
        .context("HyloSOL header not found")?
        .clone(),

      hyusd_mint: accounts[3]
        .as_ref()
        .context("HYUSD mint not found")?
        .clone(),

      shyusd_mint: accounts[4]
        .as_ref()
        .context("SHYUSD mint not found")?
        .clone(),

      xsol_mint: accounts[5].as_ref().context("XSOL mint not found")?.clone(),

      pool_config: accounts[6]
        .as_ref()
        .context("Pool config not found")?
        .clone(),

      hyusd_pool: accounts[7]
        .as_ref()
        .context("HYUSD pool not found")?
        .clone(),

      xsol_pool: accounts[8].as_ref().context("XSOL pool not found")?.clone(),

      sol_usd_pyth: accounts[9]
        .as_ref()
        .context("SOL/USD Pyth feed not found")?
        .clone(),

      clock: accounts[10]
        .as_ref()
        .context("Clock sysvar not found")?
        .clone(),
    })
  }
}
