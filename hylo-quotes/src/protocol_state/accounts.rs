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

/// Accounts fetched per exo pair: pair PDA, vault, levercoin mint, feed.
const EXO_ACCOUNTS_PER_PAIR: usize = 4;

/// Protocol accounts preceding the exo window in fetch order.
const HEAD_LEN: usize = 10;

/// Protocol accounts following the exo window in fetch order.
const TAIL_LEN: usize = 7;

/// One past the last exo account in fetch order.
const EXO_END: usize = HEAD_LEN + EXO_ACCOUNTS_PER_PAIR * EXO_PAIR_COUNT;

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

/// Groups the exo window of a fetch response into one [`ExoAccounts`] per
/// roster pair.
///
/// A pair whose accounts are not all present is dropped rather than raised:
/// most roster collaterals have no `ExoPair` on chain yet, and an
/// unregistered one must not fail the fetch for the pairs that do exist.
fn registered_exo_accounts(accounts: &[Option<Account>]) -> Vec<ExoAccounts> {
  ProtocolAccounts::EXO_MINTS
    .iter()
    .zip(
      accounts
        .get(HEAD_LEN..EXO_END)
        .unwrap_or_default()
        .chunks_exact(EXO_ACCOUNTS_PER_PAIR),
    )
    .filter_map(|(collateral_mint, group)| match group {
      [Some(exo_pair), Some(vault), Some(levercoin_mint), Some(collateral_usd_pyth)] => {
        Some(ExoAccounts {
          collateral_mint: *collateral_mint,
          exo_pair: exo_pair.clone(),
          vault: vault.clone(),
          levercoin_mint: levercoin_mint.clone(),
          collateral_usd_pyth: collateral_usd_pyth.clone(),
        })
      }
      _ => None,
    })
    .collect()
}

/// Accounts backing one registered exo pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoAccounts {
  /// Collateral mint identifying the pair
  pub collateral_mint: Pubkey,

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

  /// Accounts of every roster exo pair registered on chain.
  ///
  /// Ordered by [`ProtocolAccounts::EXO_MINTS`]; a roster collateral with no
  /// `ExoPair` on chain is simply absent.
  pub exo_pairs: Vec<ExoAccounts>,

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

/// Generates the collateral mint index and the roster-wide pubkey list.
///
/// Every roster pair contributes its four accounts to the frontier fetch,
/// whether or not it is registered on chain, so a newly registered
/// collateral becomes quotable without an SDK change. Callers wanting a
/// single pair use [`ProtocolAccounts::exo_pubkeys`] instead.
macro_rules! exo_roster {
  ($(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?) => {
    /// Number of exo pairs in the roster.
    const EXO_PAIR_COUNT: usize = [$(stringify!($exo)),+].len();

    impl ProtocolAccounts {
      /// Collateral mint of each roster exo pair, in fetch order.
      pub const EXO_MINTS: [Pubkey; EXO_PAIR_COUNT] = [$(<$exo>::MINT),+];

      /// Protocol account pubkeys in RPC fetch order.
      ///
      /// This order matches the struct field order. The exo window runs from
      /// index 10 through the end of the last roster pair, four accounts per
      /// pair in [`ProtocolAccounts::EXO_MINTS`] order.
      pub const PUBKEYS: [Pubkey;
        HEAD_LEN + EXO_ACCOUNTS_PER_PAIR * EXO_PAIR_COUNT + TAIL_LEN] = [
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
        $(
          pda::exo_pair(<$exo>::MINT),
          pda::exo_vault(<$exo>::MINT),
          pda::exo_levercoin_mint(<$exo>::MINT),
          <$exo>::FEED.address,
        )+
        pda::USDC_PAIR,
        pda::USDC_USD_PYTH_FEED,
        JITOSOL::POOL_STATE,
        HYLOSOL::POOL_STATE,
        pda::lst_vault(JITOSOL::MINT),
        pda::lst_vault(HYLOSOL::MINT),
        pda::usdc_vault(USDC::MINT),
      ];
    }
  };
}

with_exo_pairs!(exo_roster);

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
  /// Exo pairs are the one tolerant part of the response: a pair missing any
  /// of its four accounts is dropped from
  /// [`ProtocolAccounts::exo_pairs`] rather than failing the build.
  ///
  /// # Errors
  /// * Account count differs from [`ProtocolAccounts::PUBKEYS`] length
  /// * Any non-exo account is missing
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
      exo_pairs: registered_exo_accounts(accounts),
      usdc_pair: fetched_account(accounts, EXO_END, "UsdcPair")?,
      usdc_usd_pyth: fetched_account(
        accounts,
        EXO_END + 1,
        "USDC/USD Pyth feed",
      )?,
      jitosol_pool_state: fetched_account(
        accounts,
        EXO_END + 2,
        "JitoSOL pool state",
      )?,
      hylosol_pool_state: fetched_account(
        accounts,
        EXO_END + 3,
        "hyloSOL pool state",
      )?,
      jitosol_vault: fetched_account(accounts, EXO_END + 4, "JitoSOL vault")?,
      hylosol_vault: fetched_account(accounts, EXO_END + 5, "hyloSOL vault")?,
      usdc_vault: fetched_account(accounts, EXO_END + 6, "USDC vault")?,
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

  /// A fetch response in which every requested account exists.
  fn all_present() -> Vec<Option<Account>> {
    vec![Some(Account::default()); ProtocolAccounts::PUBKEYS.len()]
  }

  /// Blanks the accounts of the roster pair at `index`.
  fn drop_pair(fetched: &mut [Option<Account>], index: usize) {
    let base = HEAD_LEN + EXO_ACCOUNTS_PER_PAIR * index;
    (base..base + EXO_ACCOUNTS_PER_PAIR).for_each(|i| fetched[i] = None);
  }

  fn collateral_mints(accounts: &ProtocolAccounts) -> Vec<Pubkey> {
    accounts
      .exo_pairs
      .iter()
      .map(|exo| exo.collateral_mint)
      .collect()
  }

  #[test]
  fn pubkeys_hold_one_window_per_roster_pair() {
    assert_eq!(ProtocolAccounts::EXO_MINTS.len(), EXO_PAIR_COUNT);
    assert_eq!(
      ProtocolAccounts::PUBKEYS.len(),
      HEAD_LEN + EXO_ACCOUNTS_PER_PAIR * EXO_PAIR_COUNT + TAIL_LEN
    );
    ProtocolAccounts::EXO_MINTS
      .iter()
      .enumerate()
      .for_each(|(i, mint)| {
        let base = HEAD_LEN + EXO_ACCOUNTS_PER_PAIR * i;
        assert_eq!(ProtocolAccounts::PUBKEYS[base], pda::exo_pair(*mint));
        assert_eq!(ProtocolAccounts::PUBKEYS[base + 1], pda::exo_vault(*mint));
        assert_eq!(
          ProtocolAccounts::PUBKEYS[base + 2],
          pda::exo_levercoin_mint(*mint)
        );
      });
  }

  #[test]
  fn cbbtc_window_keeps_its_historical_offset() {
    assert_eq!(ProtocolAccounts::EXO_MINTS[0], CBBTC::MINT);
    assert_eq!(ProtocolAccounts::PUBKEYS[10], pda::exo_pair(CBBTC::MINT));
    assert_eq!(ProtocolAccounts::PUBKEYS[13], CBBTC::FEED.address);
  }

  #[test]
  fn every_registered_pair_is_collected() -> Result<()> {
    let accounts = ProtocolAccounts::from_fetched(&all_present())?;
    assert_eq!(collateral_mints(&accounts), ProtocolAccounts::EXO_MINTS);
    Ok(())
  }

  #[test]
  fn unregistered_pair_does_not_fail_the_fetch() -> Result<()> {
    let mut fetched = all_present();
    drop_pair(&mut fetched, 1);
    let accounts = ProtocolAccounts::from_fetched(&fetched)?;
    let mints = collateral_mints(&accounts);
    assert_eq!(mints.len(), EXO_PAIR_COUNT - 1);
    assert!(!mints.contains(&ProtocolAccounts::EXO_MINTS[1]));
    assert!(mints.contains(&ProtocolAccounts::EXO_MINTS[0]));
    Ok(())
  }

  #[test]
  fn partially_present_pair_is_dropped_whole() -> Result<()> {
    let mut fetched = all_present();
    fetched[HEAD_LEN + 1] = None;
    let accounts = ProtocolAccounts::from_fetched(&fetched)?;
    let mints = collateral_mints(&accounts);
    assert_eq!(mints.len(), EXO_PAIR_COUNT - 1);
    assert!(!mints.contains(&ProtocolAccounts::EXO_MINTS[0]));
    Ok(())
  }

  #[test]
  fn every_pair_unregistered_still_yields_accounts() -> Result<()> {
    let mut fetched = all_present();
    (0..EXO_PAIR_COUNT).for_each(|i| drop_pair(&mut fetched, i));
    let accounts = ProtocolAccounts::from_fetched(&fetched)?;
    assert!(accounts.exo_pairs.is_empty());
    Ok(())
  }

  #[test]
  fn missing_non_exo_account_still_fails() {
    let mut fetched = all_present();
    fetched[EXO_END] = None;
    assert!(ProtocolAccounts::from_fetched(&fetched).is_err());
  }
}
