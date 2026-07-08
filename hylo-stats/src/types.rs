//! Data types for earn pool yield statistics.

use anchor_lang::prelude::Pubkey;
use fix::prelude::*;
use hylo_core::borrow_rate::BorrowRateConfig;
use hylo_core::yields::{HarvestCache, YieldHarvestConfig};

/// Snapshot of one harvest stream from its on-chain [`HarvestCache`]:
/// the most recent harvest's epoch, the hyUSD it deposited into the
/// pool, and staleness (no harvest yet for the current epoch).
#[derive(Debug, Clone, Copy)]
pub struct RealizedHarvest {
  pub epoch: u64,
  pub hyusd_to_pool: UFix64<N6>,
  pub is_stale: bool,
}

/// One LST's projection inputs: vault SOL value and per-epoch growth.
#[derive(Debug, Clone, Copy)]
pub struct LstPosition {
  pub sol_value: UFix64<N9>,
  pub epoch_growth: UFix64<N9>,
}

/// One exogenous-collateral pair's inputs for the borrow-rate stream,
/// labeled by its collateral mint.
#[derive(Debug, Clone, Copy)]
pub struct ExoSnapshot {
  pub collateral_mint: Pubkey,
  pub harvest_cache: HarvestCache,
  pub borrow_rate_config: BorrowRateConfig,
  pub levercoin_market_cap: UFix64<N9>,
}

/// Deserialized on-chain inputs for
/// [`compute_stats`](crate::earn_pool_stats::compute_stats).
#[derive(Debug, Clone)]
pub struct StatsInputs {
  pub current_epoch: u64,
  pub pool_balance: UFix64<N6>,
  pub shyusd_supply: UFix64<N6>,
  pub lst_harvest_cache: HarvestCache,
  pub harvest_config: YieldHarvestConfig,
  pub lst_positions: Vec<LstPosition>,
  pub exo_snapshots: Vec<ExoSnapshot>,
  pub sol_usd_spot: UFix64<N9>,
  pub outstanding_drawdown: UFix64<N6>,
  pub epochs_per_year: f64,
}

/// Per-stream results for one exo borrow-rate stream, labeled by its
/// collateral mint: realized harvest snapshot and projected next-epoch
/// hyUSD inflow.
#[derive(Debug, Clone, Copy)]
pub struct ExoStats {
  pub collateral_mint: Pubkey,
  pub harvest: RealizedHarvest,
  pub projected_inflow: UFix64<N6>,
}

/// Earn pool yield statistics for sHYUSD.
///
/// * `nav` — hyUSD per sHYUSD
/// * `pool_balance` — current hyUSD in the pool, the denominator for both yield
///   rates
/// * `epochs_per_year` — annualization basis used for both APYs
/// * `lst_harvest` — LST staking-yield stream (`harvest_yield`); `exo_stats` —
///   exo borrow-rate streams (`harvest_borrow_rate`), one per pair
/// * `last_epoch_yield_rate` — sum of the LST stream plus all exo streams at
///   the most recent harvested epoch, over the pool; `naive_apy` — `(1 +
///   last_epoch_yield_rate)^epochs_per_year - 1`
/// * `projected_epoch_rate` — net projected inflow next epoch (LST + exo, minus
///   outstanding drawdown) over the pool; `projected_apy` — its annualization
#[derive(Debug, Clone)]
pub struct EarnPoolStats {
  pub nav: UFix64<N6>,
  pub pool_balance: UFix64<N6>,
  pub shyusd_supply: UFix64<N6>,
  pub current_epoch: u64,
  pub epochs_per_year: f64,
  pub lst_harvest: RealizedHarvest,
  pub exo_stats: Vec<ExoStats>,
  pub last_epoch_yield_rate: UFix64<N9>,
  pub naive_apy: f64,
  pub projected_lst_inflow: UFix64<N6>,
  pub projected_exo_inflow: UFix64<N6>,
  pub outstanding_drawdown: UFix64<N6>,
  pub projected_epoch_rate: UFix64<N9>,
  pub projected_apy: f64,
}
