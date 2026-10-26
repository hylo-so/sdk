use anyhow::Result;
use hylo_core::error::CoreError;
use hylo_idl::oracle::accounts::PriceAggregate;

/// Converts the newest sample time to seconds.
///
/// # Errors
/// * The aggregate has no samples
/// * The time does not fit in `i64`
pub(crate) fn latest_timestamp_secs(aggregate: &PriceAggregate) -> Result<i64> {
  aggregate
    .samples
    .iter()
    .map(|sample| sample.posted_timestamp_us)
    .max()
    .and_then(|timestamp_us| timestamp_us.checked_div(1_000_000))
    .and_then(|timestamp_secs| i64::try_from(timestamp_secs).ok())
    .ok_or(CoreError::PythOracleOutdated.into())
}
