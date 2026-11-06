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
    MarketSession, OracleConfig, PriceAggregateExt, PriceObservation,
    PythLazerFeedId, PythLazerSlot, PythLazerSlots,
  };

  struct TestPriceAggregate(PythLazerSlots);

  impl PriceAggregateExt for TestPriceAggregate {
    fn pyth_lazer_slots(&self) -> PythLazerSlots {
      self.0
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
    let observation = PriceObservation {
      price: 1_000_000_000,
      exponent: -9,
      confidence: 10_000_000,
      posted_slot: 100,
      posted_timestamp_us: 10_000_000,
      market_session: MarketSession::Regular,
    };
    let price_aggregate = TestPriceAggregate(PythLazerSlots([
      Some(PythLazerSlot {
        feed_id: PythLazerFeedId(1),
        observation,
      }),
      None,
      None,
    ]));

    let price = price_aggregate
      .query_price(&clock, OracleConfig::new(60, UFix64::new(10_000_000)))?;

    assert_eq!(price.spot, UFix64::new(1_000_000_000));
    assert_eq!(price.conf, UFix64::new(10_000_000));
    Ok(())
  }
}
