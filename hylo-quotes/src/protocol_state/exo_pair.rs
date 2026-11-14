//! Accounts and state for one exo registry entry.

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, ensure, Context, Result};
use fix::prelude::*;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::{ExchangeContext, ExoExchangeContext};
use hylo_core::fees::controller::{FeeExtract, LevercoinFees};
use hylo_core::idl::exchange::accounts::ExoPair;
use hylo_core::idl::router::types::ExoEntry;
use hylo_core::pyth::OracleConfig;
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::util::{denormalize_mint_amount, normalize_mint_exp};
use hylo_core::virtual_stablecoin::{validate_burn, VirtualStablecoin};
use hylo_idl::pda;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use serde::{Deserialize, Serialize};

use crate::protocol_state::accounts::decode;
use crate::protocol_state::state::in_stablecoin_oracle_window;
use crate::token_operation::{atom_rate, gate, RedeemOperationOutput};

/// Field count of [`ExoPairAccounts`].
pub const EXO_PAIR_ACCOUNTS: usize = 5;

/// Reads the oracle pubkey from an exo pair account.
pub(crate) fn exo_pair_oracle(exo_pair: &Account) -> Result<Pubkey> {
  decode::<ExoPair>(exo_pair, "Exo pair").map(|pair| pair.oracle)
}

/// Raw accounts for one exo pair.
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
  pub collateral_mint_info: Mint,
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
  /// * Oracle feed id differs from the pair's
  /// * Vault balance normalization, context load, or fixed-point conversion
  pub fn pair_state<C: SolanaClock>(
    &self,
    entry: &ExoEntry,
    clock: C,
  ) -> Result<ExoPairState<C>> {
    ExoPairState::from_accounts(
      entry,
      &self.exo_pair,
      &self.vault,
      &self.levercoin_mint,
      &self.collateral_mint,
      &self.oracle,
      clock,
    )
  }
}

impl<C: SolanaClock> ExoPairState<C> {
  /// Decodes the accounts, checks them against the registry entry, and
  /// loads the pair state under `clock`.
  ///
  /// # Errors
  /// * Account deserialization
  /// * Pair collateral differs from the entry
  /// * Entry levercoin mint is not the pair's levercoin PDA
  /// * Oracle feed id differs from the pair's
  /// * Vault balance normalization, context load, or fixed-point conversion
  pub fn from_accounts(
    entry: &ExoEntry,
    exo_pair: &Account,
    vault: &Account,
    levercoin_mint: &Account,
    collateral_mint: &Account,
    oracle: &Account,
    clock: C,
  ) -> Result<ExoPairState<C>> {
    let exo_pair: ExoPair = decode(exo_pair, "Exo pair")?;
    validate_entry(entry, &exo_pair)?;
    let vault: TokenAccount = decode(vault, "Exo vault")?;
    let levercoin_mint: Mint = decode(levercoin_mint, "Exo levercoin mint")?;
    let collateral_mint: Mint = decode(collateral_mint, "Exo collateral mint")?;
    let oracle: PriceUpdateV2 = decode(oracle, "Exo collateral/USD Pyth feed")?;
    validate_exo_pair_oracle(&exo_pair, &oracle)?;
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
      collateral_mint_info: collateral_mint,
      context,
      paused: exo_pair.paused,
      pool_drawdown: exo_pair.pool_drawdown.into(),
      borrow_rate_harvest_epoch: exo_pair.borrow_rate_harvest_cache.epoch,
      supply_floor: exo_pair.virtual_stablecoin_supply_floor.try_into()?,
      oracle_publish_time: oracle.price_message.publish_time,
      oracle_interval_secs: exo_pair.oracle_interval_secs,
    })
  }

  /// Computes collateral owed before fees within the vault balance.
  ///
  /// # Errors
  /// * NAV, conversion, or insufficient collateral
  pub(crate) fn redemption_fee_base(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<UFix64<N9>, CoreError> {
    let collateral_out = self
      .context
      .exo_conversion()
      .token_to_exo(in_amount, self.context.stablecoin_nav()?)?;
    gate(
      collateral_out <= self.context.total_collateral,
      CoreError::InsufficientLiquidity,
    )?;
    Ok(collateral_out)
  }

  /// Quotes redemption with strict fees without checking pause or freshness.
  /// Rounds collateral to mint precision before normalizing the output.
  ///
  /// # Errors
  /// * Liquidity, fee, burn floor, precision, or marginal rate failure
  pub(crate) fn redemption_quote(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<RedeemOperationOutput, CoreError> {
    let collateral_out = self.redemption_fee_base(in_amount)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self.context.stablecoin_redeem_fee(collateral_out)?;
    validate_burn(
      self.context.virtual_stablecoin_supply()?,
      in_amount,
      self.supply_floor,
    )?;
    let collateral_amount =
      denormalize_mint_amount(&self.collateral_mint_info, amount_remaining)?;
    let out_amount =
      normalize_mint_exp(&self.collateral_mint_info, collateral_amount.bits)?;
    let marginal_rate =
      atom_rate::<N6, N9>(self.context.stablecoin_redeem_marginal(in_amount)?);
    Ok(RedeemOperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: self.collateral_mint,
      fee_base: collateral_out,
      marginal_rate,
    })
  }

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
