use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::{
  CheckedAdd, CheckedDivFix, MulDiv, UFix128, UFix64, N2, N3, N4, N5, N6, N7,
  N8, N9,
};
use fix::typenum::Z0;

use crate::{FeedConfig, FeedSourceConfig};

pub const DEFAULT_MAX_STALENESS_SLOTS: u64 = 30;
pub const DEFAULT_MAX_STALENESS_SECS: u64 = 5;
pub const MICROS_PER_SECOND: u64 = 1_000_000;
pub const DEFAULT_MAX_STALENESS_SECS_US: u64 =
  DEFAULT_MAX_STALENESS_SECS * MICROS_PER_SECOND;
pub const DEFAULT_MAX_CONFIDENCE_INTERVAL_BPS: u16 = 500;
pub const DEFAULT_MAX_PRICE_BAND_BPS: u16 = 500;
pub const MIN_EXPONENT: i16 = -9;
pub const MAX_EXPONENT: i16 = -2;
pub const MAX_PRICE_AGGREGATE_SAMPLES: usize = 8;
pub const MAX_BASIS_POINTS: u16 = 10_000;

/// Method for aggregating price samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationMethod {
  SimpleAverage,
}

/// Slot count, second count, and confidence limit in basis points.
/// Absent fields use the aggregate defaults.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ValidationConfig {
  pub slots: Option<u64>,
  pub secs: Option<u64>,
  pub confidence: Option<u16>,
}

/// Trading session recorded with a price sample.
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

/// Source observation with a shared price and confidence exponent.
/// A zero price marks an unoccupied slot.
#[derive(
  Debug, Default, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace,
)]
pub struct PriceSample {
  pub price: u64,
  pub exponent: i16,
  pub confidence: u64,
  pub posted_slot: u64,
  pub posted_timestamp_us: u64,
  pub market_session: MarketSession,
}

/// Failure while validating or aggregating price samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceAggregateError {
  ArithmeticOverflow,
  ConfidenceExceededLimit,
  ConversionError,
  ExponentOutOfRange,
  InvalidPrice,
  InvalidReferenceFeed,
  NoFreshPriceSamples,
  PriceBandExceeded,
  SampleCapacityExceeded,
  StalePriceSample,
}

impl PriceSample {
  fn normalized(
    value: u64,
    exponent: i16,
  ) -> Result<UFix64<N9>, PriceAggregateError> {
    PriceSample::validate_exponent(exponent)?;

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
    .ok_or(PriceAggregateError::ArithmeticOverflow)
  }

  fn validate_price_band(
    &self,
    reference: &PriceSample,
    price_band_bps: Option<u16>,
  ) -> Result<(), PriceAggregateError> {
    let price = PriceSample::normalized(self.price, self.exponent)?;
    let reference_price =
      PriceSample::normalized(reference.price, reference.exponent)?;
    let price_band = UFix64::<N4>::new(u64::from(
      price_band_bps.unwrap_or(DEFAULT_MAX_PRICE_BAND_BPS),
    ));
    let limit = reference_price
      .mul_div_floor(price_band, UFix64::<N4>::one())
      .ok_or(PriceAggregateError::ArithmeticOverflow)?;

    (price.abs_diff(&reference_price) <= limit)
      .then_some(())
      .ok_or(PriceAggregateError::PriceBandExceeded)
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
    max_confidence_bps: Option<u16>,
  ) -> Result<(), PriceAggregateError> {
    self.validate_price()?;
    self.validate_slot_age(current_slot, max_staleness_slots)?;
    self.validate_timestamp(current_timestamp_us, max_staleness_secs)?;
    self.validate_confidence(max_confidence_bps)
  }

  fn validate_price(&self) -> Result<(), PriceAggregateError> {
    (self.price > 0)
      .then_some(())
      .ok_or(PriceAggregateError::InvalidPrice)
  }

  fn validate_slot_age(
    &self,
    current_slot: u64,
    max_staleness_slots: Option<u64>,
  ) -> Result<(), PriceAggregateError> {
    let age = current_slot
      .checked_sub(self.posted_slot)
      .ok_or(PriceAggregateError::StalePriceSample)?;

    (age <= max_staleness_slots.unwrap_or(DEFAULT_MAX_STALENESS_SLOTS))
      .then_some(())
      .ok_or(PriceAggregateError::StalePriceSample)
  }

  fn validate_timestamp(
    &self,
    current_timestamp_us: u64,
    max_staleness_secs: Option<u64>,
  ) -> Result<(), PriceAggregateError> {
    let age = current_timestamp_us
      .checked_sub(self.posted_timestamp_us)
      .ok_or(PriceAggregateError::StalePriceSample)?;
    let limit = max_staleness_secs
      .map_or(Some(DEFAULT_MAX_STALENESS_SECS_US), |secs| {
        secs.checked_mul(MICROS_PER_SECOND)
      })
      .ok_or(PriceAggregateError::ArithmeticOverflow)?;

    (age <= limit)
      .then_some(())
      .ok_or(PriceAggregateError::StalePriceSample)
  }

  fn validate_confidence(
    &self,
    max_confidence_bps: Option<u16>,
  ) -> Result<(), PriceAggregateError> {
    let price = PriceSample::normalized(self.price, self.exponent)?;
    let confidence = PriceSample::normalized(self.confidence, self.exponent)?;
    let basis_points =
      UFix64::<Z0>::new(u64::from(MAX_BASIS_POINTS)).convert::<N9>();
    let max_confidence = UFix64::<Z0>::new(u64::from(
      max_confidence_bps.unwrap_or(DEFAULT_MAX_CONFIDENCE_INTERVAL_BPS),
    ))
    .convert::<N9>();
    let confidence_bps = confidence
      .mul_div_floor(basis_points, price)
      .ok_or(PriceAggregateError::InvalidPrice)?;

    (confidence_bps <= max_confidence)
      .then_some(())
      .ok_or(PriceAggregateError::ConfidenceExceededLimit)
  }

  /// Checks whether an exponent has supported price precision.
  ///
  /// # Errors
  ///
  /// * The exponent is outside the supported range.
  fn validate_exponent(exponent: i16) -> Result<(), PriceAggregateError> {
    (MIN_EXPONENT..=MAX_EXPONENT)
      .contains(&exponent)
      .then_some(())
      .ok_or(PriceAggregateError::ExponentOutOfRange)
  }
}

/// Samples posted for one feed, one slot per configured source.
#[derive(
  Debug, Default, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace,
)]
pub struct PriceAggregateSamples([PriceSample; MAX_PRICE_AGGREGATE_SAMPLES]);

enum SampleStatus {
  Fresh,
  Rejected,
}

impl PriceAggregateSamples {
  /// Combines fresh samples into a 9-decimal price.
  ///
  /// # Errors
  ///
  /// * The feed sources do not produce a fresh price.
  /// * A sample price or the average overflows.
  pub fn price(
    &self,
    aggregation_method: AggregationMethod,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
    feed_config: &FeedConfig,
  ) -> Result<UFix64<N9>, PriceAggregateError> {
    let mut samples = self.fresh_samples(
      current_slot,
      current_timestamp_us,
      validation_config,
      feed_config,
    )?;

    match aggregation_method {
      AggregationMethod::SimpleAverage => {
        let (sum, count) = samples.try_fold(
          (UFix128::<N9>::new(0), 0_u8),
          |(sum, count), sample| {
            PriceSample::normalized(sample.price, sample.exponent)
              .map(fix::Fix::widen::<u128>)
              .and_then(|price| {
                sum
                  .checked_add(&price)
                  .ok_or(PriceAggregateError::ArithmeticOverflow)
              })
              .and_then(|sum| {
                count
                  .checked_add(1)
                  .map(|count| (sum, count))
                  .ok_or(PriceAggregateError::ArithmeticOverflow)
              })
          },
        )?;

        (count > 0)
          .then_some(())
          .ok_or(PriceAggregateError::NoFreshPriceSamples)
          .and_then(|()| {
            sum
              .checked_div(&UFix128::<Z0>::new(u128::from(count)))
              .ok_or(PriceAggregateError::ArithmeticOverflow)
          })
          .and_then(|average| {
            average
              .narrow::<u64>()
              .ok_or(PriceAggregateError::ConversionError)
          })
      }
    }
  }

  fn fresh_samples(
    &self,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
    feed_config: &FeedConfig,
  ) -> Result<impl Iterator<Item = &PriceSample>, PriceAggregateError> {
    self
      .0
      .iter()
      .zip(feed_config.feeds.iter())
      .map(|(sample, source)| {
        match self.sample_status(
          sample,
          source,
          current_slot,
          current_timestamp_us,
          validation_config,
        )? {
          SampleStatus::Fresh => Ok(Some(sample)),
          SampleStatus::Rejected => Ok(None),
        }
      })
      .collect::<Result<Vec<_>, PriceAggregateError>>()
      .map(|samples| samples.into_iter().flatten())
  }

  fn sample_status(
    &self,
    sample: &PriceSample,
    source: &FeedSourceConfig,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
  ) -> Result<SampleStatus, PriceAggregateError> {
    match self.validate_sample(
      sample,
      source,
      current_slot,
      current_timestamp_us,
      validation_config,
    ) {
      Ok(()) => Ok(SampleStatus::Fresh),
      Err(error) if PriceAggregateSamples::rejects_sample(error) => {
        Ok(SampleStatus::Rejected)
      }
      Err(error) => Err(error),
    }
  }

  fn rejects_sample(error: PriceAggregateError) -> bool {
    matches!(
      error,
      PriceAggregateError::ConfidenceExceededLimit
        | PriceAggregateError::InvalidPrice
        | PriceAggregateError::InvalidReferenceFeed
        | PriceAggregateError::PriceBandExceeded
        | PriceAggregateError::StalePriceSample
    )
  }

  fn validate_sample(
    &self,
    sample: &PriceSample,
    source: &FeedSourceConfig,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
  ) -> Result<(), PriceAggregateError> {
    match sample.market_session {
      MarketSession::Regular => sample.validate(
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
        sample.validate_price()?;
        sample.validate_confidence(validation_config.confidence)?;
        let index = usize::from(
          source
            .reference_feed
            .ok_or(PriceAggregateError::InvalidReferenceFeed)?,
        );
        let reference = self
          .0
          .get(index)
          .ok_or(PriceAggregateError::InvalidReferenceFeed)?;

        (reference.market_session == MarketSession::Regular)
          .then_some(())
          .ok_or(PriceAggregateError::InvalidReferenceFeed)?;
        reference.validate(
          current_slot,
          current_timestamp_us,
          validation_config.slots,
          validation_config.secs,
          validation_config.confidence,
        )?;
        sample.validate_price_band(reference, None)
      }
    }
  }

  /// Stores a sample at its configured source index.
  ///
  /// # Errors
  ///
  /// * The source index exceeds the sample capacity.
  /// * The sample exponent is outside the supported range.
  pub fn update_sample(
    &mut self,
    index: usize,
    sample: PriceSample,
  ) -> Result<(), PriceAggregateError> {
    PriceSample::validate_exponent(sample.exponent)?;

    self
      .0
      .get_mut(index)
      .map(|slot| *slot = sample)
      .ok_or(PriceAggregateError::SampleCapacityExceeded)
  }
}

#[cfg(test)]
mod tests {
  use fix::prelude::UFix64;
  use fix::typenum::N9;

  use super::{
    AggregationMethod, MarketSession, PriceAggregateError,
    PriceAggregateSamples, PriceSample, ValidationConfig,
  };
  use crate::{
    FeedConfig, FeedSourceConfig, OracleSourceConfig, PythLazerFeedId,
  };

  const SOURCE: FeedSourceConfig = FeedSourceConfig {
    source: OracleSourceConfig::PythLazer {
      feed_id: PythLazerFeedId(1),
    },
    reference_feed: None,
  };

  const CONFIG: FeedConfig = FeedConfig {
    id: 0,
    feeds: &[SOURCE, SOURCE],
  };

  const BANDED_CONFIG: FeedConfig = FeedConfig {
    id: 0,
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

  fn sample(
    price: u64,
    posted_slot: u64,
    posted_timestamp_us: u64,
  ) -> PriceSample {
    PriceSample {
      price,
      exponent: -9,
      confidence: 0,
      posted_slot,
      posted_timestamp_us,
      market_session: MarketSession::Regular,
    }
  }

  #[test]
  fn price_averages_fresh_samples() {
    let current_slot = 1;
    let current_timestamp_us = 1_000_000;
    let mut samples = PriceAggregateSamples::default();
    assert!(samples
      .update_sample(
        0,
        sample(100_000_000_000, current_slot, current_timestamp_us)
      )
      .is_ok());
    assert!(samples
      .update_sample(
        1,
        sample(200_000_000_000, current_slot, current_timestamp_us)
      )
      .is_ok());

    assert_eq!(
      samples.price(
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
  fn price_excludes_stale_samples() {
    let current_slot = 31;
    let current_timestamp_us = 31_000_000;
    let mut samples = PriceAggregateSamples::default();
    assert!(samples
      .update_sample(0, sample(100_000_000_000, 0, 0))
      .is_ok());
    assert!(samples
      .update_sample(
        1,
        sample(200_000_000_000, current_slot, current_timestamp_us)
      )
      .is_ok());

    assert_eq!(
      samples.price(
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
    let sample = PriceSample {
      price: 100_000_000_000,
      exponent: -9,
      confidence: 5_010_000_000,
      ..PriceSample::default()
    };

    assert_eq!(
      sample.validate_confidence(None),
      Err(PriceAggregateError::ConfidenceExceededLimit),
    );
  }

  #[test]
  fn non_regular_samples_use_a_fresh_regular_reference() {
    let current_slot = 100;
    let current_timestamp_us = 100_000_000;
    let mut samples = PriceAggregateSamples::default();
    assert!(samples
      .update_sample(
        1,
        PriceSample {
          price: 10_000,
          exponent: -2,
          confidence: 1,
          posted_slot: current_slot,
          posted_timestamp_us: current_timestamp_us,
          market_session: MarketSession::Regular,
        },
      )
      .is_ok());
    assert!(samples
      .update_sample(
        0,
        PriceSample {
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
      samples.price(
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
