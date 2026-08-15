//! Type-safe collection of protocol state accounts

use std::convert::TryFrom;

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar;
use anyhow::{anyhow, ensure, Context, Result};
use hylo_core::error::CoreError;
use hylo_core::pyth::PythOracle;
use hylo_idl::tokens::{
  Exo, StakePool, TokenMint, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, ONYC, PST,
  SHYUSD, USDC, WETH, XSOL, ZEC,
};
use hylo_idl::{pda, with_exo_pairs};
use serde::{Deserialize, Serialize};

/// Protocol accounts fetched ahead of the appended exo pair windows.
///
/// Indices `0..21` are the fetch order shipped in 2.2.2 and never move.
const BASE_ACCOUNTS: usize = 21;

/// Accounts fetched per exo pair: pair PDA, vault, levercoin mint, feed.
const EXO_WINDOW_LEN: usize = 4;

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

/// Position of `pubkey` in [`ProtocolAccounts::PUBKEYS`].
fn pubkey_index(pubkey: &Pubkey) -> Option<usize> {
  ProtocolAccounts::PUBKEYS
    .iter()
    .position(|key| key == pubkey)
}

/// Reads the four-account window of exo collateral `E` from a fetch response.
///
/// Yields `None` unless the whole window came back, which is the ordinary
/// case: most roster collaterals have no `ExoPair` registered on chain, and
/// an unregistered one must never fail the fetch for the pairs that do
/// exist.
fn exo_pair_accounts<E: Exo + PythOracle>(
  accounts: &[Option<Account>],
) -> Option<ExoPairAccounts> {
  let base = pubkey_index(&pda::exo_pair(E::MINT))?;
  match accounts.get(base..base + EXO_WINDOW_LEN)? {
    [Some(exo_pair), Some(vault), Some(levercoin_mint), Some(collateral_usd_pyth)] => {
      Some(ExoPairAccounts {
        exo_pair: exo_pair.clone(),
        vault: vault.clone(),
        levercoin_mint: levercoin_mint.clone(),
        collateral_usd_pyth: collateral_usd_pyth.clone(),
      })
    }
    _ => None,
  }
}

/// The four accounts backing one exo pair.
///
/// Groups what the cbBTC fields of [`ProtocolAccounts`] hold flat, so a
/// collateral whose pair is unregistered is absent as a whole rather than
/// half-loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoPairAccounts {
  /// `ExoPair` PDA
  pub exo_pair: Account,

  /// Collateral vault token account
  pub vault: Account,

  /// Levercoin mint
  pub levercoin_mint: Account,

  /// Pyth collateral/USD price feed
  pub collateral_usd_pyth: Account,
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

  /// cbBTC `ExoPair` PDA
  pub cbbtc_exo_pair: Account,

  /// cbBTC collateral vault token account
  pub cbbtc_vault: Account,

  /// xBTC levercoin mint
  pub xbtc_mint: Account,

  /// Pyth BTC/USD price feed
  pub btc_usd_pyth: Account,

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

  /// HYPE exo pair accounts, absent unless the pair is registered on chain
  #[serde(default)]
  pub hype_pair_accounts: Option<ExoPairAccounts>,

  /// ONYC exo pair accounts, absent unless the pair is registered on chain
  #[serde(default)]
  pub onyc_pair_accounts: Option<ExoPairAccounts>,

  /// PST exo pair accounts, absent unless the pair is registered on chain
  #[serde(default)]
  pub pst_pair_accounts: Option<ExoPairAccounts>,

  /// WETH exo pair accounts, absent unless the pair is registered on chain
  #[serde(default)]
  pub weth_pair_accounts: Option<ExoPairAccounts>,

  /// ZEC exo pair accounts, absent unless the pair is registered on chain
  #[serde(default)]
  pub zec_pair_accounts: Option<ExoPairAccounts>,
}

/// Builds [`ProtocolAccounts::PUBKEYS`] from the exo pair roster.
///
/// cbBTC keeps the window it has always held at indices `10..14`, so every
/// index of the 2.2.2 array is preserved; the remaining roster pairs append
/// their windows after the base accounts, in roster order. Requesting a
/// window whose `ExoPair` is not registered on chain is harmless: the
/// accounts come back empty and [`ProtocolAccounts::from_fetched`] leaves
/// that pair unset.
macro_rules! protocol_pubkeys {
  (
    (CBBTC, $cbbtc_lever:ident, $cbbtc_exp:ty),
    $(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?
  ) => {
    /// Roster exo pairs whose windows append after the base accounts.
    const APPENDED_PAIRS: usize = [$(<$exo>::MINT),+].len();

    impl ProtocolAccounts {
      /// Protocol account pubkeys in RPC fetch order.
      ///
      /// The first [`BASE_ACCOUNTS`] entries match the struct field order
      /// through `usdc_vault` and keep the indices they had in 2.2.2. Each
      /// remaining roster pair then appends a four-account window — pair
      /// PDA, vault, levercoin mint, collateral/USD feed — in the order the
      /// `with_exo_pairs!` roster lists them.
      pub const PUBKEYS: [Pubkey;
        BASE_ACCOUNTS + EXO_WINDOW_LEN * APPENDED_PAIRS] = [
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
        pda::exo_pair(CBBTC::MINT),
        pda::exo_vault(CBBTC::MINT),
        pda::exo_levercoin_mint(CBBTC::MINT),
        CBBTC::FEED.address,
        pda::USDC_PAIR,
        pda::USDC_USD_PYTH_FEED,
        JITOSOL::POOL_STATE,
        HYLOSOL::POOL_STATE,
        pda::lst_vault(JITOSOL::MINT),
        pda::lst_vault(HYLOSOL::MINT),
        pda::usdc_vault(USDC::MINT),
        $(
          pda::exo_pair(<$exo>::MINT),
          pda::exo_vault(<$exo>::MINT),
          pda::exo_levercoin_mint(<$exo>::MINT),
          <$exo>::FEED.address,
        )+
      ];
    }
  };
  ($($roster:tt)*) => {
    compile_error!(
      "cbBTC must stay first in `with_exo_pairs!`: `ProtocolAccounts::PUBKEYS` \
       pins its window to indices 10..14"
    );
  };
}

with_exo_pairs!(protocol_pubkeys);

impl ProtocolAccounts {
  /// Get the list of account pubkeys in the order expected by RPC
  #[deprecated(since = "2.1.0", note = "use `ProtocolAccounts::PUBKEYS`")]
  #[must_use]
  pub fn pubkeys() -> Vec<Pubkey> {
    ProtocolAccounts::PUBKEYS.to_vec()
  }

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
  /// Order: exo pair, vault, levercoin mint, collateral/USD feed, clock.
  #[must_use]
  pub fn exo_pubkeys<E: Exo + PythOracle>() -> [Pubkey; 5] {
    [
      pda::exo_pair(E::MINT),
      pda::exo_vault(E::MINT),
      pda::exo_levercoin_mint(E::MINT),
      E::FEED.address,
      sysvar::clock::ID,
    ]
  }

  /// Expected number of protocol accounts
  #[deprecated(since = "2.1.0", note = "use `ProtocolAccounts::PUBKEYS.len()`")]
  #[must_use]
  pub const fn expected_count() -> usize {
    ProtocolAccounts::PUBKEYS.len()
  }

  /// Build from RPC-fetched accounts in [`ProtocolAccounts::PUBKEYS`] order.
  ///
  /// The appended exo pair windows are the tolerant part of the response: a
  /// pair missing any of its four accounts is left unset instead of failing
  /// the build, so an unregistered collateral costs only its own routes.
  ///
  /// # Errors
  /// * Account count differs from [`ProtocolAccounts::PUBKEYS`] length
  /// * Any base account is missing
  pub fn from_fetched(
    accounts: &[Option<Account>],
  ) -> Result<ProtocolAccounts> {
    ensure!(
      accounts.len() == ProtocolAccounts::PUBKEYS.len(),
      "Expected {} accounts, got {}",
      ProtocolAccounts::PUBKEYS.len(),
      accounts.len()
    );
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
      cbbtc_exo_pair: fetched_account(accounts, 10, "cbBTC ExoPair")?,
      cbbtc_vault: fetched_account(accounts, 11, "cbBTC vault")?,
      xbtc_mint: fetched_account(accounts, 12, "xBTC mint")?,
      btc_usd_pyth: fetched_account(accounts, 13, "BTC/USD Pyth feed")?,
      usdc_pair: fetched_account(accounts, 14, "UsdcPair")?,
      usdc_usd_pyth: fetched_account(accounts, 15, "USDC/USD Pyth feed")?,
      jitosol_pool_state: fetched_account(accounts, 16, "JitoSOL pool state")?,
      hylosol_pool_state: fetched_account(accounts, 17, "hyloSOL pool state")?,
      jitosol_vault: fetched_account(accounts, 18, "JitoSOL vault")?,
      hylosol_vault: fetched_account(accounts, 19, "hyloSOL vault")?,
      usdc_vault: fetched_account(accounts, 20, "USDC vault")?,
      hype_pair_accounts: exo_pair_accounts::<HYPE>(accounts),
      onyc_pair_accounts: exo_pair_accounts::<ONYC>(accounts),
      pst_pair_accounts: exo_pair_accounts::<PST>(accounts),
      weth_pair_accounts: exo_pair_accounts::<WETH>(accounts),
      zec_pair_accounts: exo_pair_accounts::<ZEC>(accounts),
    })
  }

  /// Validate that pubkeys and accounts match expected protocol accounts
  ///
  /// Validates:
  /// * Pubkeys and accounts have matching lengths
  /// * We have the expected number of accounts
  /// * Each pubkey matches the expected protocol account in order
  ///
  /// # Errors
  /// Returns error if any validation fails
  pub fn validate(
    pubkeys: &[Pubkey],
    accounts: &[Option<Account>],
  ) -> Result<()> {
    ensure!(
      pubkeys.len() == accounts.len(),
      "Mismatch: {} pubkeys but {} accounts",
      pubkeys.len(),
      accounts.len()
    );

    let expected_count = ProtocolAccounts::PUBKEYS.len();
    ensure!(
      pubkeys.len() == expected_count,
      "Expected {} accounts, got {}",
      expected_count,
      pubkeys.len()
    );

    // Validate pubkeys match expected
    let expected = ProtocolAccounts::PUBKEYS;
    expected.iter().zip(pubkeys.iter()).enumerate().try_fold(
      (),
      |(), (i, (expected_pubkey, actual_pubkey))| {
        if expected_pubkey == actual_pubkey {
          Ok(())
        } else {
          Err(anyhow!(
            "Account {i} mismatch: expected {expected_pubkey}, got \
             {actual_pubkey}"
          ))
        }
      },
    )
  }
}

/// Deprecated: use [`ProtocolAccounts::from_fetched`]. Removed in 3.0.
impl TryFrom<(&[Pubkey], &[Option<Account>])> for ProtocolAccounts {
  type Error = anyhow::Error;

  fn try_from(
    (pubkeys, accounts): (&[Pubkey], &[Option<Account>]),
  ) -> Result<ProtocolAccounts> {
    ProtocolAccounts::validate(pubkeys, accounts)?;
    ProtocolAccounts::from_fetched(accounts)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// The fetch order published in 2.2.2, which no release may renumber.
  const BASE_PUBKEYS: [Pubkey; BASE_ACCOUNTS] = [
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
    pda::exo_pair(CBBTC::MINT),
    pda::exo_vault(CBBTC::MINT),
    pda::exo_levercoin_mint(CBBTC::MINT),
    CBBTC::FEED.address,
    pda::USDC_PAIR,
    pda::USDC_USD_PYTH_FEED,
    JITOSOL::POOL_STATE,
    HYLOSOL::POOL_STATE,
    pda::lst_vault(JITOSOL::MINT),
    pda::lst_vault(HYLOSOL::MINT),
    pda::usdc_vault(USDC::MINT),
  ];

  /// Collateral mint and Pyth feed of each appended window, in fetch order.
  const APPENDED_WINDOWS: [(Pubkey, Pubkey); APPENDED_PAIRS] = [
    (HYPE::MINT, HYPE::FEED.address),
    (ONYC::MINT, ONYC::FEED.address),
    (PST::MINT, PST::FEED.address),
    (WETH::MINT, WETH::FEED.address),
    (ZEC::MINT, ZEC::FEED.address),
  ];

  /// A fetch response in which every requested account exists.
  fn all_present() -> Vec<Option<Account>> {
    vec![Some(Account::default()); ProtocolAccounts::PUBKEYS.len()]
  }

  /// The mainnet-shaped response: cbBTC is the only registered exo pair, so
  /// every appended window comes back empty.
  fn base_only() -> Vec<Option<Account>> {
    (0..ProtocolAccounts::PUBKEYS.len())
      .map(|i| (i < BASE_ACCOUNTS).then(Account::default))
      .collect()
  }

  /// First index of the window fetched for `collateral_mint`.
  fn window_base(collateral_mint: Pubkey) -> Result<usize> {
    pubkey_index(&pda::exo_pair(collateral_mint))
      .context("collateral has no window in PUBKEYS")
  }

  #[test]
  fn base_pubkeys_keep_their_indices() {
    assert_eq!(ProtocolAccounts::PUBKEYS[..BASE_ACCOUNTS], BASE_PUBKEYS);
    assert_eq!(
      ProtocolAccounts::PUBKEYS.len(),
      BASE_ACCOUNTS + EXO_WINDOW_LEN * APPENDED_PAIRS
    );
  }

  #[test]
  fn cbbtc_window_stays_within_the_base() -> Result<()> {
    assert_eq!(window_base(CBBTC::MINT)?, 10);
    Ok(())
  }

  #[test]
  fn roster_windows_append_in_order() {
    APPENDED_WINDOWS.iter().enumerate().for_each(
      |(i, (collateral_mint, feed))| {
        let base = BASE_ACCOUNTS + EXO_WINDOW_LEN * i;
        assert_eq!(
          ProtocolAccounts::PUBKEYS[base],
          pda::exo_pair(*collateral_mint)
        );
        assert_eq!(
          ProtocolAccounts::PUBKEYS[base + 1],
          pda::exo_vault(*collateral_mint)
        );
        assert_eq!(
          ProtocolAccounts::PUBKEYS[base + 2],
          pda::exo_levercoin_mint(*collateral_mint)
        );
        assert_eq!(ProtocolAccounts::PUBKEYS[base + 3], *feed);
      },
    );
  }

  #[test]
  fn appended_window_matches_the_isolated_fetch() -> Result<()> {
    let base = window_base(HYPE::MINT)?;
    let isolated = ProtocolAccounts::exo_pubkeys::<HYPE>();
    assert_eq!(
      ProtocolAccounts::PUBKEYS[base..base + EXO_WINDOW_LEN],
      isolated[..EXO_WINDOW_LEN]
    );
    Ok(())
  }

  #[test]
  fn every_registered_window_is_read() -> Result<()> {
    let accounts = ProtocolAccounts::from_fetched(&all_present())?;
    assert!(accounts.hype_pair_accounts.is_some());
    assert!(accounts.onyc_pair_accounts.is_some());
    assert!(accounts.pst_pair_accounts.is_some());
    assert!(accounts.weth_pair_accounts.is_some());
    assert!(accounts.zec_pair_accounts.is_some());
    Ok(())
  }

  #[test]
  fn unregistered_pairs_do_not_fail_the_fetch() -> Result<()> {
    let accounts = ProtocolAccounts::from_fetched(&base_only())?;
    assert!(accounts.hype_pair_accounts.is_none());
    assert!(accounts.onyc_pair_accounts.is_none());
    assert!(accounts.pst_pair_accounts.is_none());
    assert!(accounts.weth_pair_accounts.is_none());
    assert!(accounts.zec_pair_accounts.is_none());
    Ok(())
  }

  #[test]
  fn partial_window_is_dropped_whole() -> Result<()> {
    let mut fetched = all_present();
    fetched[window_base(ZEC::MINT)? + 1] = None;
    let accounts = ProtocolAccounts::from_fetched(&fetched)?;
    assert!(accounts.zec_pair_accounts.is_none());
    assert!(accounts.weth_pair_accounts.is_some());
    Ok(())
  }

  #[test]
  fn missing_base_account_still_fails() {
    let mut fetched = all_present();
    fetched[BASE_ACCOUNTS - 1] = None;
    assert!(ProtocolAccounts::from_fetched(&fetched).is_err());
  }
}
