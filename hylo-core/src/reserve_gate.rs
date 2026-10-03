use fix::prelude::*;

use crate::error::CoreError;

/// Computes the USDC reserve required for a collateral pair.
pub fn required_reserve(
  reserve_per_tvl: UFix64<N6>,
  tvl_cap: UFix64<N6>,
  tvl: UFix64<N6>,
) -> Result<UFix64<N6>, CoreError> {
  if reserve_per_tvl == UFix64::zero() {
    Ok(UFix64::zero())
  } else {
    let capped_pair_size = if tvl_cap == UFix64::zero() {
      tvl
    } else {
      tvl.min(tvl_cap)
    };
    capped_pair_size
      .checked_mul(&reserve_per_tvl)
      .and_then(Fix::checked_convert::<N6>)
      .ok_or(CoreError::InsufficientLiquidity)
  }
}
