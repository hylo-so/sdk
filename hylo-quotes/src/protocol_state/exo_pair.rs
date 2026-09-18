//! Accounts and state for one Exo registry entry.

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, ensure, Context, Result};
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
use serde::{Deserialize, Serialize};

use crate::protocol_state::accounts::decode;
use crate::protocol_state::exo_registry::exo_pyth_feed_by_mint;
use crate::protocol_state::state::in_stablecoin_oracle_window;

/// Raw accounts for one registry entry, in fetch order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoPairAccounts {
  pub exo_pair: Account,
  pub vault: Account,
  pub levercoin_mint: Account,
  pub collateral_mint: Account,
  pub oracle: Account,
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
}

/// Decoded accounts for one registry entry, checked against the entry.
#[derive(Clone)]
pub struct ExoAccounts {
  exo_pair: ExoPair,
  vault: TokenAccount,
  levercoin_mint: Mint,
  collateral_mint: Mint,
  oracle: PriceUpdateV2,
}

/// Everything a route needs from one registered [`ExoPair`].
#[derive(Clone)]
pub struct ExoPairState<C: SolanaClock> {
  pub collateral_mint: Pubkey,
  pub collateral_mint_decimals: u8,
  pub context: ExoExchangeContext<C>,
  pub paused: bool,
  pub pool_drawdown: PoolDrawdown,
  pub borrow_rate_harvest_epoch: u64,
  pub supply_floor: UFix64<N6>,
  pub oracle_publish_time: i64,
  pub oracle_interval_secs: u64,
}

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
    "Exo pair oracle {} does not match SDK feed {}",
    exo_pair.oracle,
    feed.address,
  );
  ensure!(
    exo_pair.oracle_feed_id == feed.feed_id,
    "Exo pair feed id does not match SDK feed for {}",
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
    "registry levercoin mint {} is not the levercoin PDA for {}",
    entry.levercoin_mint,
    entry.collateral_mint,
  );
  Ok(())
}

impl ExoAccounts {
  /// Decodes the raw accounts and checks them against the registry entry.
  ///
  /// # Errors
  /// * Account deserialization
  /// * Pair collateral differs from the entry
  /// * Entry levercoin mint is not the pair's levercoin PDA
  /// * Pair oracle differs from the SDK feed
  pub fn parse(entry: &ExoEntry, raw: &ExoPairAccounts) -> Result<ExoAccounts> {
    let exo_pair = decode(&raw.exo_pair, "Exo pair")?;
    validate_entry(entry, &exo_pair)?;
    validate_exo_pair_oracle(&exo_pair)?;
    Ok(ExoAccounts {
      exo_pair,
      vault: decode(&raw.vault, "Exo vault")?,
      levercoin_mint: decode(&raw.levercoin_mint, "Exo levercoin mint")?,
      collateral_mint: decode(&raw.collateral_mint, "Exo collateral mint")?,
      oracle: decode(&raw.oracle, "Exo collateral/USD Pyth feed")?,
    })
  }

  #[must_use]
  pub fn collateral_mint(&self) -> Pubkey {
    self.exo_pair.collateral_mint
  }

  /// Loads the pair state under `clock`.
  ///
  /// # Errors
  /// * Vault balance normalization
  /// * Context load or fixed-point conversion
  pub fn pair_state<C: SolanaClock>(
    &self,
    clock: C,
  ) -> Result<ExoPairState<C>> {
    let total_collateral =
      normalize_mint_exp(&self.collateral_mint, self.vault.amount)?;
    let oracle_config = OracleConfig::new(
      self.exo_pair.oracle_interval_secs,
      self.exo_pair.oracle_conf_tolerance.try_into()?,
    );
    let virtual_stablecoin: VirtualStablecoin =
      self.exo_pair.virtual_stablecoin.into();
    let levercoin_fees: LevercoinFees = self.exo_pair.levercoin_fees.into();
    let context = ExoExchangeContext::load(
      clock,
      total_collateral,
      self.exo_pair.stablecoin_mint_threshold.try_into()?,
      oracle_config,
      levercoin_fees,
      &self.oracle,
      virtual_stablecoin,
      Some(&self.levercoin_mint),
      self.exo_pair.sell_curve_config.into(),
      self.exo_pair.buy_curve_config.into(),
      self.exo_pair.levercoin_market_cap_limit.try_into()?,
    )
    .context("ExoExchangeContext::load")?;
    Ok(ExoPairState {
      collateral_mint: self.exo_pair.collateral_mint,
      collateral_mint_decimals: self.collateral_mint.decimals,
      context,
      paused: self.exo_pair.paused,
      pool_drawdown: self.exo_pair.pool_drawdown.into(),
      borrow_rate_harvest_epoch: self.exo_pair.borrow_rate_harvest_cache.epoch,
      supply_floor: self.exo_pair.virtual_stablecoin_supply_floor.try_into()?,
      oracle_publish_time: self.oracle.price_message.publish_time,
      oracle_interval_secs: self.exo_pair.oracle_interval_secs,
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
