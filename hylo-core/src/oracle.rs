use fix::prelude::*;
use fix::typenum::Integer;

use crate::error::CoreError;
use crate::error::CoreError::{
  OracleConfToleranceInvalid, OracleIntervalSecsInvalid, OracleConfidence,
  OracleExponent, OracleNegativePrice, OracleNegativeTime,
  OracleOutdated, OraclePriceRange,
};
use crate::solana_clock::SolanaClock;

const MIN_INTERVAL_SECS: u64 = 1;
const MAX_INTERVAL_SECS: u64 = 60;
const MIN_CONF_TOLERANCE: UFix64<N9> = UFix64::constant(0);
const MAX_CONF_TOLERANCE: UFix64<N9> = UFix64::constant(50_000_000);

/// Divides oracle secs to a tighter tolerance.
pub const ORACLE_DIVISOR: u64 = 4;

#[derive(Copy, Clone)]
pub struct OracleConfig {
  pub interval_secs: u64,
  pub conf_tolerance: UFix64<N9>,
}

impl OracleConfig {
  #[must_use]
  pub fn new(interval_secs: u64, conf_tolerance: UFix64<N9>) -> OracleConfig {
    OracleConfig {
      interval_secs,
      conf_tolerance,
    }
  }

  #[must_use]
  pub fn for_stablecoin(self) -> OracleConfig {
    OracleConfig {
      interval_secs: self.interval_secs.div_ceil(ORACLE_DIVISOR),
      ..self
    }
  }
}

/// Oracle interval must be in `[MIN, MAX]`.
pub fn validate_interval_secs(secs: u64) -> Result<u64, CoreError> {
  if (MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&secs) {
    Ok(secs)
  } else {
    Err(OracleIntervalSecsInvalid)
  }
}

/// Confidence tolerance must be in `[MIN, MAX]`.
pub fn validate_conf_tolerance(
  tolerance: UFixValue64,
) -> Result<UFixValue64, CoreError> {
  let pct: UFix64<N9> = tolerance.try_into()?;
  if (MIN_CONF_TOLERANCE..=MAX_CONF_TOLERANCE).contains(&pct) {
    Ok(tolerance)
  } else {
    Err(OracleConfToleranceInvalid)
  }
}

/// Spread of an asset price, with a lower and upper quote.
/// Use lower in minting, higher in redeeming.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PriceRange<Exp: Integer> {
  pub lower: UFix64<Exp>,
  pub upper: UFix64<Exp>,
}

impl<Exp: Integer> PriceRange<Exp> {
  /// The oracle publishes a spot price with a confidence interval
  /// `(μ-σ, μ+σ)` rather than a single "true" price; this returns the lower or
  /// upper bound of that range. See [Pyth's confidence-interval guidance](https://docs.pyth.network/price-feeds/best-practices#confidence-intervals)
  pub fn from_conf(
    price: UFix64<Exp>,
    conf: UFix64<Exp>,
  ) -> Result<PriceRange<Exp>, CoreError> {
    let (lower, upper) = price
      .checked_sub(&conf)
      .zip(price.checked_add(&conf))
      .ok_or(OraclePriceRange)?;
    Ok(Self::new(lower, upper))
  }

  /// Makes a range of one price, useful in test scenarios.
  #[must_use]
  pub fn one(price: UFix64<Exp>) -> PriceRange<Exp> {
    Self::new(price, price)
  }

  #[must_use]
  pub(crate) fn new(lower: UFix64<Exp>, upper: UFix64<Exp>) -> PriceRange<Exp> {
    PriceRange { lower, upper }
  }
}

/// Checks the ratio of `conf / price` against given tolerance.
/// Guards against unusually large spreads in the oracle price.
fn validate_conf(
  price: UFix64<N9>,
  conf: UFix64<N9>,
  tolerance: UFix64<N9>,
) -> Result<UFix64<N9>, CoreError> {
  conf
    .mul_div_floor(UFix64::one(), price)
    .filter(|diff| diff.le(&tolerance))
    .map(|_| conf)
    .ok_or(OracleConfidence)
}

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
      Err(OracleNegativeTime)
    }?;
  let not_stale = publish_time.saturating_add(oracle_interval) >= clock_time;
  let not_future =
    publish_time <= clock_time.saturating_add(oracle_interval);
  if not_stale && not_future {
    Ok(())
  } else {
    Err(OracleOutdated)
  }
}

/// Validates an oracle price is positive and normalizes to `N9`.
///
/// # Errors
/// * Negative price or unsupported exponent
fn validate_price(price: i64, exp: i32) -> Result<UFix64<N9>, CoreError> {
  if price <= 0 {
    Err(OracleNegativePrice)
  } else {
    normalize_price(price.unsigned_abs(), exp)
  }
}

/// Normalizes a raw Pyth price to canonical `N9` precision.
/// Accepts Pyth exponents from `-2` through `-9`.
///
/// # Errors
/// * Unsupported exponent or conversion overflow
fn normalize_price(price: u64, exp: i32) -> Result<UFix64<N9>, CoreError> {
  match exp {
    -2 => UFix64::<N2>::new(price).checked_convert(),
    -3 => UFix64::<N3>::new(price).checked_convert(),
    -4 => UFix64::<N4>::new(price).checked_convert(),
    -5 => UFix64::<N5>::new(price).checked_convert(),
    -6 => UFix64::<N6>::new(price).checked_convert(),
    -7 => UFix64::<N7>::new(price).checked_convert(),
    -8 => UFix64::<N8>::new(price).checked_convert(),
    -9 => Some(UFix64::<N9>::new(price)),
    _ => None,
  }
  .ok_or(OracleExponent)
}

/// Validated oracle spot price and confidence interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OraclePrice {
  pub spot: UFix64<N9>,
  pub conf: UFix64<N9>,
}

impl OraclePrice {
  /// Builds a price range from validated spot and confidence.
  ///
  /// # Errors
  /// * Arithmetic overflow from `PriceRange::from_conf`
  pub fn price_range(&self) -> Result<PriceRange<N9>, CoreError> {
    PriceRange::from_conf(self.spot, self.conf)
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
  .map_err(|_| OracleNegativeTime)?;
  validate_publish_time(
    publish_time_secs,
    interval_secs,
    clock.unix_timestamp(),
  )?;
  let exp = i32::from(observation.exponent);
  let spot = validate_price(observation.price, exp)?;
  let conf = normalize_price(observation.confidence, exp)?;
  validate_conf(spot, conf, conf_tolerance)?;
  Ok(OraclePrice { spot, conf })
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::Clock;
  use fix::prelude::*;
  use proptest::prelude::*;

  use super::*;

  /// Max safe raw price bits for a given exponent before N9 overflow.
  /// `u64::MAX / 10^(9 - |exp|)`
  fn pyth_price_max(exp: i32) -> u64 {
    u64::MAX / 10u64.pow(9 - exp.unsigned_abs())
  }

  /// Supported Pyth exponent (-9 through -2).
  fn pyth_exponent() -> BoxedStrategy<i32> {
    (-9i32..=-2).boxed()
  }

  /// Raw Pyth price and exponent pair safe for N9 conversion.
  fn pyth_price() -> BoxedStrategy<(u64, i32)> {
    pyth_exponent()
      .prop_flat_map(|exp| (1u64..=pyth_price_max(exp), Just(exp)))
      .boxed()
  }

  proptest! {
    #[test]
    fn normalize_safe_price_succeeds(
      (price, exp) in pyth_price(),
    ) {
      prop_assert!(normalize_price(price, exp).is_ok());
    }

    #[test]
    fn normalize_unsupported_exp_fails(
      price in 0u64..,
      exp in prop_oneof![-100i32..=-10, -1i32..=100],
    ) {
      prop_assert!(normalize_price(price, exp).is_err());
    }

    #[test]
    fn normalize_n9_identity(price: u64) {
      let result = normalize_price(price, -9)?;
      prop_assert_eq!(result.bits, price);
    }

    #[test]
    fn normalize_overflow_fails(
      exp in -8i32..=-2,
    ) {
      let over = pyth_price_max(exp) + 1;
      prop_assert!(normalize_price(over, exp).is_err());
    }
  }

  #[test]
  fn normalize_n8_known_value() -> Result<(), CoreError> {
    let result = normalize_price(14_640_110_937, -8)?;
    assert_eq!(result, UFix64::<N9>::new(146_401_109_370));
    Ok(())
  }

  #[test]
  fn normalize_n9_passthrough() -> Result<(), CoreError> {
    let result = normalize_price(123_456_789, -9)?;
    assert_eq!(result, UFix64::<N9>::new(123_456_789));
    Ok(())
  }

  #[test]
  fn normalize_n9_max() -> Result<(), CoreError> {
    let result = normalize_price(u64::MAX, -9)?;
    assert_eq!(result, UFix64::<N9>::new(u64::MAX));
    Ok(())
  }

  #[test]
  fn normalize_n2_small() -> Result<(), CoreError> {
    let result = normalize_price(14_640, -2)?;
    assert_eq!(result, UFix64::<N9>::new(146_400_000_000));
    Ok(())
  }

  #[test]
  fn normalize_n2_overflow() {
    let over = pyth_price_max(-2) + 1;
    assert!(normalize_price(over, -2).is_err());
  }

  #[test]
  fn normalize_n8_overflow() {
    let over = pyth_price_max(-8) + 1;
    assert!(normalize_price(over, -8).is_err());
  }

  #[test]
  fn normalize_unsupported_exponents() {
    assert!(normalize_price(100, -1).is_err());
    assert!(normalize_price(100, -10).is_err());
    assert!(normalize_price(100, -11).is_err());
    assert!(normalize_price(100, 0).is_err());
    assert!(normalize_price(100, 5).is_err());
  }

  #[test]
  fn normalize_zero_price() -> Result<(), CoreError> {
    let result = normalize_price(0, -8)?;
    assert_eq!(result, UFix64::<N9>::zero());
    Ok(())
  }

  #[test]
  fn validate_conf_within_tolerance() -> Result<(), CoreError> {
    let price = UFix64::<N9>::new(146_401_109_370);
    let conf = UFix64::<N9>::new(80_000_000);
    let tolerance = UFix64::<N9>::new(1_000_000);
    let result = validate_conf(price, conf, tolerance)?;
    assert_eq!(result, conf);
    Ok(())
  }

  #[test]
  fn validate_conf_exceeds_tolerance() {
    let price = UFix64::<N9>::new(146_401_109_370);
    let conf = UFix64::<N9>::new(2_000_000_000);
    let tolerance = UFix64::<N9>::new(1_000_000);
    assert!(validate_conf(price, conf, tolerance).is_err());
  }

  #[test]
  fn validate_conf_exact_boundary() -> Result<(), CoreError> {
    let price = UFix64::<N9>::new(1_000_000_000);
    let conf = UFix64::<N9>::new(10_000_000);
    let tolerance = UFix64::<N9>::new(10_000_000);
    let result = validate_conf(price, conf, tolerance)?;
    assert_eq!(result, conf);
    Ok(())
  }

  #[test]
  fn validate_conf_zero_passes() -> Result<(), CoreError> {
    let price = UFix64::<N9>::new(100_000_000_000);
    let conf = UFix64::<N9>::zero();
    let tolerance = UFix64::<N9>::new(1_000_000);
    let result = validate_conf(price, conf, tolerance)?;
    assert_eq!(result, conf);
    Ok(())
  }

  #[test]
  fn publish_time_exact_boundary() {
    assert!(validate_publish_time(100, 60, 160).is_ok());
  }

  #[test]
  fn publish_time_just_expired() {
    assert!(validate_publish_time(100, 60, 161).is_err());
  }

  #[test]
  fn publish_time_large_interval() {
    assert!(validate_publish_time(1000, 3600, 4500).is_ok());
  }

  #[test]
  fn publish_time_zero_interval() {
    assert!(validate_publish_time(100, 0, 100).is_ok());
    assert!(validate_publish_time(100, 0, 101).is_err());
  }

  #[test]
  fn publish_time_negative_publish() {
    assert!(validate_publish_time(-1, 120, 100).is_err());
  }

  #[test]
  fn publish_time_negative_clock() {
    assert!(validate_publish_time(100, 30, -1).is_err());
  }

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
      Err(OracleOutdated)
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
      Err(OracleOutdated)
    );
  }

  #[test]
  fn hylo_oracle_confidence_too_wide() {
    let clock = test_clock(NOW_SECS);
    let obs =
      observation(14_640_110_937, -8, 2_000_000_000, secs_to_us(NOW_SECS));
    assert_eq!(
      query_hylo_oracle(&clock, &obs, hylo_config()),
      Err(OracleConfidence)
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
}
