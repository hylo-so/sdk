use fix::prelude::*;

use crate::error::CoreError;

/// Computes the USDC reserve required for a collateral pair.
pub fn required_reserve(
  reserve_ratio: UFix64<N6>,
  pair_size_cap_usd: UFix64<N6>,
  pair_size_usd: UFix64<N6>,
) -> Result<UFix64<N6>, CoreError> {
  if reserve_ratio == UFix64::zero() {
    Ok(UFix64::zero())
  } else {
    let capped_pair_size = if pair_size_cap_usd == UFix64::zero() {
      pair_size_usd
    } else {
      pair_size_usd.min(pair_size_cap_usd)
    };
    capped_pair_size
      .checked_mul(&reserve_ratio)
      .and_then(|reserve| reserve.checked_convert::<N6>())
      .ok_or(CoreError::InsufficientLiquidity)
  }
}
