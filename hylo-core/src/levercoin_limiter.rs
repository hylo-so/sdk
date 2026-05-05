//! Supply-side guardrail capping levercoin (xSOL / xAsset) issuance by
//! USD market cap.
//!
//! Bounds protocol risk in dollar terms rather than token units, so NAV
//! swings cannot expand exposure beyond the configured ceiling. Admin
//! instructions set the limit via [`validate_levercoin_market_cap_limit`];
//! mint instructions enforce it via
//! [`LevercoinMarketCapLimiter::validate_token_out`].

use anchor_lang::prelude::Result;
use fix::prelude::*;

use crate::error::CoreError::{
  LevercoinMarketCapArithmetic, LevercoinMarketCapLimitInvalid,
  LevercoinMarketCapLimitReached,
};
use crate::exchange_math::levercoin_market_cap;

// Admissible levercoin market cap limit range, $1M to $100M.
const MIN_MARKET_CAP_LIMIT: UFix64<N9> =
  UFix64::constant(1_000_000_000_000_000);
const MAX_MARKET_CAP_LIMIT: UFix64<N9> =
  UFix64::constant(100_000_000_000_000_000);

/// Bounds an admin-supplied market cap to the protocol-approved range
/// (`$1M..=$100M`) before it is persisted on-chain.
///
/// Prevents a misconfigured admin from effectively disabling the
/// guardrail (cap set too high) or stalling mints (cap set absurdly
/// low). Returns the raw value unchanged so callers can serialize it
/// directly without re-encoding.
pub fn validate_levercoin_market_cap_limit(
  limit_raw: UFixValue64,
) -> Result<UFixValue64> {
  let limit: UFix64<N9> = limit_raw.try_into()?;
  (MIN_MARKET_CAP_LIMIT..=MAX_MARKET_CAP_LIMIT)
    .contains(&limit)
    .then_some(limit_raw)
    .ok_or(LevercoinMarketCapLimitInvalid.into())
}

/// Snapshot of the inputs needed to evaluate the cap at a single point
/// in time: the configured USD ceiling, the current levercoin NAV, and
/// outstanding supply. Built fresh per mint instruction.
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
  ) -> Result<UFix64<N9>> {
    let target_supply = self
      .levercoin_supply
      .checked_add(&levercoin_to_mint)
      .ok_or(LevercoinMarketCapArithmetic)?;
    levercoin_market_cap(target_supply, self.levercoin_nav)
  }

  /// Mint-time gate: projects market cap after the mint and admits the
  /// issuance only if the result stays at or below the configured limit.
  ///
  /// Returns the requested amount unchanged on success so the call site
  /// can chain validation into the mint pipeline without rebinding.
  pub fn validate_token_out(
    &self,
    levercoin_to_mint: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    let target_market_cap = self.target_market_cap(levercoin_to_mint)?;
    if target_market_cap <= self.market_cap_limit {
      Ok(levercoin_to_mint)
    } else {
      Err(LevercoinMarketCapLimitReached.into())
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
  fn validate_limit_boundaries() -> Result<()> {
    let min = MIN_MARKET_CAP_LIMIT.into();
    let max = MAX_MARKET_CAP_LIMIT.into();
    let below = UFixValue64::new(MIN_MARKET_CAP_LIMIT.bits - 1, -9);
    let above = UFixValue64::new(MAX_MARKET_CAP_LIMIT.bits + 1, -9);
    assert_eq!(validate_levercoin_market_cap_limit(min)?, min);
    assert_eq!(validate_levercoin_market_cap_limit(max)?, max);
    assert_eq!(
      validate_levercoin_market_cap_limit(below).err(),
      Some(LevercoinMarketCapLimitInvalid.into())
    );
    assert_eq!(
      validate_levercoin_market_cap_limit(above).err(),
      Some(LevercoinMarketCapLimitInvalid.into())
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
  fn accept_mint_under_limit() -> Result<()> {
    let to_mint = UFix64::new(1_000_000_000_000);
    assert_eq!(limiter().validate_token_out(to_mint)?, to_mint);
    Ok(())
  }

  #[test]
  fn accept_mint_at_limit() -> Result<()> {
    let to_mint = UFix64::new(5_000_000_000_000);
    assert_eq!(limiter().validate_token_out(to_mint)?, to_mint);
    Ok(())
  }

  #[test]
  fn reject_mint_one_unit_over_limit() {
    let to_mint = UFix64::new(5_000_000_000_001);
    assert_eq!(
      limiter().validate_token_out(to_mint).err(),
      Some(LevercoinMarketCapLimitReached.into())
    );
  }

  #[test]
  fn reject_mint_on_supply_overflow() {
    let to_mint = UFix64::new(u64::MAX);
    assert_eq!(
      limiter().validate_token_out(to_mint).err(),
      Some(LevercoinMarketCapArithmetic.into())
    );
  }
}
