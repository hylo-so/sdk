use anchor_lang::prelude::*;
use fix::prelude::*;
use fix::typenum::{Integer, Z0};
use pyth_solana_receiver_sdk::price_update::{
  FeedId, PriceUpdateV2, VerificationLevel,
};

use crate::error::CoreError::{
  PythOracleConfidence, PythOracleExponent, PythOracleNegativePrice,
  PythOracleNegativeTime, PythOracleOutdated, PythOraclePriceRange,
  PythOracleSlotInvalid, PythOracleVerificationLevel,
};
use crate::solana_clock::SolanaClock;

pub const SOL_USD: FeedId = [
  239, 13, 139, 111, 218, 44, 235, 164, 29, 161, 93, 64, 149, 209, 218, 57, 42,
  13, 47, 142, 208, 198, 199, 188, 15, 76, 250, 200, 194, 128, 181, 109,
];

pub const BTC_USD: FeedId = [
  230, 45, 246, 200, 180, 168, 95, 225, 166, 125, 180, 77, 193, 45, 229, 219,
  51, 15, 122, 198, 107, 114, 220, 101, 138, 254, 223, 15, 74, 65, 91, 67,
];

pub const SOL_USD_PYTH_FEED: Pubkey =
  pubkey!("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");

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
}

/// Spread of an asset price, with a lower and upper quote.
/// Use lower in minting, higher in redeeming.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PriceRange<Exp: Integer> {
  pub lower: UFix64<Exp>,
  pub upper: UFix64<Exp>,
}

impl<Exp: Integer> PriceRange<Exp> {
  /// Pyth does not publish a "true" price but a range of values defined by a
  /// base price and a confidence interval `(μ-σ, μ+σ)`.
  /// This data type either returns the lower or upper bound of that range.
  /// See [Pyth documentation](https://docs.pyth.network/price-feeds/best-practices#confidence-intervals)
  pub fn from_conf(
    price: UFix64<Exp>,
    conf: UFix64<Exp>,
  ) -> Result<PriceRange<Exp>> {
    let (lower, upper) = price
      .checked_sub(&conf)
      .zip(price.checked_add(&conf))
      .ok_or(PythOraclePriceRange)?;
    Ok(Self::new(lower, upper))
  }

  /// Makes a range of one price, useful in test scenarios.
  #[must_use]
  pub fn one(price: UFix64<Exp>) -> PriceRange<Exp> {
    Self::new(price, price)
  }

  /// Raw construction of range from lower and upper bounds.
  #[must_use]
  pub fn new(lower: UFix64<Exp>, upper: UFix64<Exp>) -> PriceRange<Exp> {
    PriceRange { lower, upper }
  }
}

/// Checks the ratio of `conf / price` against given tolerance.
/// Guards against unusually large spreads in the oracle price.
fn validate_conf(
  price: UFix64<N9>,
  conf: UFix64<N9>,
  tolerance: UFix64<N9>,
) -> Result<UFix64<N9>> {
  conf
    .mul_div_floor(UFix64::one(), price)
    .filter(|diff| diff.le(&tolerance))
    .map(|_| conf)
    .ok_or(PythOracleConfidence.into())
}

/// Ensures the oracle's publish time is within the inclusive range:
///   `[clock_time - oracle_interval, clock_time]`
fn validate_publish_time(
  publish_time: i64,
  oracle_interval: u64,
  clock_time: i64,
) -> Result<()> {
  let (publish_time, clock_time) =
    if publish_time.is_positive() && clock_time.is_positive() {
      Ok((publish_time.unsigned_abs(), clock_time.unsigned_abs()))
    } else {
      Err(PythOracleNegativeTime)
    }?;
  if publish_time.saturating_add(oracle_interval) >= clock_time {
    Ok(())
  } else {
    Err(PythOracleOutdated.into())
  }
}

/// Number of Solana slots in configured oracle interval time.
fn slot_interval(oracle_interval_secs: u64) -> Option<u64> {
  let time: UFix64<N2> = UFix64::<Z0>::new(oracle_interval_secs).convert();
  let slot_time = UFix64::<N2>::new(40); // 400ms slot time
  time.checked_div(&slot_time).map(|i| i.bits)
}

/// Checks the posted slot of a price against the configured oracle interval.
fn validate_posted_slot(
  posted_slot: u64,
  oracle_interval_secs: u64,
  current_slot: u64,
) -> Result<()> {
  current_slot
    .checked_sub(posted_slot)
    .zip(slot_interval(oracle_interval_secs))
    .filter(|(delta, slot_interval)| *delta <= *slot_interval)
    .ok_or(PythOracleSlotInvalid.into())
    .map(|_| ())
}

/// Validates a Pyth price is positive and normalizes to `N9`.
///
/// # Errors
/// * Negative price or unsupported exponent
fn validate_price(price: i64, exp: i32) -> Result<UFix64<N9>> {
  if price <= 0 {
    Err(PythOracleNegativePrice.into())
  } else {
    normalize_pyth_price(price.unsigned_abs(), exp)
  }
}

/// Normalizes a raw Pyth price to canonical `N9` precision.
/// Accepts Pyth exponents from `-2` through `-9`.
///
/// # Errors
/// * Unsupported exponent or conversion overflow
fn normalize_pyth_price(price: u64, exp: i32) -> Result<UFix64<N9>> {
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
  .ok_or(PythOracleExponent.into())
}

/// Checks Pythnet verification level for the price update.
fn validate_verification_level(level: VerificationLevel) -> Result<()> {
  if level == VerificationLevel::Full {
    Ok(())
  } else {
    Err(PythOracleVerificationLevel.into())
  }
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
  pub fn price_range(&self) -> Result<PriceRange<N9>> {
    PriceRange::from_conf(self.spot, self.conf)
  }
}

/// Fetches validated price and confidence from Pyth.
///
/// # Errors
/// * Validation
pub fn query_pyth_oracle<C: SolanaClock>(
  clock: &C,
  oracle: &PriceUpdateV2,
  OracleConfig {
    interval_secs,
    conf_tolerance,
  }: OracleConfig,
) -> Result<OraclePrice> {
  validate_verification_level(oracle.verification_level)?;
  validate_publish_time(
    oracle.price_message.publish_time,
    interval_secs,
    clock.unix_timestamp(),
  )?;
  validate_posted_slot(oracle.posted_slot, interval_secs, clock.slot())?;

  let exp = oracle.price_message.exponent;
  let spot = validate_price(oracle.price_message.price, exp)?;
  let conf = normalize_pyth_price(oracle.price_message.conf, exp)?;
  validate_conf(spot, conf, conf_tolerance)?;
  Ok(OraclePrice { spot, conf })
}

/// Builds price range from Pyth oracle.
///
/// # Errors
/// * Validation
pub fn query_pyth_price<C: SolanaClock>(
  clock: &C,
  oracle: &PriceUpdateV2,
  config: OracleConfig,
) -> Result<PriceRange<N9>> {
  let oracle_price = query_pyth_oracle(clock, oracle, config)?;
  PriceRange::from_conf(oracle_price.spot, oracle_price.conf)
}

#[cfg(test)]
mod tests {
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
      prop_assert!(normalize_pyth_price(price, exp).is_ok());
    }

    #[test]
    fn normalize_unsupported_exp_fails(
      price in 0u64..,
      exp in prop_oneof![-100i32..=-10, -1i32..=100],
    ) {
      prop_assert!(normalize_pyth_price(price, exp).is_err());
    }

    #[test]
    fn normalize_n9_identity(price: u64) {
      let result = normalize_pyth_price(price, -9)?;
      prop_assert_eq!(result.bits, price);
    }

    #[test]
    fn normalize_overflow_fails(
      exp in -8i32..=-2,
    ) {
      let over = pyth_price_max(exp) + 1;
      prop_assert!(normalize_pyth_price(over, exp).is_err());
    }
  }

  #[test]
  fn normalize_n8_known_value() -> Result<()> {
    let result = normalize_pyth_price(14_640_110_937, -8)?;
    assert_eq!(result, UFix64::<N9>::new(146_401_109_370));
    Ok(())
  }

  #[test]
  fn normalize_n9_passthrough() -> Result<()> {
    let result = normalize_pyth_price(123_456_789, -9)?;
    assert_eq!(result, UFix64::<N9>::new(123_456_789));
    Ok(())
  }

  #[test]
  fn normalize_n9_max() -> Result<()> {
    let result = normalize_pyth_price(u64::MAX, -9)?;
    assert_eq!(result, UFix64::<N9>::new(u64::MAX));
    Ok(())
  }

  #[test]
  fn normalize_n2_small() -> Result<()> {
    let result = normalize_pyth_price(14_640, -2)?;
    assert_eq!(result, UFix64::<N9>::new(146_400_000_000));
    Ok(())
  }

  #[test]
  fn normalize_n2_overflow() {
    let over = pyth_price_max(-2) + 1;
    assert!(normalize_pyth_price(over, -2).is_err());
  }

  #[test]
  fn normalize_n8_overflow() {
    let over = pyth_price_max(-8) + 1;
    assert!(normalize_pyth_price(over, -8).is_err());
  }

  #[test]
  fn normalize_unsupported_exponents() {
    assert!(normalize_pyth_price(100, -1).is_err());
    assert!(normalize_pyth_price(100, -10).is_err());
    assert!(normalize_pyth_price(100, -11).is_err());
    assert!(normalize_pyth_price(100, 0).is_err());
    assert!(normalize_pyth_price(100, 5).is_err());
  }

  #[test]
  fn normalize_zero_price() -> Result<()> {
    let result = normalize_pyth_price(0, -8)?;
    assert_eq!(result, UFix64::<N9>::zero());
    Ok(())
  }

  #[test]
  fn validate_conf_within_tolerance() -> Result<()> {
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
  fn validate_conf_exact_boundary() -> Result<()> {
    let price = UFix64::<N9>::new(1_000_000_000);
    let conf = UFix64::<N9>::new(10_000_000);
    let tolerance = UFix64::<N9>::new(10_000_000);
    let result = validate_conf(price, conf, tolerance)?;
    assert_eq!(result, conf);
    Ok(())
  }

  #[test]
  fn validate_conf_zero_passes() -> Result<()> {
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

  #[test]
  fn slot_interval_precise() {
    assert_eq!(slot_interval(60), Some(150));
  }

  #[test]
  fn slot_interval_lossy() {
    assert_eq!(slot_interval(1), Some(2));
  }

  #[test]
  fn slot_interval_zero() {
    assert_eq!(slot_interval(0), Some(0));
  }

  #[test]
  fn slot_interval_large() {
    assert_eq!(slot_interval(3600), Some(9000));
  }

  #[test]
  fn posted_slot_within_interval() {
    assert!(validate_posted_slot(1000, 60, 1100).is_ok());
  }

  #[test]
  fn posted_slot_exact_boundary() {
    assert!(validate_posted_slot(1000, 60, 1150).is_ok());
  }

  #[test]
  fn posted_slot_one_over() {
    assert!(validate_posted_slot(1000, 60, 1151).is_err());
  }

  #[test]
  fn posted_slot_future_fails() {
    assert!(validate_posted_slot(2000, 60, 1000).is_err());
  }

  #[test]
  fn posted_slot_same() {
    assert!(validate_posted_slot(500, 60, 500).is_ok());
  }
}
