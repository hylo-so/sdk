//! Accounts and state for one exo registry entry.

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, ensure, Context, Result};
use fix::prelude::*;
use hylo_core::exchange_context::ExoExchangeContext;
use hylo_core::fees::controller::LevercoinFees;
use hylo_core::idl::exchange::accounts::ExoPair;
use hylo_core::idl::oracle::accounts::PriceAggregate;
use hylo_core::idl::router::types::ExoEntry;
use hylo_core::oracle::{OracleConfig, PriceAggregateObservations};
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::util::normalize_mint_exp;
use hylo_core::virtual_stablecoin::VirtualStablecoin;
use hylo_idl::pda;
use serde::{Deserialize, Serialize};

use crate::protocol_state::accounts::decode;
use crate::protocol_state::price_aggregate::latest_timestamp_secs;
use crate::protocol_state::state::in_stablecoin_oracle_window;

/// Accounts fetched per registry entry, the [`ExoPairAccounts`] field count.
pub const EXO_PAIR_ACCOUNTS: usize = 5;

/// Raw accounts for one registry entry, in fetch order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoPairAccounts {
  pub exo_pair: Account,
  pub vault: Account,
  pub levercoin_mint: Account,
  pub collateral_mint: Account,
  pub oracle: Account,
}

/// Everything a route needs from one registered [`ExoPair`].
#[derive(Clone)]
pub struct ExoPairState<C: SolanaClock> {
  pub collateral_mint: Pubkey,
  pub context: ExoExchangeContext<C>,
  pub paused: bool,
  pub pool_drawdown: PoolDrawdown,
  pub borrow_rate_harvest_epoch: u64,
  pub supply_floor: UFix64<N6>,
  pub oracle_publish_time: i64,
  pub oracle_interval_secs: u64,
}

/// Checks that the pair's oracle is the aggregate PDA for its collateral.
fn validate_exo_pair_oracle(exo_pair: &ExoPair) -> Result<()> {
  ensure!(
    exo_pair.oracle == pda::price_aggregate(exo_pair.collateral_mint),
    "Exo pair oracle {} does not match aggregate PDA for {}",
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

impl ExoPairAccounts {
  /// Builds from five fetched accounts in field order.
  ///
  /// # Errors
  /// * Wrong count or any account missing
  pub fn from_fetched(accounts: &[Option<Account>]) -> Result<ExoPairAccounts> {
    match accounts {
      [Some(exo_pair), Some(vault), Some(levercoin_mint), Some(collateral_mint), Some(oracle)] => {
        Ok(ExoPairAccounts {
          exo_pair: exo_pair.clone(),
          vault: vault.clone(),
          levercoin_mint: levercoin_mint.clone(),
          collateral_mint: collateral_mint.clone(),
          oracle: oracle.clone(),
        })
      }
      _ => Err(anyhow!("Exo pair accounts missing")),
    }
  }

  /// Decodes the accounts, checks them against the registry entry, and
  /// loads the pair state under `clock`.
  ///
  /// # Errors
  /// * Account deserialization
  /// * Pair collateral differs from the entry
  /// * Entry levercoin mint is not the pair's levercoin PDA
  /// * Pair oracle differs from the SDK feed
  /// * Vault balance normalization, context load, or fixed-point conversion
  pub fn pair_state<C: SolanaClock>(
    &self,
    entry: &ExoEntry,
    clock: C,
  ) -> Result<ExoPairState<C>> {
    let exo_pair: ExoPair = decode(&self.exo_pair, "Exo pair")?;
    validate_entry(entry, &exo_pair)?;
    validate_exo_pair_oracle(&exo_pair)?;
    let vault: TokenAccount = decode(&self.vault, "Exo vault")?;
    let levercoin_mint: Mint =
      decode(&self.levercoin_mint, "Exo levercoin mint")?;
    let collateral_mint: Mint =
      decode(&self.collateral_mint, "Exo collateral mint")?;
    let oracle_aggregate: PriceAggregate =
      decode(&self.oracle, "Exo collateral/USD aggregate")?;
    let oracle =
      PriceAggregateObservations::from(oracle_aggregate.observations.0);
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
      exo_pair.collateral_mint,
    )
    .context("ExoExchangeContext::load")?;
    Ok(ExoPairState {
      collateral_mint: exo_pair.collateral_mint,
      context,
      paused: exo_pair.paused,
      pool_drawdown: exo_pair.pool_drawdown.into(),
      borrow_rate_harvest_epoch: exo_pair.borrow_rate_harvest_cache.epoch,
      supply_floor: exo_pair.virtual_stablecoin_supply_floor.try_into()?,
      oracle_publish_time: latest_timestamp_secs(&oracle)?,
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
