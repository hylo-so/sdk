//! Yield statistics math for the earn pool (sHYUSD).
//!
//! Realized yield comes from [`crate::yields::HarvestCache`] snapshots
//! written by the `harvest_yield` and `harvest_borrow_rate` cranks.
//! Projected yield combines the last completed epoch's LST price
//! appreciation with current protocol parameters.

use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::EpochYieldRate;

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

#[cfg(test)]
mod tests {
  use super::*;

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
}
