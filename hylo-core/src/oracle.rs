use fix::prelude::*;
use fix::typenum::{Integer, Z0};
use hylo_idl::oracle::accounts::PriceAggregate;
use hylo_idl::oracle::types::MarketSession as IdlMarketSession;
use hylo_oracle_types::{
  AggregationMethod, FeedConfig, MarketSession, PriceAggregateError,
  PriceAggregateSamples, PriceSample, ValidationConfig, MICROS_PER_SECOND,
};

use crate::error::CoreError;
use crate::error::CoreError::{
  OracleConfToleranceInvalid, OracleIntervalSecsInvalid, PythOracleConfidence,
  PythOracleExponent, PythOracleNegativePrice, PythOracleNegativeTime,
  PythOracleOutdated, PythOraclePriceRange,
};
use crate::solana_clock::SolanaClock;

const MIN_INTERVAL_SECS: u64 = 1;
const MAX_INTERVAL_SECS: u64 = 60;
const MIN_CONF_TOLERANCE: UFix64<N9> = UFix64::constant(0);
const MAX_CONF_TOLERANCE: UFix64<N9> = UFix64::constant(50_000_000);

/// Divides oracle secs to a tighter tolerance.
pub const ORACLE_DIVISOR: u64 = 4;

#[derive(Clone, Copy)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceRange<Exp: Integer> {
  pub lower: UFix64<Exp>,
  pub upper: UFix64<Exp>,
}

impl<Exp: Integer> PriceRange<Exp> {
  /// Builds a range around the price using the confidence interval.
  pub fn from_conf(
    price: UFix64<Exp>,
    conf: UFix64<Exp>,
  ) -> Result<PriceRange<Exp>, CoreError> {
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

  #[must_use]
  pub(crate) fn new(lower: UFix64<Exp>, upper: UFix64<Exp>) -> PriceRange<Exp> {
    PriceRange { lower, upper }
  }
}

/// Ensures a timestamp is within the inclusive oracle interval.
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
  if publish_time.saturating_add(oracle_interval) >= clock_time {
    Ok(())
  } else {
    Err(PythOracleOutdated)
  }
}

/// Validated oracle spot price and confidence interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OraclePrice {
  pub spot: UFix64<N9>,
  pub conf: UFix64<N9>,
}

impl OraclePrice {
  /// Builds a price range from validated spot and confidence.
  pub fn price_range(&self) -> Result<PriceRange<N9>, CoreError> {
    PriceRange::from_conf(self.spot, self.conf)
  }
}

/// Fetches the validated aggregate price for one feed.
pub fn query_price_aggregate<C: SolanaClock>(
  clock: &C,
  aggregate: &PriceAggregate,
  feed_config: &FeedConfig,
  OracleConfig {
    interval_secs,
    conf_tolerance,
  }: OracleConfig,
) -> Result<OraclePrice, CoreError> {
  let current_timestamp_us = u64::try_from(clock.unix_timestamp())
    .map_err(|_| PythOracleNegativeTime)?
    .checked_mul(MICROS_PER_SECOND)
    .ok_or(PythOracleNegativeTime)?;
  let validation_config = ValidationConfig {
    slots: slot_interval(interval_secs),
    secs: Some(interval_secs),
    confidence: Some(conf_tolerance),
  };
  let spot = PriceAggregateSamples::new(aggregate.samples.map(|sample| {
    PriceSample {
      price: sample.price,
      exponent: sample.exponent,
      confidence: sample.confidence,
      posted_slot: sample.posted_slot,
      posted_timestamp_us: sample.posted_timestamp_us,
      market_session: match sample.market_session {
        IdlMarketSession::PreMarket => MarketSession::PreMarket,
        IdlMarketSession::Regular => MarketSession::Regular,
        IdlMarketSession::PostMarket => MarketSession::PostMarket,
        IdlMarketSession::Overnight => MarketSession::Overnight,
        IdlMarketSession::Closed => MarketSession::Closed,
      },
    }
  }))
  .price(
    AggregationMethod::SimpleAverage,
    clock.slot(),
    current_timestamp_us,
    validation_config,
    feed_config,
  )?;
  Ok(OraclePrice {
    spot,
    conf: UFix64::new(0),
  })
}

fn slot_interval(oracle_interval_secs: u64) -> Option<u64> {
  let time: UFix64<N2> = UFix64::<Z0>::new(oracle_interval_secs).convert();
  let slot_time = UFix64::<N2>::new(20);
  time.checked_div(&slot_time).map(|interval| interval.bits)
}

impl From<PriceAggregateError> for CoreError {
  fn from(error: PriceAggregateError) -> CoreError {
    match error {
      PriceAggregateError::ConfidenceExceededLimit => PythOracleConfidence,
      PriceAggregateError::ExponentOutOfRange => PythOracleExponent,
      PriceAggregateError::InvalidPrice => PythOracleNegativePrice,
      PriceAggregateError::NoFreshPriceSamples
      | PriceAggregateError::StalePriceSample => PythOracleOutdated,
      PriceAggregateError::ArithmeticOverflow
      | PriceAggregateError::ConversionError
      | PriceAggregateError::InvalidReferenceFeed
      | PriceAggregateError::PriceBandExceeded
      | PriceAggregateError::SampleCapacityExceeded => PythOraclePriceRange,
    }
  }
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::Clock;
  use hylo_idl::oracle::accounts::PriceAggregate;
  use hylo_idl::oracle::types::{
    MarketSession as IdlMarketSession, PriceSample as IdlPriceSample,
  };
  use hylo_oracle_types::FEEDS;

  use super::*;

  #[test]
  fn query_price_aggregate_uses_caller_validation_config(
  ) -> Result<(), CoreError> {
    let clock = Clock {
      slot: 100,
      unix_timestamp: 10,
      ..Clock::default()
    };
    let sample = IdlPriceSample {
      price: 1_000_000_000,
      exponent: -9,
      confidence: 10_000_000,
      posted_slot: 100,
      posted_timestamp_us: 10_000_000,
      market_session: IdlMarketSession::Regular,
    };
    let aggregate = PriceAggregate {
      bump: 0,
      samples: [sample; 8],
      _reserved: [0; 64],
    };

    let price = query_price_aggregate(
      &clock,
      &aggregate,
      &FEEDS[0],
      OracleConfig::new(60, UFix64::new(10_000_000)),
    )?;

    assert_eq!(price.spot, UFix64::new(1_000_000_000));
    assert_eq!(price.conf, UFix64::zero());
    Ok(())
  }
}
