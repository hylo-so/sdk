//! Source-neutral observations alongside the existing Pyth read path.
//! Callers must authenticate the account owner and canonical feed PDA before
//! reading an observation. This module only validates its price and freshness.
use crate::error::CoreError;
use crate::error::CoreError::{PythOracleNegativeTime, PythOracleOutdated};
use crate::pyth::{normalize_pyth_price, validate_conf, validate_price};
pub use crate::pyth::{OracleConfig, OraclePrice, PriceRange};
use crate::solana_clock::SolanaClock;

/// Ensures the oracle's publish time is within the freshness window
/// `[clock_time - oracle_interval, clock_time + oracle_interval]`: not stale,
/// and not implausibly future-dated. The producer bounds write-time skew; the
/// upper bound is defense-in-depth against a grossly-future timestamp that
/// would otherwise suppress staleness indefinitely.
pub fn validate_publish_time(
  publish_time: i64,
  oracle_interval: u64,
  clock_time: i64,
) -> Result<(), CoreError> {
  let (publish_time, clock_time) =
    if publish_time.is_positive() && clock_time.is_positive() {
      Ok((publish_time.unsigned_abs(), clock_time.unsigned_abs()))
    } else {
      Err(PythOracleNegativeTime)
    }?;
  let not_stale = publish_time.saturating_add(oracle_interval) >= clock_time;
  let not_future = publish_time <= clock_time.saturating_add(oracle_interval);
  if not_stale && not_future {
    Ok(())
  } else {
    Err(PythOracleOutdated)
  }
}

/// Fetches validated price and confidence from a hylo-oracle observation.
/// Source-neutral: verification/authentication happened at write time, so this
/// does NOT re-check verification level, and it gates freshness on the
/// price-determination time (`price_timestamp_us`), not the crank write slot.
pub fn query_hylo_oracle<C: SolanaClock>(
  clock: &C,
  observation: &hylo_oracle_types::OracleObservation,
  OracleConfig {
    interval_secs,
    conf_tolerance,
  }: OracleConfig,
) -> Result<OraclePrice, CoreError> {
  let publish_time_secs = i64::try_from(
    observation.price_timestamp_us / hylo_oracle_types::MICROS_PER_SECOND,
  )
  .map_err(|_| PythOracleNegativeTime)?;
  validate_publish_time(
    publish_time_secs,
    interval_secs,
    clock.unix_timestamp(),
  )?;
  let exp = i32::from(observation.exponent);
  let spot = validate_price(observation.price, exp)?;
  let conf = normalize_pyth_price(observation.confidence, exp)?;
  validate_conf(spot, conf, conf_tolerance)?;
  Ok(OraclePrice { spot, conf })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::CoreError::PythOracleConfidence;
  use anchor_lang::prelude::Clock;
  use fix::prelude::*;
  const NOW_SECS: i64 = 2_000_000;
  const INTERVAL_SECS: i64 = 60;

  fn test_clock(unix_timestamp: i64) -> Clock {
    Clock {
      unix_timestamp,
      ..Clock::default()
    }
  }

  fn hylo_config() -> OracleConfig {
    OracleConfig::new(
      INTERVAL_SECS.unsigned_abs(),
      UFix64::<N9>::new(1_000_000),
    )
  }

  fn secs_to_us(secs: i64) -> u64 {
    secs.unsigned_abs() * hylo_oracle_types::MICROS_PER_SECOND
  }

  // `OracleObservation._reserved` is private, so the struct-literal form clippy
  // would suggest won't compile here; build via `Default` + field assignment.
  #[allow(clippy::field_reassign_with_default)]
  fn observation(
    price: i64,
    exponent: i16,
    confidence: u64,
    price_timestamp_us: u64,
  ) -> hylo_oracle_types::OracleObservation {
    let mut obs = hylo_oracle_types::OracleObservation::default();
    obs.price = price;
    obs.exponent = exponent;
    obs.confidence = confidence;
    obs.price_timestamp_us = price_timestamp_us;
    obs
  }

  #[test]
  fn hylo_oracle_valid_observation() {
    let clock = test_clock(NOW_SECS);
    let obs = observation(14_640_110_937, -8, 8_000_000, secs_to_us(NOW_SECS));
    assert_eq!(
      query_hylo_oracle(&clock, &obs, hylo_config()),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(146_401_109_370),
        conf: UFix64::<N9>::new(80_000_000),
      })
    );
  }

  #[test]
  fn hylo_oracle_stale_observation() {
    let clock = test_clock(NOW_SECS);
    let obs = observation(
      14_640_110_937,
      -8,
      8_000_000,
      secs_to_us(NOW_SECS - INTERVAL_SECS - 1),
    );
    assert_eq!(
      query_hylo_oracle(&clock, &obs, hylo_config()),
      Err(PythOracleOutdated)
    );
  }

  #[test]
  fn hylo_oracle_micros_to_secs_boundary() {
    let clock = test_clock(NOW_SECS);
    let almost_a_second = hylo_oracle_types::MICROS_PER_SECOND - 1;
    // Exactly `interval` secs old; sub-second µs floor away, so still fresh.
    let at_edge = observation(
      14_640_110_937,
      -8,
      8_000_000,
      secs_to_us(NOW_SECS - INTERVAL_SECS) + almost_a_second,
    );
    assert!(query_hylo_oracle(&clock, &at_edge, hylo_config()).is_ok());
    // One whole second older floors below the window, so stale.
    let past_edge = observation(
      14_640_110_937,
      -8,
      8_000_000,
      secs_to_us(NOW_SECS - INTERVAL_SECS - 1),
    );
    assert_eq!(
      query_hylo_oracle(&clock, &past_edge, hylo_config()),
      Err(PythOracleOutdated)
    );
  }

  #[test]
  fn hylo_oracle_confidence_too_wide() {
    let clock = test_clock(NOW_SECS);
    let obs =
      observation(14_640_110_937, -8, 2_000_000_000, secs_to_us(NOW_SECS));
    assert_eq!(
      query_hylo_oracle(&clock, &obs, hylo_config()),
      Err(PythOracleConfidence)
    );
  }

  #[test]
  fn hylo_oracle_exponent_range_normalizes() {
    let clock = test_clock(NOW_SECS);
    let n2 = observation(14_640, -2, 0, secs_to_us(NOW_SECS));
    assert_eq!(
      query_hylo_oracle(&clock, &n2, hylo_config()),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(146_400_000_000),
        conf: UFix64::<N9>::zero(),
      })
    );
    let n9 = observation(123_456_789, -9, 0, secs_to_us(NOW_SECS));
    assert_eq!(
      query_hylo_oracle(&clock, &n9, hylo_config()),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(123_456_789),
        conf: UFix64::<N9>::zero(),
      })
    );
  }
  #[test]
  fn future_price_is_bounded_even_after_reposting() {
    let mut obs =
      observation(100_000_000, -8, 0, secs_to_us(NOW_SECS + INTERVAL_SECS + 1));
    obs.posted_unix_timestamp = NOW_SECS;
    assert_eq!(
      query_hylo_oracle(&test_clock(NOW_SECS), &obs, hylo_config()),
      Err(PythOracleOutdated)
    );
  }

  #[test]
  fn both_sources_share_main_price_range_math() {
    use pyth_solana_receiver_sdk::price_update::{
      PriceUpdateV2, VerificationLevel,
    };
    let obs = observation(14_640_110_937, -8, 8_000_000, secs_to_us(NOW_SECS));
    let pyth = PriceUpdateV2 {
      write_authority: Default::default(),
      verification_level: VerificationLevel::Full,
      price_message: pyth_solana_receiver_sdk::price_update::PriceFeedMessage {
        feed_id: Default::default(),
        price: obs.price,
        conf: obs.confidence,
        exponent: i32::from(obs.exponent),
        publish_time: NOW_SECS,
        prev_publish_time: NOW_SECS - 1,
        ema_price: obs.price,
        ema_conf: obs.confidence,
      },
      posted_slot: 0,
    };
    let clock = test_clock(NOW_SECS);
    assert_eq!(
      query_hylo_oracle(&clock, &obs, hylo_config()),
      crate::pyth::query_pyth_oracle(&clock, &pyth, hylo_config())
    );
  }
}
