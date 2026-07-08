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
/// the same order [`build_stats_inputs`] destructures.
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
    let accounts = resolve_stats_accounts(fetched)?;
    let clock: Clock =
      bincode::deserialize(&accounts[STATS_ACCOUNT_COUNT - 1].data)
        .map_err(ClockDeserialize)?;
    let epochs_per_year = self.measure_epochs_per_year(clock.epoch).await?;
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

/// Resolves a fetched account list, erroring with the keys of any
/// missing accounts.
fn resolve_stats_accounts(
  accounts: Vec<Option<Account>>,
) -> Result<[Account; STATS_ACCOUNT_COUNT]> {
  let missing = STATS_ACCOUNT_KEYS
    .iter()
    .zip(&accounts)
    .filter(|(_, account)| account.is_none())
    .map(|(key, _)| *key)
    .collect::<Vec<Pubkey>>();
  if missing.is_empty() {
    Ok(
      accounts
        .into_iter()
        .flatten()
        .collect::<Vec<Account>>()
        .try_into()
        .map_err(|accounts: Vec<Account>| AccountCountMismatch {
          expected: STATS_ACCOUNT_COUNT,
          actual: accounts.len(),
        })?,
    )
  } else {
    Err(MissingAccounts(missing).into())
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

/// Builds [`StatsInputs`] from fetched accounts (order of
/// [`STATS_ACCOUNT_KEYS`]).
///
/// # Errors
/// * Deserialization failure
/// * Oracle validation failure
pub fn build_stats_inputs(
  accounts: &[Account; STATS_ACCOUNT_COUNT],
  epochs_per_year: f64,
) -> Result<StatsInputs> {
  let [hylo, jitosol_header, hylosol_header, jitosol_vault, hylosol_vault, jitosol_pool_state, hylosol_pool_state, hyusd_pool, shyusd_mint, exo_pair, exo_collateral_mint, exo_vault, exo_levercoin_mint, btc_usd, sol_usd, clock] =
    accounts;

  let hylo = Hylo::try_deserialize(&mut hylo.data.as_slice())?;
  let jitosol_header =
    LstHeader::try_deserialize(&mut jitosol_header.data.as_slice())?;
  let hylosol_header =
    LstHeader::try_deserialize(&mut hylosol_header.data.as_slice())?;
  let jitosol_vault =
    TokenAccount::try_deserialize(&mut jitosol_vault.data.as_slice())?;
  let hylosol_vault =
    TokenAccount::try_deserialize(&mut hylosol_vault.data.as_slice())?;
  let jitosol_pool_state = SplStakePool::from_bytes(&jitosol_pool_state.data)?;
  let hylosol_pool_state = SplStakePool::from_bytes(&hylosol_pool_state.data)?;
  let hyusd_pool =
    TokenAccount::try_deserialize(&mut hyusd_pool.data.as_slice())?;
  let shyusd_mint = Mint::try_deserialize(&mut shyusd_mint.data.as_slice())?;
  let exo_pair = ExoPair::try_deserialize(&mut exo_pair.data.as_slice())?;
  let exo_collateral_mint =
    Mint::try_deserialize(&mut exo_collateral_mint.data.as_slice())?;
  let exo_vault =
    TokenAccount::try_deserialize(&mut exo_vault.data.as_slice())?;
  let exo_levercoin_mint =
    Mint::try_deserialize(&mut exo_levercoin_mint.data.as_slice())?;
  let btc_usd = PriceUpdateV2::try_deserialize(&mut btc_usd.data.as_slice())?;
  let sol_usd = PriceUpdateV2::try_deserialize(&mut sol_usd.data.as_slice())?;
  let clock: Clock =
    bincode::deserialize(&clock.data).map_err(ClockDeserialize)?;

  let oracle_config = OracleConfig::new(
    hylo.oracle_interval_secs,
    hylo.oracle_conf_tolerance.try_into()?,
  );
  let sol_usd_spot = query_pyth_oracle(&clock, &sol_usd, oracle_config)?.spot;

  let levercoin_market_cap = exo_levercoin_market_cap(
    &clock,
    &exo_pair,
    &exo_collateral_mint,
    &exo_vault,
    &exo_levercoin_mint,
    &btc_usd,
  )?;
  let outstanding_drawdown = total_outstanding_drawdown(&hylo, &exo_pair)?;

  Ok(StatsInputs {
    current_epoch: clock.epoch,
    pool_balance: UFix64::new(hyusd_pool.amount),
    shyusd_supply: UFix64::new(shyusd_mint.supply),
    lst_harvest_cache: hylo.yield_harvest_cache.into(),
    harvest_config: hylo.yield_harvest_config.into(),
    lst_positions: vec![
      lst_position(&jitosol_header, &jitosol_vault, &jitosol_pool_state)?,
      lst_position(&hylosol_header, &hylosol_vault, &hylosol_pool_state)?,
    ],
    exo_snapshots: vec![ExoSnapshot {
      collateral_mint: CBBTC::MINT,
      harvest_cache: exo_pair.borrow_rate_harvest_cache.into(),
      borrow_rate_config: exo_pair.borrow_rate_config.into(),
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
