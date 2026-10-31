use fix::prelude::*;
use fix::typenum::Z0;

use super::{
  AggregationMethod, OracleConfig, PriceAggregateExt, PriceRange,
  ValidationConfig, MICROS_PER_SECOND,
};
use crate::error::CoreError;
use crate::error::CoreError::OracleNegativeTime;
use crate::solana_clock::SolanaClock;

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
pub fn query_price_aggregate<C: SolanaClock, P: PriceAggregateExt>(
  clock: &C,
  price_aggregate: &P,
  OracleConfig {
    interval_secs,
    conf_tolerance,
  }: OracleConfig,
) -> Result<OraclePrice, CoreError> {
  let current_timestamp_us = u64::try_from(clock.unix_timestamp())
    .map_err(|_| OracleNegativeTime)?
    .checked_mul(MICROS_PER_SECOND)
    .ok_or(OracleNegativeTime)?;
  let validation_config = ValidationConfig {
    slots: slot_interval(interval_secs),
    secs: Some(interval_secs),
    confidence: Some(conf_tolerance),
  };
  let observations = price_aggregate.price_observations();
  let observation_configs = price_aggregate.observation_configs();
  let spot = observations.price(
    AggregationMethod::SimpleAverage,
    clock.slot(),
    current_timestamp_us,
    validation_config,
    &observation_configs,
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

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::Clock;

  use super::*;
  use crate::oracle::{
    OracleObservationConfig, PriceAggregateObservations,
    MAX_PRICE_AGGREGATE_OBSERVATIONS,
  };

  struct TestPriceAggregate {
    observations: PriceAggregateObservations,
    observation_configs: Vec<OracleObservationConfig>,
  }

  impl PriceAggregateExt for TestPriceAggregate {
    fn price_observations(&self) -> PriceAggregateObservations {
      self.observations
    }

    fn observation_configs(&self) -> Vec<OracleObservationConfig> {
      self.observation_configs.clone()
    }
  }

  #[test]
  fn query_price_aggregate_uses_caller_validation_config(
  ) -> Result<(), CoreError> {
    let clock = Clock {
      slot: 100,
      unix_timestamp: 10,
      ..Clock::default()
    };
    let observation = super::super::PriceObservation {
      price: 1_000_000_000,
      exponent: -9,
      confidence: 10_000_000,
      posted_slot: 100,
      posted_timestamp_us: 10_000_000,
      market_session: super::super::MarketSession::Regular,
    };
    let observations = PriceAggregateObservations::new(
      [observation; MAX_PRICE_AGGREGATE_OBSERVATIONS],
    );

    let price_aggregate = TestPriceAggregate {
      observations,
      observation_configs: vec![OracleObservationConfig {
        index: 0,
        reference_index: None,
      }],
    };

    let price = query_price_aggregate(
      &clock,
      &price_aggregate,
      OracleConfig::new(60, UFix64::new(10_000_000)),
    )?;

    assert_eq!(price.spot, UFix64::new(1_000_000_000));
    assert_eq!(price.conf, UFix64::zero());
    Ok(())
  }
}
