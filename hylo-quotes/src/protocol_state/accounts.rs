//! Type-safe collection of protocol state accounts

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar;
use anchor_lang::AccountDeserialize;
use anyhow::{ensure, Context, Result};
use hylo_core::error::CoreError;
use hylo_core::pyth::PythOracle;
use hylo_idl::pda;
use hylo_idl::tokens::{
  Exo, StakePool, TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XSOL,
};
use serde::{Deserialize, Serialize};

use crate::protocol_state::ExoPairAccounts;

/// Deserializes an Anchor account, named `name` in errors.
pub(crate) fn decode<A: AccountDeserialize>(
  account: &Account,
  name: &str,
) -> Result<A> {
  A::try_deserialize(&mut account.data.as_slice())
    .with_context(|| format!("{name} deserialization"))
}

/// Extracts the fetched account at `index`, named `name` in errors.
///
/// # Errors
/// * [`CoreError::ProtocolAccountNotFound`] if absent or out of range
fn fetched_account(
  accounts: &[Option<Account>],
  index: usize,
  name: &str,
) -> Result<Account> {
  accounts
    .get(index)
    .and_then(Option::as_ref)
    .cloned()
    .ok_or(CoreError::ProtocolAccountNotFound)
    .with_context(|| format!("{name} not found"))
}

/// Type-safe collection of protocol state accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
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

  /// Earn pool configuration
  pub pool_config: Account,

  /// HYUSD earn pool token account
  pub hyusd_pool: Account,

  /// Pyth SOL/USD price feed
  pub sol_usd_pyth: Account,

  /// Solana clock sysvar
  pub clock: Account,

  /// Canonical router-owned Exo registry
  pub exo_registry: Account,

  /// Raw accounts per registry entry
  pub exo_accounts: Vec<ExoPairAccounts>,

  /// `UsdcPair` PDA
  pub usdc_pair: Account,

  /// Pyth USDC/USD price feed
  pub usdc_usd_pyth: Account,

  /// `JitoSOL` SPL stake pool state
  pub jitosol_pool_state: Account,

  /// `hyloSOL` SPL stake pool state
  pub hylosol_pool_state: Account,

  /// `JitoSOL` collateral vault token account
  pub jitosol_vault: Account,

  /// `hyloSOL` collateral vault token account
  pub hylosol_vault: Account,

  /// USDC collateral vault token account
  pub usdc_vault: Account,
}

impl ProtocolAccounts {
  /// Protocol account pubkeys in RPC fetch order.
  ///
  /// This order matches the struct field order.
  pub const PUBKEYS: [Pubkey; 18] = [
    pda::HYLO,
    pda::lst_header(JITOSOL::MINT),
    pda::lst_header(HYLOSOL::MINT),
    HYUSD::MINT,
    SHYUSD::MINT,
    XSOL::MINT,
    pda::POOL_CONFIG,
    pda::HYUSD_POOL,
    hylo_core::pyth::SOL_USD.address,
    sysvar::clock::ID,
    pda::USDC_PAIR,
    pda::USDC_USD_PYTH_FEED,
    JITOSOL::POOL_STATE,
    HYLOSOL::POOL_STATE,
    pda::lst_vault(JITOSOL::MINT),
    pda::lst_vault(HYLOSOL::MINT),
    pda::usdc_vault(USDC::MINT),
    pda::EXO_REGISTRY,
  ];

  /// Pubkey subset for the isolated LST exchange context.
  ///
  /// Order: Hylo, xSOL mint, SOL/USD feed, clock.
  #[must_use]
  pub const fn lst_pubkeys() -> [Pubkey; 4] {
    [
      pda::HYLO,
      XSOL::MINT,
      hylo_core::pyth::SOL_USD.address,
      sysvar::clock::ID,
    ]
  }

  /// Pubkey subset for one isolated exo pair.
  ///
  /// Order: exo pair, vault, levercoin mint, collateral mint, collateral/USD
  /// feed, clock.
  #[must_use]
  pub fn exo_pubkeys<E: Exo + PythOracle>() -> [Pubkey; 6] {
    [
      pda::exo_pair(E::MINT),
      pda::exo_vault(E::MINT),
      pda::exo_levercoin_mint(E::MINT),
      E::MINT,
      E::FEED.address,
      sysvar::clock::ID,
    ]
  }

  /// Build from RPC-fetched accounts in [`ProtocolAccounts::PUBKEYS`] order,
  /// followed by five accounts per registry entry.
  ///
  /// # Errors
  /// * Fewer accounts than [`ProtocolAccounts::PUBKEYS`]
  /// * Any account is missing
  pub fn from_fetched(
    accounts: &[Option<Account>],
  ) -> Result<ProtocolAccounts> {
    const BASE: usize = ProtocolAccounts::PUBKEYS.len();
    ensure!(
      accounts.len() >= BASE,
      "Expected at least {BASE} accounts, got {}",
      accounts.len()
    );
    let exo_registry = fetched_account(accounts, BASE - 1, "Exo registry")?;
    let exo_accounts = accounts[BASE..]
      .chunks(5)
      .map(ExoPairAccounts::from_fetched)
      .collect::<Result<Vec<_>>>()?;
    Ok(ProtocolAccounts {
      hylo: fetched_account(accounts, 0, "Hylo account")?,
      jitosol_header: fetched_account(accounts, 1, "JitoSOL header")?,
      hylosol_header: fetched_account(accounts, 2, "HyloSOL header")?,
      hyusd_mint: fetched_account(accounts, 3, "HYUSD mint")?,
      shyusd_mint: fetched_account(accounts, 4, "SHYUSD mint")?,
      xsol_mint: fetched_account(accounts, 5, "XSOL mint")?,
      pool_config: fetched_account(accounts, 6, "Pool config")?,
      hyusd_pool: fetched_account(accounts, 7, "HYUSD pool")?,
      sol_usd_pyth: fetched_account(accounts, 8, "SOL/USD Pyth feed")?,
      clock: fetched_account(accounts, 9, "Clock sysvar")?,
      exo_registry,
      exo_accounts,
      usdc_pair: fetched_account(accounts, 10, "UsdcPair")?,
      usdc_usd_pyth: fetched_account(accounts, 11, "USDC/USD Pyth feed")?,
      jitosol_pool_state: fetched_account(accounts, 12, "JitoSOL pool state")?,
      hylosol_pool_state: fetched_account(accounts, 13, "hyloSOL pool state")?,
      jitosol_vault: fetched_account(accounts, 14, "JitoSOL vault")?,
      hylosol_vault: fetched_account(accounts, 15, "hyloSOL vault")?,
      usdc_vault: fetched_account(accounts, 16, "USDC vault")?,
    })
  }
}
