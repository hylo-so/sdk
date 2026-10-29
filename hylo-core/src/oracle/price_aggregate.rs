use std::ops::Deref;

use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::{
  CheckedAdd, CheckedDivFix, MulDiv, UFix128, UFix64, N2, N3, N4, N5, N6, N7,
  N8, N9,
};
use fix::typenum::Z0;
use pyth_lazer_protocol::api::MarketSession as LazerMarketSession;

use super::{FeedConfig, FeedSourceConfig};
use crate::error::CoreError;

pub const DEFAULT_MAX_STALENESS_SLOTS: u64 = 30;
pub const DEFAULT_MAX_STALENESS_SECS: u64 = 5;
pub const MICROS_PER_SECOND: u64 = 1_000_000;
pub const DEFAULT_MAX_STALENESS_SECS_US: u64 =
  DEFAULT_MAX_STALENESS_SECS * MICROS_PER_SECOND;
pub const DEFAULT_MAX_CONFIDENCE_INTERVAL_BPS: u16 = 500;
pub const DEFAULT_MAX_PRICE_BAND_BPS: u16 = 500;
pub const MIN_EXPONENT: i16 = -9;
pub const MAX_EXPONENT: i16 = -2;
pub const MAX_PRICE_AGGREGATE_OBSERVATIONS: usize = 8;

/// Method for aggregating price observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationMethod {
  SimpleAverage,
}

/// Slot count, second count, and confidence ratio.
/// Absent fields use the aggregate defaults.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ValidationConfig {
  pub slots: Option<u64>,
  pub secs: Option<u64>,
  pub confidence: Option<UFix64<N9>>,
}

/// Trading session recorded with a price observation.
#[derive(
  Debug,
  Default,
  Clone,
  Copy,
  PartialEq,
  Eq,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
)]
pub enum MarketSession {
  PreMarket,
  #[default]
  Regular,
  PostMarket,
  Overnight,
  Closed,
}

impl From<LazerMarketSession> for MarketSession {
  fn from(session: LazerMarketSession) -> MarketSession {
    match session {
      LazerMarketSession::PreMarket => MarketSession::PreMarket,
      LazerMarketSession::Regular => MarketSession::Regular,
      LazerMarketSession::PostMarket => MarketSession::PostMarket,
      LazerMarketSession::OverNight => MarketSession::Overnight,
      LazerMarketSession::Closed => MarketSession::Closed,
    }
  }
}

/// Source observation with a shared price and confidence exponent.
/// A zero price marks an unoccupied slot.
#[derive(
  Debug, Default, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace,
)]
pub struct PriceObservation {
  pub price: u64,
  pub exponent: i16,
  pub confidence: u64,
  pub posted_slot: u64,
  pub posted_timestamp_us: u64,
  pub market_session: MarketSession,
}

impl PriceObservation {
  fn normalized(value: u64, exponent: i16) -> Result<UFix64<N9>, CoreError> {
    PriceObservation::validate_exponent(exponent)?;

    match exponent {
      -2 => UFix64::<N2>::new(value).checked_convert(),
      -3 => UFix64::<N3>::new(value).checked_convert(),
      -4 => UFix64::<N4>::new(value).checked_convert(),
      -5 => UFix64::<N5>::new(value).checked_convert(),
      -6 => UFix64::<N6>::new(value).checked_convert(),
      -7 => UFix64::<N7>::new(value).checked_convert(),
      -8 => UFix64::<N8>::new(value).checked_convert(),
      -9 => Some(UFix64::<N9>::new(value)),
      _ => None,
    }
    .ok_or(CoreError::OracleArithmetic)
  }

  fn validate_price_band(
    &self,
    reference: &PriceObservation,
    price_band_bps: Option<u16>,
  ) -> Result<(), CoreError> {
    let price = PriceObservation::normalized(self.price, self.exponent)?;
    let reference_price =
      PriceObservation::normalized(reference.price, reference.exponent)?;
    let price_band = UFix64::<N4>::new(u64::from(
      price_band_bps.unwrap_or(DEFAULT_MAX_PRICE_BAND_BPS),
    ));
    let limit = reference_price
      .mul_div_floor(price_band, UFix64::<N4>::one())
      .ok_or(CoreError::OracleArithmetic)?;

    (price.abs_diff(&reference_price) <= limit)
      .then_some(())
      .ok_or(CoreError::OraclePriceBandExceeded)
  }

  /// Checks the price, staleness, and confidence limits.
  ///
  /// # Errors
  ///
  /// * The price is zero, stale, or outside the confidence limit.
  fn validate(
    &self,
    current_slot: u64,
    current_timestamp_us: u64,
    max_staleness_slots: Option<u64>,
    max_staleness_secs: Option<u64>,
    max_confidence: Option<UFix64<N9>>,
  ) -> Result<(), CoreError> {
    self.validate_price()?;
    self.validate_slot_age(current_slot, max_staleness_slots)?;
    self.validate_timestamp(current_timestamp_us, max_staleness_secs)?;
    self.validate_confidence(max_confidence)
  }

  fn validate_price(&self) -> Result<(), CoreError> {
    (self.price > 0)
      .then_some(())
      .ok_or(CoreError::OracleInvalidPrice)
  }

  fn validate_slot_age(
    &self,
    current_slot: u64,
    max_staleness_slots: Option<u64>,
  ) -> Result<(), CoreError> {
    let age = current_slot
      .checked_sub(self.posted_slot)
      .ok_or(CoreError::OracleOutdated)?;

    (age <= max_staleness_slots.unwrap_or(DEFAULT_MAX_STALENESS_SLOTS))
      .then_some(())
      .ok_or(CoreError::OracleOutdated)
  }

  fn validate_timestamp(
    &self,
    current_timestamp_us: u64,
    max_staleness_secs: Option<u64>,
  ) -> Result<(), CoreError> {
    let age = current_timestamp_us
      .checked_sub(self.posted_timestamp_us)
      .ok_or(CoreError::OracleOutdated)?;
    let limit = max_staleness_secs
      .map_or(Some(DEFAULT_MAX_STALENESS_SECS_US), |secs| {
        secs.checked_mul(MICROS_PER_SECOND)
      })
      .ok_or(CoreError::OracleArithmetic)?;

    (age <= limit)
      .then_some(())
      .ok_or(CoreError::OracleOutdated)
  }

  fn validate_confidence(
    &self,
    max_confidence: Option<UFix64<N9>>,
  ) -> Result<(), CoreError> {
    let price = PriceObservation::normalized(self.price, self.exponent)?;
    let confidence =
      PriceObservation::normalized(self.confidence, self.exponent)?;
    let default_max_confidence = UFix64::<N9>::new(
      u64::from(DEFAULT_MAX_CONFIDENCE_INTERVAL_BPS) * 100_000,
    );
    let confidence_ratio: fix::prelude::Fix<
      u64,
      fix::prelude::typenum::UInt<
        fix::prelude::typenum::UInt<
          fix::prelude::typenum::UInt<
            fix::prelude::typenum::UInt<
              fix::prelude::typenum::UTerm,
              fix::prelude::typenum::B1,
            >,
            fix::prelude::typenum::B0,
          >,
          fix::prelude::typenum::B1,
        >,
        fix::prelude::typenum::B0,
      >,
      fix::prelude::typenum::NInt<
        fix::prelude::typenum::UInt<
          fix::prelude::typenum::UInt<
            fix::prelude::typenum::UInt<
              fix::prelude::typenum::UInt<
                fix::prelude::typenum::UTerm,
                fix::prelude::typenum::B1,
              >,
              fix::prelude::typenum::B0,
            >,
            fix::prelude::typenum::B0,
          >,
          fix::prelude::typenum::B1,
        >,
      >,
    > = confidence
      .mul_div_floor(UFix64::one(), price)
      .ok_or(CoreError::OracleInvalidPrice)?;

    (confidence_ratio <= max_confidence.unwrap_or(default_max_confidence))
      .then_some(())
      .ok_or(CoreError::OracleConfidence)
  }

  /// Checks whether an exponent has supported price precision.
  ///
  /// # Errors
  ///
  /// * The exponent is outside the supported range.
  pub fn validate_exponent(exponent: i16) -> Result<(), CoreError> {
    (MIN_EXPONENT..=MAX_EXPONENT)
      .contains(&exponent)
      .then_some(())
      .ok_or(CoreError::OracleExponent)
  }
}

/// Observations posted for one feed, one slot per configured source.
#[derive(
  Debug, Default, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace,
)]
pub struct PriceAggregateObservations(
  [PriceObservation; MAX_PRICE_AGGREGATE_OBSERVATIONS],
);

impl PriceAggregateObservations {
  /// Wraps the fixed observation array stored by the oracle account.
  #[must_use]
  pub const fn new(
    observations: [PriceObservation; MAX_PRICE_AGGREGATE_OBSERVATIONS],
  ) -> PriceAggregateObservations {
    PriceAggregateObservations(observations)
  }
}

impl Deref for PriceAggregateObservations {
  type Target = [PriceObservation; MAX_PRICE_AGGREGATE_OBSERVATIONS];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

enum ObservationStatus {
  Fresh,
  Rejected,
}

impl PriceAggregateObservations {
  /// Combines fresh observations into a 9-decimal price.
  ///
  /// # Errors
  ///
  /// * The feed sources do not produce a fresh price.
  /// * An observation price or the average overflows.
  pub fn price(
    &self,
    aggregation_method: AggregationMethod,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
    feed_config: &FeedConfig,
  ) -> Result<UFix64<N9>, CoreError> {
    let mut observations = self.fresh_observations(
      current_slot,
      current_timestamp_us,
      validation_config,
      feed_config,
    )?;

    match aggregation_method {
      AggregationMethod::SimpleAverage => {
        let (sum, count) = observations.try_fold(
          (UFix128::<N9>::new(0), 0_u8),
          |(sum, count), observation| {
            PriceObservation::normalized(
              observation.price,
              observation.exponent,
            )
            .map(fix::Fix::widen::<u128>)
            .and_then(|price| {
              sum.checked_add(&price).ok_or(CoreError::OracleArithmetic)
            })
            .and_then(|sum| {
              count
                .checked_add(1)
                .map(|count| (sum, count))
                .ok_or(CoreError::OracleArithmetic)
            })
          },
        )?;

        (count > 0)
          .then_some(())
          .ok_or(CoreError::OracleNoFreshPriceObservations)
          .and_then(|()| {
            sum
              .checked_div(&UFix128::<Z0>::new(u128::from(count)))
              .ok_or(CoreError::OracleArithmetic)
          })
          .and_then(|average| {
            average.narrow::<u64>().ok_or(CoreError::OracleConversion)
          })
      }
    }
  }

  fn fresh_observations(
    &self,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
    feed_config: &FeedConfig,
  ) -> Result<impl Iterator<Item = &PriceObservation>, CoreError> {
    self
      .0
      .iter()
      .zip(feed_config.feeds.iter())
      .map(|(observation, source)| {
        match self.observation_status(
          observation,
          source,
          current_slot,
          current_timestamp_us,
          validation_config,
        )? {
          ObservationStatus::Fresh => Ok(Some(observation)),
          ObservationStatus::Rejected => Ok(None),
        }
      })
      .collect::<Result<Vec<_>, CoreError>>()
      .map(|observations| observations.into_iter().flatten())
  }

  fn observation_status(
    &self,
    observation: &PriceObservation,
    source: &FeedSourceConfig,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
  ) -> Result<ObservationStatus, CoreError> {
    match self.validate_observation(
      observation,
      source,
      current_slot,
      current_timestamp_us,
      validation_config,
    ) {
      Ok(()) => Ok(ObservationStatus::Fresh),
      Err(error) if PriceAggregateObservations::rejects_observation(error) => {
        Ok(ObservationStatus::Rejected)
      }
      Err(error) => Err(error),
    }
  }

  fn rejects_observation(error: CoreError) -> bool {
    matches!(
      error,
      CoreError::OracleConfidence
        | CoreError::OracleInvalidPrice
        | CoreError::OracleInvalidReferenceFeed
        | CoreError::OraclePriceBandExceeded
        | CoreError::OracleOutdated
    )
  }

  fn validate_observation(
    &self,
    observation: &PriceObservation,
    source: &FeedSourceConfig,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
  ) -> Result<(), CoreError> {
    match observation.market_session {
      MarketSession::Regular => observation.validate(
        current_slot,
        current_timestamp_us,
        validation_config.slots,
        validation_config.secs,
        validation_config.confidence,
      ),
      MarketSession::PreMarket
      | MarketSession::PostMarket
      | MarketSession::Overnight
      | MarketSession::Closed => {
        observation.validate_price()?;
        observation.validate_confidence(validation_config.confidence)?;
        let index = usize::from(
          source
            .reference_feed
            .ok_or(CoreError::OracleInvalidReferenceFeed)?,
        );
        let reference = self
          .0
          .get(index)
          .ok_or(CoreError::OracleInvalidReferenceFeed)?;

        (reference.market_session == MarketSession::Regular)
          .then_some(())
          .ok_or(CoreError::OracleInvalidReferenceFeed)?;
        reference.validate(
          current_slot,
          current_timestamp_us,
          validation_config.slots,
          validation_config.secs,
          validation_config.confidence,
        )?;
        observation.validate_price_band(reference, None)
      }
    }
  }

  /// Stores an observation at its configured source index.
  ///
  /// # Errors
  ///
  /// * The source index exceeds the observation capacity.
  /// * The observation exponent is outside the supported range.
  pub fn update_observation(
    &mut self,
    index: usize,
    observation: PriceObservation,
  ) -> Result<(), CoreError> {
    PriceObservation::validate_exponent(observation.exponent)?;

    self
      .0
      .get_mut(index)
      .map(|slot| *slot = observation)
      .ok_or(CoreError::OracleObservationCapacityExceeded)
  }
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::Pubkey;
  use fix::prelude::UFix64;
  use fix::typenum::N9;
  use pyth_lazer_protocol::api::MarketSession as LazerMarketSession;

  use super::{
    AggregationMethod, FeedConfig, FeedSourceConfig, MarketSession,
    PriceAggregateObservations, PriceObservation, ValidationConfig,
  };
  use crate::error::CoreError;
  use crate::oracle::{OracleSourceConfig, PythLazerFeedId};

  const SOURCE: FeedSourceConfig = FeedSourceConfig {
    source: OracleSourceConfig::PythLazer {
      feed_id: PythLazerFeedId(1),
    },
    reference_feed: None,
  };

  const CONFIG: FeedConfig = FeedConfig {
    mint: Pubkey::new_from_array([0; 32]),
    feeds: &[SOURCE, SOURCE],
  };

  const BANDED_CONFIG: FeedConfig = FeedConfig {
    mint: Pubkey::new_from_array([0; 32]),
    feeds: &[
      FeedSourceConfig {
        source: OracleSourceConfig::PythLazer {
          feed_id: PythLazerFeedId(2),
        },
        reference_feed: Some(1),
      },
      SOURCE,
    ],
  };

  fn observation(
    price: u64,
    posted_slot: u64,
    posted_timestamp_us: u64,
  ) -> PriceObservation {
    PriceObservation {
      price,
      exponent: -9,
      confidence: 0,
      posted_slot,
      posted_timestamp_us,
      market_session: MarketSession::Regular,
    }
  }

  #[test]
  fn lazer_market_session_conversion_preserves_variant() {
    [
      (LazerMarketSession::PreMarket, MarketSession::PreMarket),
      (LazerMarketSession::Regular, MarketSession::Regular),
      (LazerMarketSession::PostMarket, MarketSession::PostMarket),
      (LazerMarketSession::OverNight, MarketSession::Overnight),
      (LazerMarketSession::Closed, MarketSession::Closed),
    ]
    .into_iter()
    .for_each(|(source, expected)| {
      assert_eq!(MarketSession::from(source), expected);
    });
  }

  #[test]
  fn price_averages_fresh_observations() {
    let current_slot = 1;
    let current_timestamp_us = 1_000_000;
    let mut observations = PriceAggregateObservations::default();
    assert!(observations
      .update_observation(
        0,
        observation(100_000_000_000, current_slot, current_timestamp_us)
      )
      .is_ok());
    assert!(observations
      .update_observation(
        1,
        observation(200_000_000_000, current_slot, current_timestamp_us)
      )
      .is_ok());

    assert_eq!(
      observations.price(
        AggregationMethod::SimpleAverage,
        current_slot,
        current_timestamp_us,
        ValidationConfig::default(),
        &CONFIG,
      ),
      Ok(UFix64::<N9>::new(150_000_000_000)),
    );
  }

  #[test]
  fn price_excludes_stale_observations() {
    let current_slot = 31;
    let current_timestamp_us = 31_000_000;
    let mut observations = PriceAggregateObservations::default();
    assert!(observations
      .update_observation(0, observation(100_000_000_000, 0, 0))
      .is_ok());
    assert!(observations
      .update_observation(
        1,
        observation(200_000_000_000, current_slot, current_timestamp_us)
      )
      .is_ok());

    assert_eq!(
      observations.price(
        AggregationMethod::SimpleAverage,
        current_slot,
        current_timestamp_us,
        ValidationConfig::default(),
        &CONFIG,
      ),
      Ok(UFix64::<N9>::new(200_000_000_000)),
    );
  }

  #[test]
  fn confidence_limit_uses_price_units() {
    let observation = PriceObservation {
      price: 100_000_000_000,
      exponent: -9,
      confidence: 5_010_000_000,
      posted_slot: 0,
      posted_timestamp_us: 0,
      market_session: MarketSession::Regular,
    };

    assert_eq!(
      observation.validate_confidence(None),
      Err(CoreError::OracleConfidence),
    );
  }

  #[test]
  fn non_regular_observations_use_a_fresh_regular_reference() {
    let current_slot = 100;
    let current_timestamp_us = 100_000_000;
    let mut observations = PriceAggregateObservations::default();
    assert!(observations
      .update_observation(
        1,
        PriceObservation {
          price: 10_000,
          exponent: -2,
          confidence: 1,
          posted_slot: current_slot,
          posted_timestamp_us: current_timestamp_us,
          market_session: MarketSession::Regular,
        },
      )
      .is_ok());
    assert!(observations
      .update_observation(
        0,
        PriceObservation {
          price: 105_000,
          exponent: -3,
          confidence: 1,
          posted_slot: 1,
          posted_timestamp_us: 1,
          market_session: MarketSession::Closed,
        },
      )
      .is_ok());

    assert_eq!(
      observations.price(
        AggregationMethod::SimpleAverage,
        current_slot,
        current_timestamp_us,
        ValidationConfig::default(),
        &BANDED_CONFIG,
      ),
      Ok(UFix64::<N9>::new(102_500_000_000)),
    );
  }
}
