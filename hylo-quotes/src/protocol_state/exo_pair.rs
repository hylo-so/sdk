//! Accounts and state for one exo registry entry.

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{ensure, Context, Result};
use fix::prelude::*;
use hylo_core::exchange_context::ExoExchangeContext;
use hylo_core::fees::controller::LevercoinFees;
use hylo_core::idl::exchange::accounts::ExoPair;
use hylo_core::idl::router::types::ExoEntry;
use hylo_core::pyth::OracleConfig;
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::util::normalize_mint_exp;
use hylo_core::virtual_stablecoin::VirtualStablecoin;
use hylo_idl::pda;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::protocol_state::accounts::decode;
use crate::protocol_state::state::in_stablecoin_oracle_window;

/// Accounts derivable from a registry entry, the [`ExoEntryAccounts`]
/// field count.
pub const EXO_PAIR_ACCOUNTS: usize = 4;

/// Accounts derivable from one registry entry, in fetch order, with the
/// pair decoded.
#[derive(Debug, Clone)]
pub struct ExoEntryAccounts {
  pub exo_pair: ExoPair,
  pub vault: Account,
  pub levercoin_mint: Account,
  pub collateral_mint: Account,
}

/// Entry accounts plus the oracle the pair names.
#[derive(Debug, Clone)]
pub struct ExoPairAccounts {
  pub entry: ExoEntryAccounts,
  pub oracle: Account,
}

/// Everything a route needs from one registered [`ExoPair`].
#[derive(Clone)]
pub struct ExoPairState<C: SolanaClock> {
  pub collateral_mint: Pubkey,
  pub oracle: Pubkey,
  pub context: ExoExchangeContext<C>,
  pub paused: bool,
  pub pool_drawdown: PoolDrawdown,
  pub borrow_rate_harvest_epoch: u64,
  pub supply_floor: UFix64<N6>,
  pub oracle_publish_time: i64,
  pub oracle_interval_secs: u64,
}

/// Checks that the fetched oracle carries the feed id the pair expects.
fn validate_exo_pair_oracle(
  exo_pair: &ExoPair,
  oracle: &PriceUpdateV2,
) -> Result<()> {
  ensure!(
    exo_pair.oracle_feed_id == oracle.price_message.feed_id,
    "Exo pair oracle {} feed id does not match pair for {}",
    exo_pair.oracle,
    exo_pair.collateral_mint,
  );
  Ok(())
}

/// Checks the pair against its registry entry.
fn validate_entry(entry: &ExoEntry, exo_pair: &ExoPair) -> Result<()> {
  ensure!(
    exo_pair.collateral_mint == entry.collateral_mint,
    "Exo pair collateral {} does not match registry entry {}",
    exo_pair.collateral_mint,
    entry.collateral_mint,
  );
  ensure!(
    pda::exo_levercoin_mint(entry.collateral_mint) == entry.levercoin_mint,
    "Registry levercoin mint {} is not the levercoin PDA for {}",
    entry.levercoin_mint,
    entry.collateral_mint,
  );
  Ok(())
}

impl ExoEntryAccounts {
  /// Builds from [`EXO_PAIR_ACCOUNTS`] fetched accounts in field order, or
  /// `None` unless every account is present.
  ///
  /// # Errors
  /// * Pair deserialization
  pub fn from_fetched(
    accounts: &[Option<Account>],
  ) -> Result<Option<ExoEntryAccounts>> {
    match accounts {
      [Some(exo_pair), Some(vault), Some(levercoin_mint), Some(collateral_mint)] => {
        Ok(Some(ExoEntryAccounts {
          exo_pair: decode(exo_pair, "Exo pair")?,
          vault: vault.clone(),
          levercoin_mint: levercoin_mint.clone(),
          collateral_mint: collateral_mint.clone(),
        }))
      }
      _ => Ok(None),
    }
  }

  /// Oracle pubkey the pair names.
  #[must_use]
  pub fn oracle(&self) -> Pubkey {
    self.exo_pair.oracle
  }

  /// Attaches the fetched oracle account.
  #[must_use]
  pub fn with_oracle(self, oracle: Account) -> ExoPairAccounts {
    ExoPairAccounts {
      entry: self,
      oracle,
    }
  }
}

impl ExoPairAccounts {
  /// Decodes the accounts, checks them against the registry entry, and
  /// loads the pair state under `clock`.
  ///
  /// # Errors
  /// * Account deserialization
  /// * Pair collateral differs from the entry
  /// * Entry levercoin mint is not the pair's levercoin PDA
  /// * Oracle feed id differs from the pair's
  /// * Vault balance normalization, context load, or fixed-point conversion
  pub fn pair_state<C: SolanaClock>(
    &self,
    entry: &ExoEntry,
    clock: C,
  ) -> Result<ExoPairState<C>> {
    let exo_pair = &self.entry.exo_pair;
    validate_entry(entry, exo_pair)?;
    let vault: TokenAccount = decode(&self.entry.vault, "Exo vault")?;
    let levercoin_mint: Mint =
      decode(&self.entry.levercoin_mint, "Exo levercoin mint")?;
    let collateral_mint: Mint =
      decode(&self.entry.collateral_mint, "Exo collateral mint")?;
    let oracle: PriceUpdateV2 =
      decode(&self.oracle, "Exo collateral/USD Pyth feed")?;
    validate_exo_pair_oracle(exo_pair, &oracle)?;
    let total_collateral = normalize_mint_exp(&collateral_mint, vault.amount)?;
    let oracle_config = OracleConfig::new(
      exo_pair.oracle_interval_secs,
      exo_pair.oracle_conf_tolerance.try_into()?,
    );
    let virtual_stablecoin: VirtualStablecoin =
      exo_pair.virtual_stablecoin.into();
    let levercoin_fees: LevercoinFees = exo_pair.levercoin_fees.into();
    let context = ExoExchangeContext::load(
      clock,
      total_collateral,
      exo_pair.stablecoin_mint_threshold.try_into()?,
      oracle_config,
      levercoin_fees,
      &oracle,
      virtual_stablecoin,
      Some(&levercoin_mint),
      exo_pair.sell_curve_config.into(),
      exo_pair.buy_curve_config.into(),
      exo_pair.levercoin_market_cap_limit.try_into()?,
    )
    .context("ExoExchangeContext::load")?;
    Ok(ExoPairState {
      collateral_mint: exo_pair.collateral_mint,
      oracle: exo_pair.oracle,
      context,
      paused: exo_pair.paused,
      pool_drawdown: exo_pair.pool_drawdown.into(),
      borrow_rate_harvest_epoch: exo_pair.borrow_rate_harvest_cache.epoch,
      supply_floor: exo_pair.virtual_stablecoin_supply_floor.try_into()?,
      oracle_publish_time: oracle.price_message.publish_time,
      oracle_interval_secs: exo_pair.oracle_interval_secs,
    })
  }
}

impl<C: SolanaClock> ExoPairState<C> {
  /// Tests this pair's collateral feed against the stablecoin oracle window.
  #[must_use]
  pub fn collateral_usd_in_stablecoin_oracle_window(&self) -> bool {
    in_stablecoin_oracle_window(
      self.oracle_publish_time,
      self.oracle_interval_secs,
      self.context.clock.unix_timestamp(),
    )
  }
}
