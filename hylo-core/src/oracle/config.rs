use fix::prelude::{UFix64, UFixValue64, N9};

use crate::error::CoreError;
use crate::error::CoreError::{
  OracleConfToleranceInvalid, OracleIntervalSecsInvalid, OracleNegativeTime,
  OracleOutdated,
};

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
      Err(OracleNegativeTime)
    }?;
  if publish_time.saturating_add(oracle_interval) >= clock_time {
    Ok(())
  } else {
    Err(OracleOutdated)
  }
}
