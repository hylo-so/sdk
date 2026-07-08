//! Earn pool yield statistics for sHYUSD: realized last-epoch yield,
//! naive APY, and projected next-epoch yield.

use anyhow::Result;
use fix::prelude::*;
use hylo_core::earn_pool_math::lp_token_nav;
use hylo_core::yields::{HarvestCache, YieldHarvestConfig};

use crate::earn_pool_yield_math::{
  apply_drawdown_offset, epoch_yield_rate, projected_borrow_inflow,
  projected_lst_inflow, EPOCHS_PER_YEAR,
};
use crate::error::StatsError::{
  ProjectedExoInflowOverflow, ProjectedInflowOverflow,
  ProjectedLstInflowOverflow, RealizedYieldOverflow,
};
use crate::types::{
  EarnPoolStats, ExoSnapshot, ExoStats, LstPosition, RealizedHarvest,
  StatsInputs,
};

/// Compounded APY from an `N9` per-epoch rate at a given annualization
/// basis (epochs per year).
#[allow(clippy::cast_precision_loss)] // advisory stats; f64 suffices
#[must_use]
pub fn annualize_with(per_epoch_rate: UFix64<N9>, epochs_per_year: f64) -> f64 {
  let rate = per_epoch_rate.bits as f64 * 1e-9;
  (1.0 + rate).powf(epochs_per_year) - 1.0
}

/// Compounded APY at the protocol's 182-epochs/year convention.
#[allow(clippy::cast_precision_loss)] // advisory stats; f64 suffices
#[must_use]
pub fn annualize(per_epoch_rate: UFix64<N9>) -> f64 {
  annualize_with(per_epoch_rate, EPOCHS_PER_YEAR as f64)
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

/// Realized harvest and projected inflow for one exo snapshot.
fn exo_snapshot_stats(
  snapshot: &ExoSnapshot,
  current_epoch: u64,
) -> Result<ExoStats> {
  Ok(ExoStats {
    collateral_mint: snapshot.collateral_mint,
    harvest: realized(&snapshot.harvest_cache, current_epoch)?,
    projected_inflow: projected_borrow_inflow(
      snapshot.levercoin_market_cap,
      &snapshot.borrow_rate_config,
    )?,
  })
}

/// Realized per-epoch yield rate: harvests at the most recent
/// harvested epoch across all streams, summed over the pool.
fn realized_yield_rate(
  lst_harvest: &RealizedHarvest,
  exo_stats: &[ExoStats],
  pool_balance: UFix64<N6>,
) -> Result<UFix64<N9>> {
  let last_harvest_epoch = exo_stats
    .iter()
    .map(|stats| stats.harvest.epoch)
    .fold(lst_harvest.epoch, u64::max);
  let realized_total = std::iter::once(lst_harvest)
    .chain(exo_stats.iter().map(|stats| &stats.harvest))
    .filter(|harvest| harvest.epoch == last_harvest_epoch)
    .try_fold(UFix64::zero(), |acc: UFix64<N6>, harvest| {
      acc.checked_add(&harvest.hyusd_to_pool)
    })
    .ok_or(RealizedYieldOverflow)?;
  epoch_yield_rate(realized_total, pool_balance)
}

/// Sum of projected next-epoch hyUSD inflows across LST positions.
fn projected_lst_total(
  lst_positions: &[LstPosition],
  sol_usd_spot: UFix64<N9>,
  harvest_config: &YieldHarvestConfig,
) -> Result<UFix64<N6>> {
  lst_positions.iter().try_fold(
    UFix64::zero(),
    |acc: UFix64<N6>, position| -> Result<UFix64<N6>> {
      let inflow = projected_lst_inflow(
        position.sol_value,
        position.epoch_growth,
        sol_usd_spot,
        harvest_config,
      )?;
      Ok(acc.checked_add(&inflow).ok_or(ProjectedLstInflowOverflow)?)
    },
  )
}

/// Sum of projected next-epoch hyUSD inflows across exo streams.
fn projected_exo_total(exo_stats: &[ExoStats]) -> Result<UFix64<N6>> {
  Ok(
    exo_stats
      .iter()
      .try_fold(UFix64::zero(), |acc: UFix64<N6>, stats| {
        acc.checked_add(&stats.projected_inflow)
      })
      .ok_or(ProjectedExoInflowOverflow)?,
  )
}

/// Projected per-epoch rate: gross inflow net of outstanding
/// drawdown, over the pool.
fn projected_rate(
  projected_lst: UFix64<N6>,
  projected_exo: UFix64<N6>,
  outstanding_drawdown: UFix64<N6>,
  pool_balance: UFix64<N6>,
) -> Result<UFix64<N9>> {
  let gross = projected_lst
    .checked_add(&projected_exo)
    .ok_or(ProjectedInflowOverflow)?;
  let net = apply_drawdown_offset(gross, outstanding_drawdown);
  epoch_yield_rate(net, pool_balance)
}

/// Computes yield statistics from deserialized on-chain inputs.
///
/// # Errors
/// * Arithmetic overflow in yield math
/// * Invalid fixed-point data in harvest caches or configs
pub fn compute_stats(inputs: &StatsInputs) -> Result<EarnPoolStats> {
  let nav = lp_token_nav(inputs.pool_balance, inputs.shyusd_supply)?;
  let lst_harvest = realized(&inputs.lst_harvest_cache, inputs.current_epoch)?;
  let exo_stats = inputs
    .exo_snapshots
    .iter()
    .map(|snapshot| exo_snapshot_stats(snapshot, inputs.current_epoch))
    .collect::<Result<Vec<ExoStats>>>()?;
  let last_epoch_yield_rate =
    realized_yield_rate(&lst_harvest, &exo_stats, inputs.pool_balance)?;
  let projected_lst_inflow = projected_lst_total(
    &inputs.lst_positions,
    inputs.sol_usd_spot,
    &inputs.harvest_config,
  )?;
  let projected_exo_inflow = projected_exo_total(&exo_stats)?;
  let projected_epoch_rate = projected_rate(
    projected_lst_inflow,
    projected_exo_inflow,
    inputs.outstanding_drawdown,
    inputs.pool_balance,
  )?;

  Ok(EarnPoolStats {
    nav,
    pool_balance: inputs.pool_balance,
    shyusd_supply: inputs.shyusd_supply,
    current_epoch: inputs.current_epoch,
    epochs_per_year: inputs.epochs_per_year,
    lst_harvest,
    exo_stats,
    last_epoch_yield_rate,
    naive_apy: annualize_with(last_epoch_yield_rate, inputs.epochs_per_year),
    projected_lst_inflow,
    projected_exo_inflow,
    outstanding_drawdown: inputs.outstanding_drawdown,
    projected_epoch_rate,
    projected_apy: annualize_with(projected_epoch_rate, inputs.epochs_per_year),
  })
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::Pubkey;
  use hylo_core::borrow_rate::BorrowRateConfig;
  use hylo_core::yields::YieldHarvestConfig;

  use super::*;
  use crate::types::{ExoSnapshot, LstPosition};

  fn cache(epoch: u64, to_pool_bits: u64) -> HarvestCache {
    HarvestCache {
      epoch,
      stability_pool_cap: UFix64::<N6>::zero().into(),
      stablecoin_to_pool: UFix64::<N6>::new(to_pool_bits).into(),
    }
  }

  fn exo_snapshot(
    harvest_cache: HarvestCache,
    levercoin_market_cap: UFix64<N9>,
  ) -> ExoSnapshot {
    ExoSnapshot {
      collateral_mint: Pubkey::new_unique(),
      harvest_cache,
      borrow_rate_config: BorrowRateConfig::new(
        UFix64::<N9>::new(384_620).into(),
        UFix64::<N4>::new(500).into(),
      ),
      levercoin_market_cap,
    }
  }

  fn inputs() -> StatsInputs {
    StatsInputs {
      current_epoch: 800,
      pool_balance: UFix64::<N6>::new(1_000_000_000_000),
      shyusd_supply: UFix64::<N6>::new(950_000_000_000),
      lst_harvest_cache: cache(800, 1_000_000_000),
      harvest_config: YieldHarvestConfig {
        allocation: UFix64::<N4>::new(10_000).into(),
        fee: UFix64::<N4>::new(1_000).into(),
      },
      lst_positions: vec![LstPosition {
        sol_value: UFix64::<N9>::new(100_000_000_000_000),
        epoch_growth: UFix64::<N9>::new(500_000),
      }],
      exo_snapshots: vec![exo_snapshot(
        cache(800, 200_000_000),
        UFix64::<N9>::new(1_000_000_000_000_000),
      )],
      sol_usd_spot: UFix64::<N9>::new(150_000_000_000),
      outstanding_drawdown: UFix64::zero(),
      epochs_per_year: 182.0,
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
  fn annualize_with_matches_convention() {
    let rate = UFix64::<N9>::new(1_000_000);
    let diff = (annualize_with(rate, 182.0) - annualize(rate)).abs();
    assert!(diff < f64::EPSILON);
  }

  #[test]
  fn annualize_with_lower_basis_lowers_apy() {
    let rate = UFix64::<N9>::new(2_000_000);
    let low = annualize_with(rate, 166.0);
    let high = annualize_with(rate, 182.0);
    assert!(low < high, "low = {low}, high = {high}");
    let expected = (1.002f64).powf(166.0) - 1.0;
    assert!((low - expected).abs() < 1e-12, "low = {low}");
  }

  #[test]
  fn compute_stats_uses_given_basis() -> Result<()> {
    let mut input = inputs();
    input.epochs_per_year = 100.0;
    let stats = compute_stats(&input)?;
    assert!((stats.epochs_per_year - 100.0).abs() < f64::EPSILON);
    let expected = annualize_with(stats.last_epoch_yield_rate, 100.0);
    assert!((stats.naive_apy - expected).abs() < f64::EPSILON);
    Ok(())
  }

  #[test]
  fn compute_stats_realized_sums_matching_epochs() -> Result<()> {
    let stats = compute_stats(&inputs())?;
    // 1,000 + 200 hyUSD over 1,000,000 pool = 0.12% per epoch
    assert_eq!(stats.last_epoch_yield_rate, UFix64::<N9>::new(1_200_000));
    assert!(!stats.lst_harvest.is_stale);
    assert!(!stats.exo_stats[0].harvest.is_stale);
    Ok(())
  }

  #[test]
  fn compute_stats_ignores_older_stream_epoch() -> Result<()> {
    let mut input = inputs();
    input.exo_snapshots[0].harvest_cache = cache(799, 200_000_000);
    let stats = compute_stats(&input)?;
    // Only the LST stream (epoch 800) counts: 0.1% per epoch
    assert_eq!(stats.last_epoch_yield_rate, UFix64::<N9>::new(1_000_000));
    assert!(stats.exo_stats[0].harvest.is_stale);
    Ok(())
  }

  #[test]
  fn compute_stats_two_exo_snapshots() -> Result<()> {
    let mut input = inputs();
    input.exo_snapshots = vec![
      exo_snapshot(
        cache(800, 200_000_000),
        UFix64::<N9>::new(1_000_000_000_000_000),
      ),
      exo_snapshot(
        cache(799, 999_000_000),
        UFix64::<N9>::new(500_000_000_000_000),
      ),
    ];
    let stats = compute_stats(&input)?;
    // lst 1,000 + stream A 200 hyUSD over 1,000,000 pool; B is stale
    assert_eq!(stats.last_epoch_yield_rate, UFix64::<N9>::new(1_200_000));
    assert!(stats.exo_stats[1].harvest.is_stale);
    let expected_exo = stats.exo_stats[0]
      .projected_inflow
      .checked_add(&stats.exo_stats[1].projected_inflow)
      .ok_or(ProjectedExoInflowOverflow)?;
    assert_eq!(stats.projected_exo_inflow, expected_exo);
    Ok(())
  }

  #[test]
  fn compute_stats_projection_pipeline() -> Result<()> {
    let stats = compute_stats(&inputs())?;
    // LST: $6,750 (Task 3 fixture); borrow: $365.389
    assert_eq!(stats.projected_lst_inflow, UFix64::<N6>::new(6_750_000_000));
    assert_eq!(
      stats.exo_stats[0].projected_inflow,
      UFix64::<N6>::new(365_389_000)
    );
    assert_eq!(stats.projected_exo_inflow, UFix64::<N6>::new(365_389_000));
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
}
