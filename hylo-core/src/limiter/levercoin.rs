use fix::prelude::*;

use crate::error::CoreError;
use crate::error::CoreError::{
  LevercoinMarketCapArithmetic, LevercoinMarketCapLimitInvalid,
  LevercoinMarketCapLimitReached,
};
use crate::exchange_math::levercoin_market_cap;

// Admissible levercoin market bounds, $1M to $100M.
const MIN_MARKET_CAP_LIMIT: UFix64<N9> =
  UFix64::constant(1_000_000_000_000_000);
const MAX_MARKET_CAP_LIMIT: UFix64<N9> =
  UFix64::constant(100_000_000_000_000_000);

/// Validates levercoin market cap limit against bounds.
///
/// # Errors
/// * Limit outside the admissible range
pub fn validate_levercoin_market_cap_limit(
  limit_raw: UFixValue64,
) -> Result<UFixValue64, CoreError> {
  let limit: UFix64<N9> = limit_raw.try_into()?;
  (MIN_MARKET_CAP_LIMIT..=MAX_MARKET_CAP_LIMIT)
    .contains(&limit)
    .then_some(limit_raw)
    .ok_or(LevercoinMarketCapLimitInvalid)
}

/// Market cap limiter for levercoin mints.
pub struct LevercoinMarketCapLimiter {
  pub market_cap_limit: UFix64<N9>,
  pub levercoin_nav: UFix64<N9>,
  pub levercoin_supply: UFix64<N6>,
}

impl LevercoinMarketCapLimiter {
  #[must_use]
  pub fn new(
    market_cap_limit: UFix64<N9>,
    levercoin_nav: UFix64<N9>,
    levercoin_supply: UFix64<N6>,
  ) -> LevercoinMarketCapLimiter {
    LevercoinMarketCapLimiter {
      market_cap_limit,
      levercoin_nav,
      levercoin_supply,
    }
  }

  fn target_market_cap(
    &self,
    levercoin_to_mint: UFix64<N6>,
  ) -> Result<UFix64<N9>, CoreError> {
    let target_supply = self
      .levercoin_supply
      .checked_add(&levercoin_to_mint)
      .ok_or(LevercoinMarketCapArithmetic)?;
    levercoin_market_cap(target_supply, self.levercoin_nav)
  }

  /// Largest mint keeping the projected market cap within the limit:
  /// inverse of [`levercoin_market_cap`].
  ///
  /// # Errors
  /// * Zero levercoin NAV
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_token_out(&self) -> Result<UFix64<N6>, CoreError> {
    let max_supply = self
      .market_cap_limit
      .div_floor(self.levercoin_nav)
      .and_then(UFix64::checked_convert::<N6>)
      .ok_or(LevercoinMarketCapArithmetic)?;
    let headroom = max_supply.checked_sub(&self.levercoin_supply);
    Ok(headroom.unwrap_or_default())
  }

  /// Checks given mint amount against configured market cap limit.
  ///
  /// # Errors
  /// * Supply overflow
  /// * Projected market cap exceeds the limit
  pub fn validate_token_out(
    &self,
    levercoin_to_mint: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let target_market_cap = self.target_market_cap(levercoin_to_mint)?;
    if target_market_cap <= self.market_cap_limit {
      Ok(levercoin_to_mint)
    } else {
      Err(LevercoinMarketCapLimitReached)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::CoreError::{
    LevercoinMarketCapArithmetic, LevercoinMarketCapLimitInvalid,
    LevercoinMarketCapLimitReached,
  };

  #[test]
  fn validate_limit_boundaries() -> Result<(), CoreError> {
    let min = MIN_MARKET_CAP_LIMIT.into();
    let max = MAX_MARKET_CAP_LIMIT.into();
    let below = UFixValue64::new(MIN_MARKET_CAP_LIMIT.bits - 1, -9);
    let above = UFixValue64::new(MAX_MARKET_CAP_LIMIT.bits + 1, -9);
    assert_eq!(validate_levercoin_market_cap_limit(min)?, min);
    assert_eq!(validate_levercoin_market_cap_limit(max)?, max);
    assert_eq!(
      validate_levercoin_market_cap_limit(below).err(),
      Some(LevercoinMarketCapLimitInvalid)
    );
    assert_eq!(
      validate_levercoin_market_cap_limit(above).err(),
      Some(LevercoinMarketCapLimitInvalid)
    );
    Ok(())
  }

  #[test]
  fn reject_limit_wrong_exp() {
    let raw = UFixValue64::new(10_000_000_000_000, -6);
    assert!(validate_levercoin_market_cap_limit(raw).is_err());
  }

  // $10M limit, $1 NAV, 5M xSOL outstanding, $5M of room
  fn limiter() -> LevercoinMarketCapLimiter {
    LevercoinMarketCapLimiter::new(
      UFix64::new(10_000_000_000_000_000),
      UFix64::new(1_000_000_000),
      UFix64::new(5_000_000_000_000),
    )
  }

  #[test]
  fn accept_mint_under_limit() -> Result<(), CoreError> {
    let to_mint = UFix64::new(1_000_000_000_000);
    assert_eq!(limiter().validate_token_out(to_mint)?, to_mint);
    Ok(())
  }

  #[test]
  fn accept_mint_at_limit() -> Result<(), CoreError> {
    let to_mint = UFix64::new(5_000_000_000_000);
    assert_eq!(limiter().validate_token_out(to_mint)?, to_mint);
    Ok(())
  }

  #[test]
  fn reject_mint_one_unit_over_limit() {
    let to_mint = UFix64::new(5_000_000_000_001);
    assert_eq!(
      limiter().validate_token_out(to_mint).err(),
      Some(LevercoinMarketCapLimitReached)
    );
  }

  #[test]
  fn reject_mint_on_supply_overflow() {
    let to_mint = UFix64::new(u64::MAX);
    assert_eq!(
      limiter().validate_token_out(to_mint).err(),
      Some(LevercoinMarketCapArithmetic)
    );
  }
}
