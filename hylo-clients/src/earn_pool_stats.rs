//! Earn pool yield statistics for sHYUSD.
//!
//! Realized yield reads the on-chain `HarvestCache` snapshots; projected
//! yield recomputes next-epoch inflows from current protocol parameters
//! with last epoch's LST price appreciation as the growth estimate.
//! All math lives in [`hylo_core::earn_pool_stats`]; this module owns the
//! result types and assembly.

use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar;
use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Result};
use fix::prelude::*;
use hylo_core::borrow_rate::BorrowRateConfig;
use hylo_core::earn_pool_math::lp_token_nav;
use hylo_core::earn_pool_stats::{
  apply_drawdown_offset, epoch_yield_rate, lst_epoch_growth,
  projected_borrow_inflow, projected_lst_inflow, EPOCHS_PER_YEAR,
};
use hylo_core::exchange_context::{ExchangeContext, ExoExchangeContext};
use hylo_core::idl::exchange::accounts::{ExoPair, Hylo, LstHeader};
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::pyth::{query_pyth_oracle, OracleConfig, SOL_USD};
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::yields::{HarvestCache, YieldHarvestConfig};
use hylo_idl::pda;
use hylo_idl::tokens::{StakePool, TokenMint, CBBTC, HYLOSOL, JITOSOL, SHYUSD};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

/// Snapshot of one harvest stream from its on-chain [`HarvestCache`].
#[derive(Debug, Clone, Copy)]
pub struct RealizedHarvest {
  /// Epoch of the most recent harvest for this stream.
  pub epoch: u64,
  /// hyUSD deposited into the pool by that harvest.
  pub hyusd_to_pool: UFix64<N6>,
  /// True if no harvest has landed for the current epoch yet.
  pub is_stale: bool,
}

/// One LST's contribution to the projection: current vault holdings
/// valued in SOL, and last epoch's per-epoch price growth.
#[derive(Debug, Clone, Copy)]
pub struct LstPosition {
  pub sol_value: UFix64<N9>,
  pub epoch_growth: UFix64<N9>,
}

/// Deserialized on-chain inputs for [`compute_stats`].
#[derive(Debug, Clone)]
pub struct StatsInputs {
  pub current_epoch: u64,
  pub pool_balance: UFix64<N6>,
  pub shyusd_supply: UFix64<N6>,
  pub lst_harvest_cache: HarvestCache,
  pub borrow_harvest_cache: HarvestCache,
  pub harvest_config: YieldHarvestConfig,
  pub borrow_rate_config: BorrowRateConfig,
  pub lst_positions: Vec<LstPosition>,
  pub sol_usd_spot: UFix64<N9>,
  pub levercoin_market_cap: UFix64<N9>,
  pub outstanding_drawdown: UFix64<N6>,
}

/// Earn pool yield statistics for sHYUSD.
#[derive(Debug, Clone, Copy)]
pub struct EarnPoolStats {
  /// hyUSD per sHYUSD.
  pub nav: UFix64<N6>,
  /// Current hyUSD in the pool (realized-yield denominator; approximate
  /// if large deposits/withdrawals happened since the last harvest).
  pub pool_balance: UFix64<N6>,
  pub shyusd_supply: UFix64<N6>,
  pub current_epoch: u64,
  /// LST staking-yield stream (`harvest_yield`).
  pub lst_harvest: RealizedHarvest,
  /// cbBTC borrow-rate stream (`harvest_borrow_rate`).
  pub borrow_harvest: RealizedHarvest,
  /// Sum of streams at the most recent harvested epoch, over the pool.
  pub last_epoch_yield_rate: UFix64<N9>,
  /// `(1 + last_epoch_yield_rate)^182 - 1`
  pub naive_apy: f64,
  pub projected_lst_inflow: UFix64<N6>,
  pub projected_borrow_inflow: UFix64<N6>,
  pub outstanding_drawdown: UFix64<N6>,
  /// Net projected inflow next epoch over the pool.
  pub projected_epoch_rate: UFix64<N9>,
  pub projected_apy: f64,
}

/// Compounded annual percentage yield from an `N9` per-epoch rate.
#[allow(clippy::cast_precision_loss)] // advisory stats; f64 suffices
#[must_use]
pub fn annualize(per_epoch_rate: UFix64<N9>) -> f64 {
  let rate = per_epoch_rate.bits as f64 * 1e-9;
  let epochs = EPOCHS_PER_YEAR as f64;
  (1.0 + rate).powf(epochs) - 1.0
}

fn realized(
  harvest_cache: &HarvestCache,
  current_epoch: u64,
) -> Result<RealizedHarvest> {
  Ok(RealizedHarvest {
    epoch: harvest_cache.epoch,
    hyusd_to_pool: harvest_cache.stablecoin_to_pool.try_into()?,
    is_stale: harvest_cache.is_stale(current_epoch),
  })
}

/// Computes yield statistics from deserialized on-chain inputs.
///
/// # Errors
/// * Arithmetic overflow in yield math
/// * Invalid fixed-point data in harvest caches or configs
pub fn compute_stats(inputs: &StatsInputs) -> Result<EarnPoolStats> {
  let nav = lp_token_nav(inputs.pool_balance, inputs.shyusd_supply)?;
  let lst_harvest = realized(&inputs.lst_harvest_cache, inputs.current_epoch)?;
  let borrow_harvest =
    realized(&inputs.borrow_harvest_cache, inputs.current_epoch)?;

  let last_harvest_epoch = lst_harvest.epoch.max(borrow_harvest.epoch);
  let realized_total = [&lst_harvest, &borrow_harvest]
    .iter()
    .filter(|harvest| harvest.epoch == last_harvest_epoch)
    .try_fold(UFix64::zero(), |acc: UFix64<N6>, harvest| {
      acc.checked_add(&harvest.hyusd_to_pool)
    })
    .ok_or_else(|| anyhow!("Realized yield overflow"))?;
  let last_epoch_yield_rate =
    epoch_yield_rate(realized_total, inputs.pool_balance)?;

  let projected_lst = inputs.lst_positions.iter().try_fold(
    UFix64::zero(),
    |acc: UFix64<N6>, position| {
      let inflow = projected_lst_inflow(
        position.sol_value,
        position.epoch_growth,
        inputs.sol_usd_spot,
        &inputs.harvest_config,
      )?;
      acc
        .checked_add(&inflow)
        .ok_or_else(|| anyhow!("Projected LST inflow overflow"))
    },
  )?;
  let projected_borrow = projected_borrow_inflow(
    inputs.levercoin_market_cap,
    &inputs.borrow_rate_config,
  )?;
  let gross = projected_lst
    .checked_add(&projected_borrow)
    .ok_or_else(|| anyhow!("Projected inflow overflow"))?;
  let net = apply_drawdown_offset(gross, inputs.outstanding_drawdown);
  let projected_epoch_rate = epoch_yield_rate(net, inputs.pool_balance)?;

  Ok(EarnPoolStats {
    nav,
    pool_balance: inputs.pool_balance,
    shyusd_supply: inputs.shyusd_supply,
    current_epoch: inputs.current_epoch,
    lst_harvest,
    borrow_harvest,
    last_epoch_yield_rate,
    naive_apy: annualize(last_epoch_yield_rate),
    projected_lst_inflow: projected_lst,
    projected_borrow_inflow: projected_borrow,
    outstanding_drawdown: inputs.outstanding_drawdown,
    projected_epoch_rate,
    projected_apy: annualize(projected_epoch_rate),
  })
}

/// Account keys required for [`EarnPoolStats`], in fetch order.
/// Index constants below must match this list.
#[must_use]
pub fn stats_account_keys() -> Vec<Pubkey> {
  vec![
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
    pda::exo_vault(CBBTC::MINT),
    pda::exo_levercoin_mint(CBBTC::MINT),
    pda::BTC_USD_PYTH_FEED,
    SOL_USD.address,
    sysvar::clock::ID,
  ]
}

const HYLO_IDX: usize = 0;
const JITOSOL_HEADER_IDX: usize = 1;
const HYLOSOL_HEADER_IDX: usize = 2;
const JITOSOL_VAULT_IDX: usize = 3;
const HYLOSOL_VAULT_IDX: usize = 4;
const JITOSOL_POOL_STATE_IDX: usize = 5;
const HYLOSOL_POOL_STATE_IDX: usize = 6;
const HYUSD_POOL_IDX: usize = 7;
const SHYUSD_MINT_IDX: usize = 8;
const EXO_PAIR_IDX: usize = 9;
const EXO_VAULT_IDX: usize = 10;
const XBTC_MINT_IDX: usize = 11;
const BTC_USD_IDX: usize = 12;
const SOL_USD_IDX: usize = 13;
const CLOCK_IDX: usize = 14;

fn require<'a>(
  accounts: &'a [Option<Account>],
  keys: &[Pubkey],
  index: usize,
) -> Result<&'a Account> {
  accounts.get(index).and_then(Option::as_ref).ok_or_else(|| {
    anyhow!("Missing stats account at index {index}: {}", keys[index])
  })
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
    .ok_or_else(|| anyhow!("LST vault SOL value overflow"))?;
  Ok(LstPosition {
    sol_value,
    epoch_growth,
  })
}

/// Builds [`StatsInputs`] from fetched accounts (order of
/// [`stats_account_keys`]).
///
/// # Errors
/// * Missing account or deserialization failure
/// * Oracle validation failure
pub fn build_stats_inputs(
  keys: &[Pubkey],
  accounts: &[Option<Account>],
) -> Result<StatsInputs> {
  let hylo = Hylo::try_deserialize(
    &mut require(accounts, keys, HYLO_IDX)?.data.as_slice(),
  )?;
  let jitosol_header = LstHeader::try_deserialize(
    &mut require(accounts, keys, JITOSOL_HEADER_IDX)?.data.as_slice(),
  )?;
  let hylosol_header = LstHeader::try_deserialize(
    &mut require(accounts, keys, HYLOSOL_HEADER_IDX)?.data.as_slice(),
  )?;
  let jitosol_vault = TokenAccount::try_deserialize(
    &mut require(accounts, keys, JITOSOL_VAULT_IDX)?.data.as_slice(),
  )?;
  let hylosol_vault = TokenAccount::try_deserialize(
    &mut require(accounts, keys, HYLOSOL_VAULT_IDX)?.data.as_slice(),
  )?;
  let jitosol_pool_state = SplStakePool::from_bytes(
    &require(accounts, keys, JITOSOL_POOL_STATE_IDX)?.data,
  )?;
  let hylosol_pool_state = SplStakePool::from_bytes(
    &require(accounts, keys, HYLOSOL_POOL_STATE_IDX)?.data,
  )?;
  let hyusd_pool = TokenAccount::try_deserialize(
    &mut require(accounts, keys, HYUSD_POOL_IDX)?.data.as_slice(),
  )?;
  let shyusd_mint = Mint::try_deserialize(
    &mut require(accounts, keys, SHYUSD_MINT_IDX)?.data.as_slice(),
  )?;
  let exo_pair = ExoPair::try_deserialize(
    &mut require(accounts, keys, EXO_PAIR_IDX)?.data.as_slice(),
  )?;
  let exo_vault = TokenAccount::try_deserialize(
    &mut require(accounts, keys, EXO_VAULT_IDX)?.data.as_slice(),
  )?;
  let xbtc_mint = Mint::try_deserialize(
    &mut require(accounts, keys, XBTC_MINT_IDX)?.data.as_slice(),
  )?;
  let btc_usd = PriceUpdateV2::try_deserialize(
    &mut require(accounts, keys, BTC_USD_IDX)?.data.as_slice(),
  )?;
  let sol_usd = PriceUpdateV2::try_deserialize(
    &mut require(accounts, keys, SOL_USD_IDX)?.data.as_slice(),
  )?;
  let clock: Clock =
    bincode::deserialize(&require(accounts, keys, CLOCK_IDX)?.data)
      .map_err(|e| anyhow!("Failed to deserialize clock: {e}"))?;

  let oracle_config = OracleConfig::new(
    hylo.oracle_interval_secs,
    hylo.oracle_conf_tolerance.try_into()?,
  );
  let sol_usd_spot = query_pyth_oracle(&clock, &sol_usd, oracle_config)?.spot;

  // Mirrors hylo-quotes build_cbbtc_exchange_context: value the xBTC
  // market cap for the borrow-rate projection.
  let exo_oracle_config = OracleConfig::new(
    exo_pair.oracle_interval_secs,
    exo_pair.oracle_conf_tolerance.try_into()?,
  );
  let total_collateral: UFix64<N9> = UFix64::<N8>::new(exo_vault.amount)
    .checked_convert()
    .ok_or_else(|| anyhow!("cbBTC vault amount N8->N9 overflow"))?;
  let exo_context = ExoExchangeContext::load(
    clock.clone(),
    total_collateral,
    exo_pair.stablecoin_mint_threshold.try_into()?,
    exo_oracle_config,
    exo_pair.levercoin_fees.into(),
    &btc_usd,
    exo_pair.virtual_stablecoin.into(),
    Some(&xbtc_mint),
    exo_pair.sell_curve_config.into(),
    exo_pair.buy_curve_config.into(),
    exo_pair.levercoin_market_cap_limit.try_into()?,
  )?;
  let levercoin_market_cap = exo_context.levercoin_market_cap()?;

  let hylo_drawdown: PoolDrawdown = hylo.pool_drawdown.into();
  let exo_drawdown: PoolDrawdown = exo_pair.pool_drawdown.into();
  let outstanding_drawdown = hylo_drawdown
    .outstanding()?
    .checked_add(&exo_drawdown.outstanding()?)
    .ok_or_else(|| anyhow!("Pool drawdown overflow"))?;

  Ok(StatsInputs {
    current_epoch: clock.epoch,
    pool_balance: UFix64::new(hyusd_pool.amount),
    shyusd_supply: UFix64::new(shyusd_mint.supply),
    lst_harvest_cache: hylo.yield_harvest_cache.into(),
    borrow_harvest_cache: exo_pair.borrow_rate_harvest_cache.into(),
    harvest_config: hylo.yield_harvest_config.into(),
    borrow_rate_config: exo_pair.borrow_rate_config.into(),
    lst_positions: vec![
      lst_position(&jitosol_header, &jitosol_vault, &jitosol_pool_state)?,
      lst_position(&hylosol_header, &hylosol_vault, &hylosol_pool_state)?,
    ],
    sol_usd_spot,
    levercoin_market_cap,
    outstanding_drawdown,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn cache(epoch: u64, to_pool_bits: u64) -> HarvestCache {
    HarvestCache {
      epoch,
      stability_pool_cap: UFix64::<N6>::zero().into(),
      stablecoin_to_pool: UFix64::<N6>::new(to_pool_bits).into(),
    }
  }

  fn inputs() -> StatsInputs {
    StatsInputs {
      current_epoch: 800,
      pool_balance: UFix64::<N6>::new(1_000_000_000_000),
      shyusd_supply: UFix64::<N6>::new(950_000_000_000),
      lst_harvest_cache: cache(800, 1_000_000_000),
      borrow_harvest_cache: cache(800, 200_000_000),
      harvest_config: YieldHarvestConfig {
        allocation: UFix64::<N4>::new(10_000).into(),
        fee: UFix64::<N4>::new(1_000).into(),
      },
      borrow_rate_config: BorrowRateConfig::new(
        UFix64::<N9>::new(384_620).into(),
        UFix64::<N4>::new(500).into(),
      ),
      lst_positions: vec![LstPosition {
        sol_value: UFix64::<N9>::new(100_000_000_000_000),
        epoch_growth: UFix64::<N9>::new(500_000),
      }],
      sol_usd_spot: UFix64::<N9>::new(150_000_000_000),
      levercoin_market_cap: UFix64::<N9>::new(1_000_000_000_000_000),
      outstanding_drawdown: UFix64::zero(),
    }
  }

  #[test]
  fn annualize_compounds() {
    // 0.1% per epoch over 182 epochs ~= 19.95% APY
    let apy = annualize(UFix64::<N9>::new(1_000_000));
    assert!((apy - 0.1995).abs() < 1e-3, "apy = {apy}");
  }

  #[test]
  fn annualize_zero_rate() {
    let apy = annualize(UFix64::zero());
    assert!(apy.abs() < f64::EPSILON);
  }

  #[test]
  fn compute_stats_realized_sums_matching_epochs() -> Result<()> {
    let stats = compute_stats(&inputs())?;
    // 1,000 + 200 hyUSD over 1,000,000 pool = 0.12% per epoch
    assert_eq!(stats.last_epoch_yield_rate, UFix64::<N9>::new(1_200_000));
    assert!(!stats.lst_harvest.is_stale);
    assert!(!stats.borrow_harvest.is_stale);
    Ok(())
  }

  #[test]
  fn compute_stats_ignores_older_stream_epoch() -> Result<()> {
    let mut input = inputs();
    input.borrow_harvest_cache = cache(799, 200_000_000);
    let stats = compute_stats(&input)?;
    // Only the LST stream (epoch 800) counts: 0.1% per epoch
    assert_eq!(stats.last_epoch_yield_rate, UFix64::<N9>::new(1_000_000));
    assert!(stats.borrow_harvest.is_stale);
    Ok(())
  }

  #[test]
  fn compute_stats_projection_pipeline() -> Result<()> {
    let stats = compute_stats(&inputs())?;
    // LST: $6,750 (Task 3 fixture); borrow: $365.389
    assert_eq!(stats.projected_lst_inflow, UFix64::<N6>::new(6_750_000_000));
    assert_eq!(
      stats.projected_borrow_inflow,
      UFix64::<N6>::new(365_389_000)
    );
    // (6,750 + 365.389) / 1,000,000 = 0.7115389% per epoch
    assert_eq!(stats.projected_epoch_rate, UFix64::<N9>::new(7_115_389));
    assert!(stats.projected_apy > stats.naive_apy);
    Ok(())
  }

  #[test]
  fn compute_stats_drawdown_reduces_projection() -> Result<()> {
    let mut input = inputs();
    input.outstanding_drawdown = UFix64::<N6>::new(7_115_389_000);
    let stats = compute_stats(&input)?;
    assert_eq!(stats.projected_epoch_rate, UFix64::zero());
    assert!(stats.projected_apy.abs() < f64::EPSILON);
    Ok(())
  }

  #[test]
  fn compute_stats_nav() -> Result<()> {
    let stats = compute_stats(&inputs())?;
    // 1,000,000 / 950,000 (ceil) = 1.052632
    assert_eq!(stats.nav, UFix64::<N6>::new(1_052_632));
    Ok(())
  }

  #[test]
  fn stats_account_keys_order() {
    let keys = stats_account_keys();
    assert_eq!(keys.len(), 15);
    assert_eq!(keys[0], hylo_idl::pda::HYLO);
    assert_eq!(keys[7], hylo_idl::pda::HYUSD_POOL);
    assert_eq!(keys[14], anchor_lang::solana_program::sysvar::clock::ID);
  }
}
