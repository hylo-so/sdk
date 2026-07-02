//! Yield statistics math for the earn pool (sHYUSD).
//!
//! Realized yield comes from [`crate::yields::HarvestCache`] snapshots
//! written by the `harvest_yield` and `harvest_borrow_rate` cranks.
//! Projected yield combines the last completed epoch's LST price
//! appreciation with current protocol parameters.

use anchor_lang::prelude::*;
use fix::prelude::*;
use fix::typenum::Z0;

use crate::borrow_rate::BorrowRateConfig;
use crate::error::CoreError::{
  EpochYieldRate, LstEpochGrowth, ProjectedInflow,
};
use crate::lst::sol_price::LstSolPrice;
use crate::yields::YieldHarvestConfig;

/// Solana epochs per year (~2 per day), the protocol's annualization
/// convention (see `borrow_rate` module).
pub const EPOCHS_PER_YEAR: u64 = 182;

/// Per-epoch yield rate: hyUSD deposited into the pool over the pool's
/// hyUSD balance. A zero pool balance yields a zero rate.
///
/// # Errors
/// * Arithmetic overflow during conversion or division
pub fn epoch_yield_rate(
  hyusd_to_pool: UFix64<N6>,
  pool_balance: UFix64<N6>,
) -> Result<UFix64<N9>> {
  if pool_balance == UFix64::zero() {
    Ok(UFix64::zero())
  } else {
    hyusd_to_pool
      .checked_convert::<N9>()
      .zip(pool_balance.checked_convert::<N9>())
      .and_then(|(inflow, pool)| inflow.mul_div_floor(UFix64::one(), pool))
      .ok_or(EpochYieldRate.into())
  }
}

/// The last completed epoch's LST/SOL price appreciation, normalized per
/// epoch: `(price / prev_price - 1) / epoch_gap`.
///
/// This backward-looking growth is the forward estimate for next epoch's
/// yield — the actual next-epoch appreciation is unknowable before the
/// epoch ends. Price regression or a zero epoch gap clamps to zero:
/// harvests never withdraw from the pool.
///
/// # Errors
/// * Invalid price data or arithmetic overflow
pub fn lst_epoch_growth(
  price_sol: &LstSolPrice,
  prev_price_sol: &LstSolPrice,
) -> Result<UFix64<N9>> {
  let prev_price: UFix64<N9> = prev_price_sol.price.try_into()?;
  let epoch_gap = price_sol.epoch.saturating_sub(prev_price_sol.epoch);
  if prev_price == UFix64::zero() || epoch_gap == 0 {
    Ok(UFix64::zero())
  } else {
    // checked_delta errors on regression (cur < prev): clamp to zero.
    price_sol.checked_delta(prev_price_sol).map_or_else(
      |_| Ok(UFix64::zero()),
      |delta| {
        delta
          .mul_div_floor(UFix64::one(), prev_price)
          .and_then(|growth| growth.checked_div(&UFix64::<Z0>::new(epoch_gap)))
          .ok_or(LstEpochGrowth.into())
      },
    )
  }
}

/// Projected hyUSD inflow to the pool from one LST next epoch:
/// vault SOL value x per-epoch growth x SOL/USD spot, through the
/// harvest allocation and treasury fee.
///
/// # Errors
/// * Arithmetic overflow
/// * Invalid harvest config data
pub fn projected_lst_inflow(
  lst_sol_value: UFix64<N9>,
  epoch_growth: UFix64<N9>,
  sol_usd_spot: UFix64<N9>,
  config: &YieldHarvestConfig,
) -> Result<UFix64<N6>> {
  let usd_yield = lst_sol_value
    .mul_div_floor(epoch_growth, UFix64::one())
    .and_then(|sol| sol.mul_div_floor(sol_usd_spot, UFix64::one()))
    .and_then(UFix64::checked_convert::<N6>)
    .ok_or(ProjectedInflow)?;
  let allocated = config.apply_allocation(usd_yield)?;
  let extract = config.apply_fee(allocated)?;
  Ok(extract.amount_remaining)
}

/// Projected hyUSD inflow from the borrow-rate stream next epoch:
/// levercoin market cap x per-epoch rate, minus the treasury fee.
///
/// # Errors
/// * Arithmetic overflow
/// * Invalid borrow rate config data
pub fn projected_borrow_inflow(
  levercoin_market_cap: UFix64<N9>,
  config: &BorrowRateConfig,
) -> Result<UFix64<N6>> {
  let gross = config
    .apply_borrow_rate(levercoin_market_cap, UFix64::constant(1))?
    .checked_convert::<N6>()
    .ok_or(ProjectedInflow)?;
  let extract = config.apply_fee(gross)?;
  Ok(extract.amount_remaining)
}

/// Inflow remaining after repaying outstanding pool drawdown,
/// saturating at zero. Harvests repay bad debt before hyUSD reaches
/// the pool.
#[must_use]
pub fn apply_drawdown_offset(
  inflow: UFix64<N6>,
  outstanding_drawdown: UFix64<N6>,
) -> UFix64<N6> {
  inflow
    .checked_sub(&outstanding_drawdown)
    .unwrap_or_else(UFix64::zero)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::borrow_rate::BorrowRateConfig;
  use crate::lst::sol_price::LstSolPrice;
  use crate::yields::YieldHarvestConfig;

  #[test]
  fn epoch_yield_rate_basic() -> Result<()> {
    // 1,000 hyUSD into a 1,000,000 hyUSD pool = 0.1% per epoch
    let rate = epoch_yield_rate(
      UFix64::<N6>::new(1_000_000_000),
      UFix64::<N6>::new(1_000_000_000_000),
    )?;
    assert_eq!(rate, UFix64::<N9>::new(1_000_000));
    Ok(())
  }

  #[test]
  fn epoch_yield_rate_zero_pool() -> Result<()> {
    let rate = epoch_yield_rate(UFix64::<N6>::new(5_000_000), UFix64::zero())?;
    assert_eq!(rate, UFix64::zero());
    Ok(())
  }

  #[test]
  fn epoch_yield_rate_zero_inflow() -> Result<()> {
    let rate =
      epoch_yield_rate(UFix64::zero(), UFix64::<N6>::new(1_000_000_000_000))?;
    assert_eq!(rate, UFix64::zero());
    Ok(())
  }

  fn price(bits: u64, epoch: u64) -> LstSolPrice {
    LstSolPrice::new(UFix64::<N9>::new(bits).into(), epoch)
  }

  #[test]
  fn lst_epoch_growth_one_epoch() -> Result<()> {
    // 1.0000 -> 1.0005 over one epoch = 0.05% growth
    let prev = price(1_000_000_000, 100);
    let cur = price(1_000_500_000, 101);
    assert_eq!(lst_epoch_growth(&cur, &prev)?, UFix64::new(500_000));
    Ok(())
  }

  #[test]
  fn lst_epoch_growth_two_epoch_gap_normalizes() -> Result<()> {
    // Same appreciation over two epochs = half the per-epoch growth
    let prev = price(1_000_000_000, 100);
    let cur = price(1_000_500_000, 102);
    assert_eq!(lst_epoch_growth(&cur, &prev)?, UFix64::new(250_000));
    Ok(())
  }

  #[test]
  fn lst_epoch_growth_regression_is_zero() -> Result<()> {
    let prev = price(1_000_500_000, 100);
    let cur = price(1_000_000_000, 101);
    assert_eq!(lst_epoch_growth(&cur, &prev)?, UFix64::zero());
    Ok(())
  }

  #[test]
  fn lst_epoch_growth_same_epoch_is_zero() -> Result<()> {
    let prev = price(1_000_000_000, 100);
    let cur = price(1_000_500_000, 100);
    assert_eq!(lst_epoch_growth(&cur, &prev)?, UFix64::zero());
    Ok(())
  }

  fn harvest_config(allocation_bps: u64, fee_bps: u64) -> YieldHarvestConfig {
    YieldHarvestConfig {
      allocation: UFix64::<N4>::new(allocation_bps).into(),
      fee: UFix64::<N4>::new(fee_bps).into(),
    }
  }

  #[test]
  fn projected_lst_inflow_full_allocation() -> Result<()> {
    // 100,000 SOL at 0.05%/epoch growth, SOL at $150:
    // 50 SOL * 150 = $7,500; 100% allocation, 10% fee -> $6,750
    let inflow = projected_lst_inflow(
      UFix64::<N9>::new(100_000_000_000_000),
      UFix64::<N9>::new(500_000),
      UFix64::<N9>::new(150_000_000_000),
      &harvest_config(10_000, 1_000),
    )?;
    assert_eq!(inflow, UFix64::<N6>::new(6_750_000_000));
    Ok(())
  }

  #[test]
  fn projected_lst_inflow_partial_allocation() -> Result<()> {
    // Same yield, 80% allocation, 10% fee -> 7,500 * 0.8 * 0.9 = $5,400
    let inflow = projected_lst_inflow(
      UFix64::<N9>::new(100_000_000_000_000),
      UFix64::<N9>::new(500_000),
      UFix64::<N9>::new(150_000_000_000),
      &harvest_config(8_000, 1_000),
    )?;
    assert_eq!(inflow, UFix64::<N6>::new(5_400_000_000));
    Ok(())
  }

  #[test]
  fn projected_lst_inflow_zero_growth() -> Result<()> {
    let inflow = projected_lst_inflow(
      UFix64::<N9>::new(100_000_000_000_000),
      UFix64::zero(),
      UFix64::<N9>::new(150_000_000_000),
      &harvest_config(10_000, 1_000),
    )?;
    assert_eq!(inflow, UFix64::zero());
    Ok(())
  }

  #[test]
  fn projected_borrow_inflow_basic() -> Result<()> {
    // $1,000,000 market cap at 384,620e-9 per epoch, 5% fee:
    // $384.62 gross -> $365.389 to pool
    let config = BorrowRateConfig::new(
      UFix64::<N9>::new(384_620).into(),
      UFix64::<N4>::new(500).into(),
    );
    let inflow = projected_borrow_inflow(
      UFix64::<N9>::new(1_000_000_000_000_000),
      &config,
    )?;
    assert_eq!(inflow, UFix64::<N6>::new(365_389_000));
    Ok(())
  }

  #[test]
  fn drawdown_offset_partial() {
    let net = apply_drawdown_offset(
      UFix64::<N6>::new(100_000_000),
      UFix64::<N6>::new(30_000_000),
    );
    assert_eq!(net, UFix64::new(70_000_000));
  }

  #[test]
  fn drawdown_offset_exceeds_inflow_clamps_to_zero() {
    let net = apply_drawdown_offset(
      UFix64::<N6>::new(30_000_000),
      UFix64::<N6>::new(100_000_000),
    );
    assert_eq!(net, UFix64::zero());
  }
}
