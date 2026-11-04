use fix::prelude::*;

use super::PriceRange;
use crate::error::CoreError;

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

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::Clock;

  use super::*;
  use crate::oracle::{
    OracleConfig, OracleObservationConfig, PriceAggregateExt,
    PriceAggregateObservations, MAX_PRICE_AGGREGATE_OBSERVATIONS,
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

    let price = price_aggregate
      .query_price(&clock, OracleConfig::new(60, UFix64::new(10_000_000)))?;

    assert_eq!(price.spot, UFix64::new(1_000_000_000));
    assert_eq!(price.conf, UFix64::new(10_000_000));
    Ok(())
  }
}
