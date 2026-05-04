use anchor_lang::prelude::Result;
use fix::prelude::{UFix64, UFixValue64, N9};

use crate::error::CoreError;

const MIN_MARKET_CAP_LIMIT: UFix64<N9> =
  UFix64::constant(1_000_000_000_000_000);
const MAX_MARKET_CAP_LIMIT: UFix64<N9> =
  UFix64::constant(100_000_000_000_000_000);

/// Checks levercoin market cap limit against the range `[MIN, MAX]`.
pub fn validate_levercoin_market_cap_limit(
  limit_raw: UFixValue64,
) -> Result<UFixValue64> {
  let limit: UFix64<N9> = limit_raw.try_into()?;
  (MIN_MARKET_CAP_LIMIT..=MAX_MARKET_CAP_LIMIT)
    .contains(&limit)
    .then_some(limit_raw)
    .ok_or(CoreError::LevercoinMarketCapLimitInvalid.into())
}
