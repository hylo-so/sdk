use std::ops::Deref;

use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::{
  CheckedAdd, CheckedDivFix, MulDiv, UFix128, UFix64, N2, N3, N4, N5, N6, N7,
  N8, N9,
};
use fix::typenum::Z0;

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
pub const MAX_PRICE_AGGREGATE_SAMPLES: usize = 8;

/// Method for aggregating price samples.
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

impl PriceSample {
  fn normalized(value: u64, exponent: i16) -> Result<UFix64<N9>, CoreError> {
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
    .ok_or(CoreError::OracleArithmetic)
  }

  fn validate_price_band(
    &self,
    reference: &PriceSample,
    price_band_bps: Option<u16>,
  ) -> Result<(), CoreError> {
    let price = PriceSample::normalized(self.price, self.exponent)?;
    let reference_price =
      PriceSample::normalized(reference.price, reference.exponent)?;
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
    let price = PriceSample::normalized(self.price, self.exponent)?;
    let confidence = PriceSample::normalized(self.confidence, self.exponent)?;
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

/// Samples posted for one feed, one slot per configured source.
#[derive(
  Debug, Default, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace,
)]
pub struct PriceAggregateSamples([PriceSample; MAX_PRICE_AGGREGATE_SAMPLES]);

impl PriceAggregateSamples {
  /// Wraps the fixed sample array stored by the oracle account.
  #[must_use]
  pub const fn new(
    samples: [PriceSample; MAX_PRICE_AGGREGATE_SAMPLES],
  ) -> PriceAggregateSamples {
    PriceAggregateSamples(samples)
  }
}

impl Deref for PriceAggregateSamples {
  type Target = [PriceSample; MAX_PRICE_AGGREGATE_SAMPLES];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

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
  ) -> Result<UFix64<N9>, CoreError> {
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
          .ok_or(CoreError::OracleNoFreshPriceSamples)
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

  fn fresh_samples(
    &self,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
    feed_config: &FeedConfig,
  ) -> Result<impl Iterator<Item = &PriceSample>, CoreError> {
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
      .collect::<Result<Vec<_>, CoreError>>()
      .map(|samples| samples.into_iter().flatten())
  }

  fn sample_status(
    &self,
    sample: &PriceSample,
    source: &FeedSourceConfig,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
  ) -> Result<SampleStatus, CoreError> {
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

  fn rejects_sample(error: CoreError) -> bool {
    matches!(
      error,
      CoreError::OracleConfidence
        | CoreError::OracleInvalidPrice
        | CoreError::OracleInvalidReferenceFeed
        | CoreError::OraclePriceBandExceeded
        | CoreError::OracleOutdated
    )
  }

  fn validate_sample(
    &self,
    sample: &PriceSample,
    source: &FeedSourceConfig,
    current_slot: u64,
    current_timestamp_us: u64,
    validation_config: ValidationConfig,
  ) -> Result<(), CoreError> {
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
  ) -> Result<(), CoreError> {
    PriceSample::validate_exponent(sample.exponent)?;

    self
      .0
      .get_mut(index)
      .map(|slot| *slot = sample)
      .ok_or(CoreError::OracleSampleCapacityExceeded)
  }
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::Pubkey;
  use fix::prelude::UFix64;
  use fix::typenum::N9;

  use super::{
    AggregationMethod, FeedConfig, FeedSourceConfig, MarketSession,
    PriceAggregateSamples, PriceSample, ValidationConfig,
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
      posted_slot: 0,
      posted_timestamp_us: 0,
      market_session: MarketSession::Regular,
    };

    assert_eq!(
      sample.validate_confidence(None),
      Err(CoreError::OracleConfidence),
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
