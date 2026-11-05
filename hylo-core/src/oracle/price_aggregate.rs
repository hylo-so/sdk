use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::{
  CheckedAdd, CheckedDivFix, CheckedSub, MulDiv, UFix128, UFix64, N2, N3, N4,
  N5, N6, N7, N8, N9,
};
use fix::typenum::{Integer, Z0};

use super::{OracleConfig, OraclePrice};
use crate::error::CoreError;
use crate::error::CoreError::{OracleNegativeTime, OraclePriceRange};
use crate::solana_clock::SolanaClock;

pub const DEFAULT_MAX_STALENESS_SLOTS: u64 = 30;
pub const DEFAULT_MAX_STALENESS_SECS: u64 = 5;
pub const MICROS_PER_SECOND: u64 = 1_000_000;
pub const DEFAULT_MAX_STALENESS_SECS_US: u64 =
  DEFAULT_MAX_STALENESS_SECS * MICROS_PER_SECOND;
pub const DEFAULT_MAX_CONFIDENCE_INTERVAL_BPS: u16 = 500;
pub const DEFAULT_MAX_PRICE_BAND_BPS: u16 = 500;
pub const MIN_EXPONENT: i16 = -9;
pub const MAX_EXPONENT: i16 = -2;

/// Methods for reading prices from a price aggregate account.
pub trait PriceAggregateExt {
  /// Returns every observation stored in the aggregate.
  fn observations(&self) -> impl Iterator<Item = PriceObservation>;

  /// Fetches the validated aggregate price.
  ///
  /// # Errors
  ///
  /// * The clock timestamp is negative or overflows microseconds.
  /// * No observations pass the configured validation.
  /// * Price aggregation overflows or cannot convert to the quote precision.
  fn query_price<C: SolanaClock>(
    &self,
    clock: &C,
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
      price_band_bps: None,
    };
    aggregate_price(
      self.observations(),
      AggregationMethod::SimpleAverage,
      clock.slot(),
      current_timestamp_us,
      validation_config,
    )
  }

  /// Converts the newest posted timestamp to seconds.
  ///
  /// # Errors
  ///
  /// * The aggregate has no observations.
  /// * The timestamp does not fit in `i64`.
  fn latest_timestamp_secs(&self) -> Result<i64, CoreError> {
    self
      .observations()
      .map(|observation| observation.posted_timestamp_us)
      .max()
      .and_then(|timestamp_us| timestamp_us.checked_div(MICROS_PER_SECOND))
      .and_then(|timestamp_secs| i64::try_from(timestamp_secs).ok())
      .ok_or(CoreError::OracleOutdated)
  }
}

fn slot_interval(oracle_interval_secs: u64) -> Option<u64> {
  let time: UFix64<N2> = UFix64::<Z0>::new(oracle_interval_secs).convert();
  let slot_time = UFix64::<N2>::new(20);
  time.checked_div(&slot_time).map(|interval| interval.bits)
}

impl PriceAggregateExt for hylo_idl::oracle::accounts::PriceAggregate {
  fn observations(&self) -> impl Iterator<Item = PriceObservation> {
    self
      .pyth_lazer
      .0
      .iter()
      .flatten()
      .map(|slot| PriceObservation::from(slot.observation))
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
      .ok_or(OraclePriceRange)?;
    Ok(PriceRange { lower, upper })
  }

  /// Makes a range of one price, useful in test scenarios.
  #[must_use]
  pub fn one(price: UFix64<Exp>) -> PriceRange<Exp> {
    PriceRange {
      lower: price,
      upper: price,
    }
  }

  pub(crate) fn new(lower: UFix64<Exp>, upper: UFix64<Exp>) -> PriceRange<Exp> {
    PriceRange { lower, upper }
  }
}

/// Method for aggregating price observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationMethod {
  SimpleAverage,
}

/// Slot count, second count, confidence ratio, and price band.
/// Absent fields use the aggregate defaults.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ValidationConfig {
  pub slots: Option<u64>,
  pub secs: Option<u64>,
  pub confidence: Option<UFix64<N9>>,
  pub price_band_bps: Option<u16>,
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

/// Source observation with a shared price and confidence exponent.
/// A zero price marks a feed that has not posted yet.
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
pub struct PriceObservation {
  pub price: u64,
  pub exponent: i16,
  pub confidence: u64,
  pub posted_slot: u64,
  pub posted_timestamp_us: u64,
  pub market_session: MarketSession,
}

/// Observation price and confidence converted to 9 decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NormalizedObservation {
  price: UFix64<N9>,
  confidence: UFix64<N9>,
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

  fn normalize(&self) -> Result<NormalizedObservation, CoreError> {
    Ok(NormalizedObservation {
      price: PriceObservation::normalized(self.price, self.exponent)?,
      confidence: PriceObservation::normalized(self.confidence, self.exponent)?,
    })
  }

  fn validate_price_band(
    &self,
    reference_price: UFix64<N9>,
    price_band_bps: Option<u16>,
  ) -> Result<(), CoreError> {
    let price = PriceObservation::normalized(self.price, self.exponent)?;
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
    let age = current_timestamp_us.saturating_sub(self.posted_timestamp_us);
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
    let confidence_ratio = confidence
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

/// Combines fresh observations into a 9-decimal spot price with the widest
/// confidence among them.
///
/// Observations posted in a regular market session pass on price, staleness,
/// and confidence. Observations posted outside a regular session must also sit
/// within the price band of the average of the fresh regular observations.
///
/// # Errors
///
/// * No observation passes validation.
/// * An observation price or the average overflows.
pub fn aggregate_price(
  observations: impl Iterator<Item = PriceObservation>,
  aggregation_method: AggregationMethod,
  current_slot: u64,
  current_timestamp_us: u64,
  validation_config: ValidationConfig,
) -> Result<OraclePrice, CoreError> {
  let observations: Vec<PriceObservation> = observations.collect();
  let regular = observations
    .iter()
    .filter(|observation| observation.market_session == MarketSession::Regular)
    .filter_map(|observation| {
      fresh(
        observation,
        current_slot,
        current_timestamp_us,
        validation_config,
      )
      .transpose()
    })
    .collect::<Result<Vec<_>, CoreError>>()?;
  let reference_price = combine(aggregation_method, regular.iter().copied())
    .ok()
    .map(|price| price.spot);
  let banded = observations
    .iter()
    .filter(|observation| observation.market_session != MarketSession::Regular)
    .filter_map(|observation| {
      reference_price.and_then(|reference_price| {
        banded(
          observation,
          reference_price,
          validation_config.confidence,
          validation_config.price_band_bps,
        )
        .transpose()
      })
    })
    .collect::<Result<Vec<_>, CoreError>>()?;
  combine(aggregation_method, regular.into_iter().chain(banded))
}

fn fresh(
  observation: &PriceObservation,
  current_slot: u64,
  current_timestamp_us: u64,
  validation_config: ValidationConfig,
) -> Result<Option<NormalizedObservation>, CoreError> {
  match observation.validate(
    current_slot,
    current_timestamp_us,
    validation_config.slots,
    validation_config.secs,
    validation_config.confidence,
  ) {
    Ok(()) => observation.normalize().map(Some),
    Err(error) if rejects_observation(error) => Ok(None),
    Err(error) => Err(error),
  }
}

fn banded(
  observation: &PriceObservation,
  reference_price: UFix64<N9>,
  max_confidence: Option<UFix64<N9>>,
  price_band_bps: Option<u16>,
) -> Result<Option<NormalizedObservation>, CoreError> {
  let validation = observation
    .validate_price()
    .and_then(|()| observation.validate_confidence(max_confidence))
    .and_then(|()| {
      observation.validate_price_band(reference_price, price_band_bps)
    });
  match validation {
    Ok(()) => observation.normalize().map(Some),
    Err(error) if rejects_observation(error) => Ok(None),
    Err(error) => Err(error),
  }
}

fn rejects_observation(error: CoreError) -> bool {
  matches!(
    error,
    CoreError::OracleConfidence
      | CoreError::OracleInvalidPrice
      | CoreError::OraclePriceBandExceeded
      | CoreError::OracleOutdated
  )
}

fn combine(
  aggregation_method: AggregationMethod,
  mut observations: impl Iterator<Item = NormalizedObservation>,
) -> Result<OraclePrice, CoreError> {
  match aggregation_method {
    AggregationMethod::SimpleAverage => {
      let (sum, count, conf) = observations.try_fold(
        (UFix128::<N9>::new(0), 0_u8, UFix64::<N9>::new(0)),
        |(sum, count, conf), observation| -> Result<_, CoreError> {
          let sum = sum
            .checked_add(&observation.price.widen::<u128>())
            .ok_or(CoreError::OracleArithmetic)?;
          let count =
            count.checked_add(1).ok_or(CoreError::OracleArithmetic)?;
          let conf = if observation.confidence > conf {
            observation.confidence
          } else {
            conf
          };
          Ok((sum, count, conf))
        },
      )?;
      let spot = (count > 0)
        .then_some(())
        .ok_or(CoreError::OracleNoFreshPriceObservations)
        .and_then(|()| {
          sum
            .checked_div(&UFix128::<Z0>::new(u128::from(count)))
            .ok_or(CoreError::OracleArithmetic)
        })
        .and_then(|average| {
          average.narrow::<u64>().ok_or(CoreError::OracleConversion)
        })?;
      Ok(OraclePrice { spot, conf })
    }
  }
}

#[cfg(test)]
mod tests {
  use fix::prelude::UFix64;
  use fix::typenum::N9;

  use super::{
    aggregate_price, AggregationMethod, MarketSession, OraclePrice,
    PriceObservation, ValidationConfig,
  };
  use crate::error::CoreError;

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

  fn price(
    observations: &[PriceObservation],
    current_slot: u64,
    current_timestamp_us: u64,
  ) -> Result<OraclePrice, CoreError> {
    aggregate_price(
      observations.iter().copied(),
      AggregationMethod::SimpleAverage,
      current_slot,
      current_timestamp_us,
      ValidationConfig::default(),
    )
  }

  #[test]
  fn price_averages_fresh_observations() {
    let observations = [
      observation(100_000_000_000, 1, 1_000_000),
      observation(200_000_000_000, 1, 1_000_000),
    ];
    assert_eq!(
      price(&observations, 1, 1_000_000),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(150_000_000_000),
        conf: UFix64::zero(),
      }),
    );
  }

  #[test]
  fn price_reports_widest_confidence() {
    let mut narrow = observation(100_000_000_000, 1, 1_000_000);
    narrow.confidence = 1_000_000_000;
    let mut wide = observation(100_000_000_000, 1, 1_000_000);
    wide.confidence = 2_000_000_000;
    assert_eq!(
      price(&[narrow, wide], 1, 1_000_000),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(100_000_000_000),
        conf: UFix64::<N9>::new(2_000_000_000),
      }),
    );
  }

  #[test]
  fn price_excludes_stale_observations() {
    let observations = [
      observation(100_000_000_000, 0, 0),
      observation(200_000_000_000, 31, 31_000_000),
    ];
    assert_eq!(
      price(&observations, 31, 31_000_000),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(200_000_000_000),
        conf: UFix64::zero(),
      }),
    );
  }

  #[test]
  fn price_fails_without_fresh_observations() {
    assert_eq!(
      price(&[observation(100_000_000_000, 0, 0)], 31, 31_000_000),
      Err(CoreError::OracleNoFreshPriceObservations),
    );
    assert_eq!(
      price(&[PriceObservation::default()], 1, 1_000_000),
      Err(CoreError::OracleNoFreshPriceObservations),
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
  fn non_regular_observations_band_against_fresh_regular_ones() {
    let regular = PriceObservation {
      price: 10_000,
      exponent: -2,
      confidence: 1,
      posted_slot: 100,
      posted_timestamp_us: 100_000_000,
      market_session: MarketSession::Regular,
    };
    let closed = PriceObservation {
      price: 105_000,
      exponent: -3,
      confidence: 1,
      posted_slot: 1,
      posted_timestamp_us: 1,
      market_session: MarketSession::Closed,
    };
    assert_eq!(
      price(&[regular, closed], 100, 100_000_000),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(102_500_000_000),
        conf: UFix64::<N9>::new(10_000_000),
      }),
    );
  }

  #[test]
  fn non_regular_observations_outside_the_band_are_excluded() {
    let regular = observation(100_000_000_000, 100, 100_000_000);
    let mut closed = observation(110_000_000_000, 1, 1);
    closed.market_session = MarketSession::Closed;
    assert_eq!(
      price(&[regular, closed], 100, 100_000_000),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(100_000_000_000),
        conf: UFix64::zero(),
      }),
    );
  }

  #[test]
  fn non_regular_observations_need_a_fresh_regular_reference() {
    let stale_regular = observation(100_000_000_000, 0, 0);
    let mut closed = observation(100_000_000_000, 1, 1);
    closed.market_session = MarketSession::Closed;
    assert_eq!(
      price(&[stale_regular, closed], 100, 100_000_000),
      Err(CoreError::OracleNoFreshPriceObservations),
    );
  }
}
