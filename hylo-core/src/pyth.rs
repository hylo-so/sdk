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

pub const SOL_USD_PYTH_FEED: Pubkey =
  pubkey!("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");

#[derive(Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct OracleConfig<Exp> {
  pub interval_secs: u64,
  pub conf_tolerance: UFix64<Exp>,
}

impl<Exp> OracleConfig<Exp> {
  #[must_use]
  pub fn new(
    interval_secs: u64,
    conf_tolerance: UFix64<Exp>,
  ) -> OracleConfig<Exp> {
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
fn validate_conf<Exp>(
  price: UFix64<Exp>,
  conf: UFix64<Exp>,
  tolerance: UFix64<Exp>,
) -> Result<UFix64<Exp>>
where
  UFix64<Exp>: FixExt,
{
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

/// Ensures the `exp` given by Pyth matches the target exponent type.
/// Also checks if the quoted price is negative.
fn validate_price<Exp: Integer>(price: i64, exp: i32) -> Result<UFix64<Exp>> {
  if Exp::to_i32() != exp {
    Err(PythOracleExponent.into())
  } else if price <= 0 {
    Err(PythOracleNegativePrice.into())
  } else {
    Ok(UFix64::new(price.unsigned_abs()))
  }
}

/// Checks Pythnet verification level for the price update.
fn validate_verification_level(level: VerificationLevel) -> Result<()> {
  if level == VerificationLevel::Full {
    Ok(())
  } else {
    Err(PythOracleVerificationLevel.into())
  }
}

/// Fetches price range from a Pyth oracle with a number of validations.
pub fn query_pyth_price<Exp: Integer, C: SolanaClock>(
  clock: &C,
  oracle: &PriceUpdateV2,
  OracleConfig {
    interval_secs,
    conf_tolerance,
  }: OracleConfig<Exp>,
) -> Result<PriceRange<Exp>>
where
  UFix64<Exp>: FixExt,
{
  // Price update validations
  validate_verification_level(oracle.verification_level)?;
  validate_publish_time(
    oracle.price_message.publish_time,
    interval_secs,
    clock.unix_timestamp(),
  )?;
  validate_posted_slot(oracle.posted_slot, interval_secs, clock.slot())?;

  // Build spot range
  let spot_price =
    validate_price(oracle.price_message.price, oracle.price_message.exponent)?;
  let spot_conf = validate_conf(
    spot_price,
    UFix64::new(oracle.price_message.conf),
    conf_tolerance,
  )?;
  PriceRange::from_conf(spot_price, spot_conf)
}

#[cfg(test)]
mod tests {
  use fix::typenum::N8;
  use proptest::prelude::*;

  use super::*;

  const INTERVAL_SECS: u64 = 60;

  proptest! {
    #[test]
    fn validate_price_pos(price in i64::arbitrary()) {
      prop_assume!(price > 0);
      let out = validate_price::<N8>(price, -8)?;
      prop_assert_eq!(out, UFix64::new(price.unsigned_abs()));
    }

    #[test]
    fn validate_price_neg(price in i64::arbitrary(), exp in i32::arbitrary()) {
      prop_assume!(price < 0 || exp != -8);
      let out = validate_price::<N8>(price, exp);
      prop_assert!(out.is_err());
    }

    #[test]
    fn validate_publish_time_neg(
      publish_time in i64::arbitrary(),
      time in i64::arbitrary()
    ) {
      let out = validate_publish_time(publish_time, INTERVAL_SECS, time);
      if publish_time.is_negative() || time.is_negative() {
        prop_assert_eq!(out, Err(PythOracleNegativeTime.into()));
      } else if publish_time.unsigned_abs() + INTERVAL_SECS < time.unsigned_abs() {
        prop_assert_eq!(out, Err(PythOracleOutdated.into()));
      } else {
        prop_assert!(out.is_ok());
      }
    }

    #[allow(clippy::cast_possible_wrap)]
    #[test]
    fn validate_publish_time_pos(
      publish_time in i64::arbitrary(),
      offset in 0..INTERVAL_SECS as i64,
    ) {
      prop_assume!(publish_time.is_positive());
      let out = validate_publish_time(publish_time, INTERVAL_SECS, publish_time + offset);
      prop_assert!(out.is_ok());
    }
  }

  #[test]
  fn slot_interval_precise() {
    // 60 second interval should equate to 150 slots
    let out = slot_interval(60);
    assert_eq!(out, Some(150));
  }

  #[test]
  fn slot_interval_lossy() {
    // 1 second interval should lose half a slot, safer than rounding up
    let out = slot_interval(1);
    assert_eq!(out, Some(2));
  }

  #[test]
  fn validate_confidence_pos() {
    let price = UFix64::<N8>::new(14_640_110_937);
    let conf = UFix64::<N8>::new(9_463_582);
    let tolerance = UFix64::<N8>::new(200_000);
    let out = validate_conf(price, conf, tolerance);
    assert!(out.is_ok());
  }
}
