//! Yield statistics math for the earn pool (sHYUSD).
//!
//! Realized yield comes from [`crate::yields::HarvestCache`] snapshots
//! written by the `harvest_yield` and `harvest_borrow_rate` cranks.
//! Projected yield combines the last completed epoch's LST price
//! appreciation with current protocol parameters.

use anchor_lang::prelude::*;
use fix::prelude::*;
use fix::typenum::Z0;

use crate::error::CoreError::{EpochYieldRate, LstEpochGrowth};
use crate::lst::sol_price::LstSolPrice;

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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::lst::sol_price::LstSolPrice;

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
}
