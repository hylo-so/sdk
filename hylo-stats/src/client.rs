//! Read-only fetch layer for earn pool yield statistics.

use std::sync::Arc;

use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar;
use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::Result;
use fix::prelude::*;
use hylo_core::exchange_context::{ExchangeContext, ExoExchangeContext};
use hylo_core::idl::exchange::accounts::{ExoPair, Hylo, LstHeader};
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::pyth::{query_pyth_oracle, OracleConfig};
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::util::normalize_mint_exp;
use hylo_idl::pda;
use hylo_idl::tokens::{StakePool, TokenMint, CBBTC, HYLOSOL, JITOSOL, SHYUSD};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::earn_pool_stats::compute_stats;
use crate::earn_pool_yield_math::lst_epoch_growth;
use crate::error::StatsError::{
  AccountCountMismatch, ClockDeserialize, LstVaultValueOverflow,
  MissingAccounts, NoBlockAtOrAfterSlot, NoPreviousEpoch,
  NonPositiveEpochDuration, PoolDrawdownOverflow,
};
use crate::types::{EarnPoolStats, ExoSnapshot, LstPosition, StatsInputs};

/// Seconds in a Julian year.
const SECONDS_PER_YEAR: f64 = 31_557_600.0;

/// Number of accounts fetched for [`EarnPoolStats`].
pub const STATS_ACCOUNT_COUNT: usize = 16;

/// Account keys required for [`EarnPoolStats`], in fetch order —
/// the same order [`StatsAccounts::from_fetched`] deserializes.
pub const STATS_ACCOUNT_KEYS: [Pubkey; STATS_ACCOUNT_COUNT] = [
  pda::HYLO,
  pda::lst_header(JITOSOL::MINT),
  pda::lst_header(HYLOSOL::MINT),
  pda::lst_vault(JITOSOL::MINT),
  pda::lst_vault(HYLOSOL::MINT),
  JITOSOL::POOL_STATE,
  HYLOSOL::POOL_STATE,
  pda::HYUSD_POOL,
  SHYUSD::MINT,
  pda::exo_pair(CBBTC::MINT),
  CBBTC::MINT,
  pda::exo_vault(CBBTC::MINT),
  pda::exo_levercoin_mint(CBBTC::MINT),
  pda::BTC_USD_PYTH_FEED,
  pda::SOL_USD_PYTH_FEED,
  sysvar::clock::ID,
];

/// Deserialized onchain accounts backing one stats fetch, in
/// [`STATS_ACCOUNT_KEYS`] order.
#[derive(Clone)]
pub struct StatsAccounts {
  pub hylo: Hylo,
  pub jitosol_header: LstHeader,
  pub hylosol_header: LstHeader,
  pub jitosol_vault: TokenAccount,
  pub hylosol_vault: TokenAccount,
  pub jitosol_pool_state: SplStakePool,
  pub hylosol_pool_state: SplStakePool,
  pub hyusd_pool: TokenAccount,
  pub shyusd_mint: Mint,
  pub exo_pair: ExoPair,
  pub exo_collateral_mint: Mint,
  pub exo_vault: TokenAccount,
  pub exo_levercoin_mint: Mint,
  pub btc_usd: PriceUpdateV2,
  pub sol_usd: PriceUpdateV2,
  pub clock: Clock,
}

impl StatsAccounts {
  /// Deserializes a fetched account list, erroring with the keys of
  /// any missing accounts.
  ///
  /// # Errors
  /// * Missing account, count mismatch, or deserialization failure
  pub fn from_fetched(fetched: Vec<Option<Account>>) -> Result<StatsAccounts> {
    let actual = fetched.len();
    let missing = STATS_ACCOUNT_KEYS
      .iter()
      .zip(&fetched)
      .filter(|(_, account)| account.is_none())
      .map(|(key, _)| *key)
      .collect::<Vec<Pubkey>>();
    if actual != STATS_ACCOUNT_COUNT {
      Err(
        AccountCountMismatch {
          expected: STATS_ACCOUNT_COUNT,
          actual,
        }
        .into(),
      )
    } else if !missing.is_empty() {
      Err(MissingAccounts(missing).into())
    } else {
      let accounts = fetched.into_iter().flatten().collect::<Vec<Account>>();
      Ok(StatsAccounts {
        hylo: Hylo::try_deserialize(&mut accounts[0].data.as_slice())?,
        jitosol_header: LstHeader::try_deserialize(
          &mut accounts[1].data.as_slice(),
        )?,
        hylosol_header: LstHeader::try_deserialize(
          &mut accounts[2].data.as_slice(),
        )?,
        jitosol_vault: TokenAccount::try_deserialize(
          &mut accounts[3].data.as_slice(),
        )?,
        hylosol_vault: TokenAccount::try_deserialize(
          &mut accounts[4].data.as_slice(),
        )?,
        jitosol_pool_state: SplStakePool::from_bytes(&accounts[5].data)?,
        hylosol_pool_state: SplStakePool::from_bytes(&accounts[6].data)?,
        hyusd_pool: TokenAccount::try_deserialize(
          &mut accounts[7].data.as_slice(),
        )?,
        shyusd_mint: Mint::try_deserialize(&mut accounts[8].data.as_slice())?,
        exo_pair: ExoPair::try_deserialize(&mut accounts[9].data.as_slice())?,
        exo_collateral_mint: Mint::try_deserialize(
          &mut accounts[10].data.as_slice(),
        )?,
        exo_vault: TokenAccount::try_deserialize(
          &mut accounts[11].data.as_slice(),
        )?,
        exo_levercoin_mint: Mint::try_deserialize(
          &mut accounts[12].data.as_slice(),
        )?,
        btc_usd: PriceUpdateV2::try_deserialize(
          &mut accounts[13].data.as_slice(),
        )?,
        sol_usd: PriceUpdateV2::try_deserialize(
          &mut accounts[14].data.as_slice(),
        )?,
        clock: bincode::deserialize(&accounts[15].data)
          .map_err(ClockDeserialize)?,
      })
    }
  }
}

/// Read-only client for earn pool yield statistics. Needs no keypair
/// or program client.
#[derive(Clone)]
pub struct StatsClient {
  rpc: Arc<RpcClient>,
}

impl StatsClient {
  #[must_use]
  pub fn new(rpc: Arc<RpcClient>) -> StatsClient {
    StatsClient { rpc }
  }

  /// Fetches [`EarnPoolStats`] from current onchain state: one
  /// slot-consistent `get_multiple_accounts` call, plus an
  /// epoch-schedule fetch and two epoch-boundary block-time lookups to
  /// measure the last completed epoch's duration.
  ///
  /// # Errors
  /// * RPC fetch, deserialization, or oracle validation failure
  /// * Epoch duration measurement failure
  /// * Arithmetic overflow in yield math
  pub async fn earn_pool_stats(&self) -> Result<EarnPoolStats> {
    let fetched = self.rpc.get_multiple_accounts(&STATS_ACCOUNT_KEYS).await?;
    let accounts = StatsAccounts::from_fetched(fetched)?;
    let epochs_per_year =
      self.measure_epochs_per_year(accounts.clock.epoch).await?;
    compute_stats(&build_stats_inputs(&accounts, epochs_per_year)?)
  }

  /// Measures the last completed epoch's exact wall-clock duration
  /// from block times at the epoch boundary slots, returning epochs
  /// per year.
  ///
  /// # Errors
  /// * RPC failure, missing boundary blocks, or non-positive duration
  #[allow(clippy::cast_precision_loss)]
  pub async fn measure_epochs_per_year(
    &self,
    current_epoch: u64,
  ) -> Result<f64> {
    let prev_epoch = current_epoch.checked_sub(1).ok_or(NoPreviousEpoch)?;
    let schedule = self.rpc.get_epoch_schedule().await?;
    let start_prev = schedule.get_first_slot_in_epoch(prev_epoch);
    let start_curr = schedule.get_first_slot_in_epoch(current_epoch);
    let t0 = self.block_time_at_or_after(start_prev).await?;
    let t1 = self.block_time_at_or_after(start_curr).await?;
    let duration = t1
      .checked_sub(t0)
      .filter(|d| *d > 0)
      .ok_or(NonPositiveEpochDuration)?;
    Ok(SECONDS_PER_YEAR / duration as f64)
  }

  /// Block time of the first block at or after `slot` (epoch boundary
  /// slots can be skipped).
  async fn block_time_at_or_after(&self, slot: u64) -> Result<i64> {
    let slots = self.rpc.get_blocks_with_limit(slot, 1).await?;
    let first = slots.first().copied().ok_or(NoBlockAtOrAfterSlot(slot))?;
    Ok(self.rpc.get_block_time(first).await?)
  }
}

/// Values an exo pair's levercoin market cap for the borrow-rate
/// projection. Mirrors hylo-quotes `build_cbbtc_exchange_context`.
fn exo_levercoin_market_cap(
  clock: &Clock,
  exo_pair: &ExoPair,
  collateral_mint: &Mint,
  exo_vault: &TokenAccount,
  levercoin_mint: &Mint,
  collateral_usd: &PriceUpdateV2,
) -> Result<UFix64<N9>> {
  let oracle_config = OracleConfig::new(
    exo_pair.oracle_interval_secs,
    exo_pair.oracle_conf_tolerance.try_into()?,
  );
  let total_collateral = normalize_mint_exp(collateral_mint, exo_vault.amount)?;
  let exo_context = ExoExchangeContext::load(
    clock.clone(),
    total_collateral,
    exo_pair.stablecoin_mint_threshold.try_into()?,
    oracle_config,
    exo_pair.levercoin_fees.into(),
    collateral_usd,
    exo_pair.virtual_stablecoin.into(),
    Some(levercoin_mint),
    exo_pair.sell_curve_config.into(),
    exo_pair.buy_curve_config.into(),
    exo_pair.levercoin_market_cap_limit.try_into()?,
  )?;
  let market_cap = exo_context.levercoin_market_cap()?;
  Ok(market_cap)
}

/// Sums outstanding pool drawdown across the LST pair and cbBTC exo pair.
fn total_outstanding_drawdown(
  hylo: &Hylo,
  exo_pair: &ExoPair,
) -> Result<UFix64<N6>> {
  let hylo_drawdown: PoolDrawdown = hylo.pool_drawdown.into();
  let exo_drawdown: PoolDrawdown = exo_pair.pool_drawdown.into();
  Ok(
    hylo_drawdown
      .outstanding()?
      .checked_add(&exo_drawdown.outstanding()?)
      .ok_or(PoolDrawdownOverflow)?,
  )
}

fn lst_position(
  header: &LstHeader,
  vault: &TokenAccount,
  stake_pool: &SplStakePool,
) -> Result<LstPosition> {
  let price_sol: LstSolPrice = header.price_sol.into();
  let prev_price_sol: LstSolPrice = header.prev_price_sol.into();
  let epoch_growth = lst_epoch_growth(&price_sol, &prev_price_sol)?;
  let lst_sol_price: UFix64<N9> = stake_pool.true_price()?.price.try_into()?;
  let sol_value = UFix64::<N9>::new(vault.amount)
    .mul_div_floor(lst_sol_price, UFix64::one())
    .ok_or(LstVaultValueOverflow)?;
  Ok(LstPosition {
    sol_value,
    epoch_growth,
  })
}

/// Builds [`StatsInputs`] from deserialized accounts.
///
/// # Errors
/// * Oracle validation failure
/// * Arithmetic overflow
pub fn build_stats_inputs(
  accounts: &StatsAccounts,
  epochs_per_year: f64,
) -> Result<StatsInputs> {
  let oracle_config = OracleConfig::new(
    accounts.hylo.oracle_interval_secs,
    accounts.hylo.oracle_conf_tolerance.try_into()?,
  );
  let sol_usd_spot =
    query_pyth_oracle(&accounts.clock, &accounts.sol_usd, oracle_config)?.spot;

  let levercoin_market_cap = exo_levercoin_market_cap(
    &accounts.clock,
    &accounts.exo_pair,
    &accounts.exo_collateral_mint,
    &accounts.exo_vault,
    &accounts.exo_levercoin_mint,
    &accounts.btc_usd,
  )?;
  let outstanding_drawdown =
    total_outstanding_drawdown(&accounts.hylo, &accounts.exo_pair)?;

  Ok(StatsInputs {
    current_epoch: accounts.clock.epoch,
    pool_balance: UFix64::new(accounts.hyusd_pool.amount),
    shyusd_supply: UFix64::new(accounts.shyusd_mint.supply),
    lst_harvest_cache: accounts.hylo.yield_harvest_cache.into(),
    harvest_config: accounts.hylo.yield_harvest_config.into(),
    lst_positions: vec![
      lst_position(
        &accounts.jitosol_header,
        &accounts.jitosol_vault,
        &accounts.jitosol_pool_state,
      )?,
      lst_position(
        &accounts.hylosol_header,
        &accounts.hylosol_vault,
        &accounts.hylosol_pool_state,
      )?,
    ],
    exo_snapshots: vec![ExoSnapshot {
      collateral_mint: CBBTC::MINT,
      harvest_cache: accounts.exo_pair.borrow_rate_harvest_cache.into(),
      borrow_rate_config: accounts.exo_pair.borrow_rate_config.into(),
      levercoin_market_cap,
    }],
    sol_usd_spot,
    outstanding_drawdown,
    epochs_per_year,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn stats_account_keys_order() {
    assert_eq!(STATS_ACCOUNT_KEYS[0], hylo_idl::pda::HYLO);
    assert_eq!(STATS_ACCOUNT_KEYS[7], hylo_idl::pda::HYUSD_POOL);
    assert_eq!(
      STATS_ACCOUNT_KEYS[STATS_ACCOUNT_COUNT - 1],
      anchor_lang::solana_program::sysvar::clock::ID
    );
  }
}
