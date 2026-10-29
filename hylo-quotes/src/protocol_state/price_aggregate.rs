use anyhow::Result;
use hylo_core::error::CoreError;
use hylo_core::oracle::PriceAggregateObservations;

/// Converts the newest observation time to seconds.
///
/// # Errors
/// * The aggregate has no observations
/// * The time does not fit in `i64`
pub(crate) fn latest_timestamp_secs(
  observations: &PriceAggregateObservations,
) -> Result<i64> {
  observations
    .iter()
    .map(|observation| observation.posted_timestamp_us)
    .max()
    .and_then(|timestamp_us| timestamp_us.checked_div(1_000_000))
    .and_then(|timestamp_secs| i64::try_from(timestamp_secs).ok())
    .ok_or(CoreError::OracleOutdated.into())
}
