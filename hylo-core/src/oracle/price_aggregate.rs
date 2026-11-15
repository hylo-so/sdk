use anchor_lang::prelude::{
  borsh, Account, AccountDeserialize, AccountSerialize, AnchorDeserialize,
  AnchorSerialize, InitSpace,
};
use fix::prelude::{
  CheckedAdd, CheckedDivFix, CheckedSub, MulDiv, UFix128, UFix64, N2, N4, N9,
};
use fix::typenum::{Integer, Z0};

use super::config::MAX_CONF_TOLERANCE;
use super::{OracleConfig, OraclePrice, PythLazerSlot, PythLazerSlots};
use crate::error::CoreError;
use crate::error::CoreError::{OracleNegativeTime, OraclePriceRange};
use crate::solana_clock::SolanaClock;
use crate::util::normalize_exp;

pub const DEFAULT_MAX_STALENESS_SLOTS: u64 = 30;
pub const DEFAULT_MAX_STALENESS_SECS: u64 = 5;
pub const MICROS_PER_SECOND: u64 = 1_000_000;
pub const DEFAULT_MAX_STALENESS_SECS_US: u64 =
  DEFAULT_MAX_STALENESS_SECS * MICROS_PER_SECOND;
pub const DEFAULT_MAX_PRICE_BAND_BPS: u16 = 500;
pub const MIN_EXPONENT: i16 = -9;
pub const MAX_EXPONENT: i16 = -2;

/// Methods for reading prices from a price aggregate account.
pub trait PriceAggregateExt {
  /// Returns the Pyth Lazer slots stored in the aggregate.
  fn pyth_lazer_slots(&self) -> PythLazerSlots;

  /// Returns every occupied Pyth Lazer slot in the aggregate.
  fn occupied_slots(&self) -> impl Iterator<Item = PythLazerSlot> {
    self.pyth_lazer_slots().0.into_iter().flatten()
  }

  /// Fetches the validated aggregate price.
  ///
  /// # Errors
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
      self.occupied_slots(),
      AggregationMethod::SimpleAverage,
      clock.slot(),
      current_timestamp_us,
      validation_config,
    )
  }

  /// Converts the newest posted timestamp to seconds.
  ///
  /// # Errors
  /// * The aggregate has no occupied slots.
  /// * The timestamp does not fit in `i64`.
  fn latest_timestamp_secs(&self) -> Result<i64, CoreError> {
    self
      .occupied_slots()
      .map(|slot| slot.observation.posted_timestamp_us)
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
  fn pyth_lazer_slots(&self) -> PythLazerSlots {
    self.pyth_lazer_slots.into()
  }
}

impl<T: PriceAggregateExt + ?Sized> PriceAggregateExt for Box<T> {
  fn pyth_lazer_slots(&self) -> PythLazerSlots {
    T::pyth_lazer_slots(self)
  }
}

impl<T> PriceAggregateExt for Account<'_, T>
where
  T: PriceAggregateExt + AccountSerialize + AccountDeserialize + Clone,
{
  fn pyth_lazer_slots(&self) -> PythLazerSlots {
    T::pyth_lazer_slots(self)
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

/// Source observation with price and confidence in the feed exponent.
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

impl PythLazerSlot {
  fn normalize(&self) -> Result<NormalizedObservation, CoreError> {
    Ok(NormalizedObservation {
      price: PriceObservation::normalized(
        self.observation.price,
        self.exponent,
      )?,
      confidence: PriceObservation::normalized(
        self.observation.confidence,
        self.exponent,
      )?,
    })
  }

}

impl NormalizedObservation {
  fn validate_confidence(
    self,
    max_confidence: Option<UFix64<N9>>,
  ) -> Result<NormalizedObservation, CoreError> {
    let confidence_ratio = self
      .confidence
      .mul_div_floor(UFix64::one(), self.price)
      .ok_or(CoreError::OracleInvalidPrice)?;

    (confidence_ratio <= max_confidence.unwrap_or(MAX_CONF_TOLERANCE))
      .then_some(self)
      .ok_or(CoreError::OracleConfidence)
  }

  fn validate_price_band(
    self,
    reference_price: UFix64<N9>,
    price_band_bps: Option<u16>,
  ) -> Result<NormalizedObservation, CoreError> {
    let price_band = UFix64::<N4>::new(u64::from(
      price_band_bps.unwrap_or(DEFAULT_MAX_PRICE_BAND_BPS),
    ));
    let limit = reference_price
      .mul_div_floor(price_band, UFix64::<N4>::one())
      .ok_or(CoreError::OracleArithmetic)?;

    (self.price.abs_diff(&reference_price) <= limit)
      .then_some(self)
      .ok_or(CoreError::OraclePriceBandExceeded)
  }
}

impl PriceObservation {
  fn normalized(value: u64, exponent: i16) -> Result<UFix64<N9>, CoreError> {
    PriceObservation::validate_exponent(exponent)?;

    exponent
      .checked_neg()
      .and_then(|decimals| u8::try_from(decimals).ok())
      .and_then(|decimals| normalize_exp(decimals, value))
      .ok_or(CoreError::OracleArithmetic)
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

/// Combines fresh slot observations into a 9-decimal spot price with the
/// widest confidence among them.
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
  slots: impl Iterator<Item = PythLazerSlot>,
  aggregation_method: AggregationMethod,
  current_slot: u64,
  current_timestamp_us: u64,
  validation_config: ValidationConfig,
) -> Result<OraclePrice, CoreError> {
  let slots: Vec<PythLazerSlot> = slots.collect();
  let regular = slots
    .iter()
    .filter(|slot| slot.observation.market_session == MarketSession::Regular)
    .filter_map(|slot| {
      fresh(slot, current_slot, current_timestamp_us, validation_config)
        .transpose()
    })
    .collect::<Result<Vec<_>, CoreError>>()?;
  let reference_price = combine(aggregation_method, regular.iter().copied())
    .ok()
    .map(|price| price.spot);
  let banded = slots
    .iter()
    .filter(|slot| slot.observation.market_session != MarketSession::Regular)
    .filter_map(|slot| {
      reference_price.and_then(|reference_price| {
        banded(
          slot,
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
  slot: &PythLazerSlot,
  current_slot: u64,
  current_timestamp_us: u64,
  validation_config: ValidationConfig,
) -> Result<Option<NormalizedObservation>, CoreError> {
  let observation = &slot.observation;
  accepted(
    observation
      .validate_price()
      .and_then(|()| {
        observation.validate_slot_age(current_slot, validation_config.slots)
      })
      .and_then(|()| {
        observation
          .validate_timestamp(current_timestamp_us, validation_config.secs)
      })
      .and_then(|()| slot.normalize())
      .and_then(|normalized| {
        normalized.validate_confidence(validation_config.confidence)
      }),
  )
}

fn banded(
  slot: &PythLazerSlot,
  reference_price: UFix64<N9>,
  max_confidence: Option<UFix64<N9>>,
  price_band_bps: Option<u16>,
) -> Result<Option<NormalizedObservation>, CoreError> {
  accepted(
    slot
      .observation
      .validate_price()
      .and_then(|()| slot.normalize())
      .and_then(|normalized| normalized.validate_confidence(max_confidence))
      .and_then(|normalized| {
        normalized.validate_price_band(reference_price, price_band_bps)
      }),
  )
}

fn accepted(
  validation: Result<NormalizedObservation, CoreError>,
) -> Result<Option<NormalizedObservation>, CoreError> {
  match validation {
    Ok(normalized) => Ok(Some(normalized)),
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
    PriceObservation, PythLazerSlot, ValidationConfig,
  };
  use crate::error::CoreError;
  use crate::oracle::PythLazerFeedId;

  fn slot(
    price: u64,
    posted_slot: u64,
    posted_timestamp_us: u64,
  ) -> PythLazerSlot {
    PythLazerSlot {
      feed_id: PythLazerFeedId(1),
      exponent: -9,
      observation: PriceObservation {
        price,
        confidence: 0,
        posted_slot,
        posted_timestamp_us,
        market_session: MarketSession::Regular,
      },
    }
  }

  fn price(
    slots: &[PythLazerSlot],
    current_slot: u64,
    current_timestamp_us: u64,
  ) -> Result<OraclePrice, CoreError> {
    aggregate_price(
      slots.iter().copied(),
      AggregationMethod::SimpleAverage,
      current_slot,
      current_timestamp_us,
      ValidationConfig::default(),
    )
  }

  #[test]
  fn price_averages_fresh_observations() {
    let slots = [
      slot(100_000_000_000, 1, 1_000_000),
      slot(200_000_000_000, 1, 1_000_000),
    ];
    assert_eq!(
      price(&slots, 1, 1_000_000),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(150_000_000_000),
        conf: UFix64::zero(),
      }),
    );
  }

  #[test]
  fn price_reports_widest_confidence() {
    let mut narrow = slot(100_000_000_000, 1, 1_000_000);
    narrow.observation.confidence = 1_000_000_000;
    let mut wide = slot(100_000_000_000, 1, 1_000_000);
    wide.observation.confidence = 2_000_000_000;
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
    let slots = [
      slot(100_000_000_000, 0, 0),
      slot(200_000_000_000, 31, 31_000_000),
    ];
    assert_eq!(
      price(&slots, 31, 31_000_000),
      Ok(OraclePrice {
        spot: UFix64::<N9>::new(200_000_000_000),
        conf: UFix64::zero(),
      }),
    );
  }

  #[test]
  fn price_fails_without_fresh_observations() {
    assert_eq!(
      price(&[slot(100_000_000_000, 0, 0)], 31, 31_000_000),
      Err(CoreError::OracleNoFreshPriceObservations),
    );
    assert_eq!(
      price(&[slot(0, 1, 1_000_000)], 1, 1_000_000),
      Err(CoreError::OracleNoFreshPriceObservations),
    );
  }

  #[test]
  fn confidence_limit_uses_price_units() {
    let mut slot = slot(100_000_000_000, 0, 0);
    slot.observation.confidence = 5_010_000_000;

    assert_eq!(
      slot
        .normalize()
        .and_then(|normalized| normalized.validate_confidence(None)),
      Err(CoreError::OracleConfidence),
    );
  }

  #[test]
  fn non_regular_observations_band_against_fresh_regular_ones() {
    let mut regular = slot(10_000, 100, 100_000_000);
    regular.exponent = -2;
    regular.observation.confidence = 1;
    let mut closed = slot(105_000, 1, 1);
    closed.exponent = -3;
    closed.observation.confidence = 1;
    closed.observation.market_session = MarketSession::Closed;
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
    let regular = slot(100_000_000_000, 100, 100_000_000);
    let mut closed = slot(110_000_000_000, 1, 1);
    closed.observation.market_session = MarketSession::Closed;
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
    let stale_regular = slot(100_000_000_000, 0, 0);
    let mut closed = slot(100_000_000_000, 1, 1);
    closed.observation.market_session = MarketSession::Closed;
    assert_eq!(
      price(&[stale_regular, closed], 100, 100_000_000),
      Err(CoreError::OracleNoFreshPriceObservations),
    );
  }
}
