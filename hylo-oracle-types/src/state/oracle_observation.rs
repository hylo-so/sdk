use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

use crate::feeds::OracleSource;

const_assert_eq!(OracleObservation::INIT_SPACE, 108);

/// Normalized latest price for one feed; facts within bounds. The
/// price/exponent/confidence layout mirrors Pyth's `PriceFeedMessage` field for
/// field (raw ints sharing one exponent) rather than `UFixValue64`, because
/// this is a source-neutral facts record; typed conversion is the consumer's
/// job. Populated iff `price_timestamp_us != 0`. Consumers gate freshness on
/// `price_timestamp_us` (the price-determination time, not `posted_slot`),
/// confidence on `confidence`, and must pin the account by PDA address.
#[account]
#[derive(InitSpace)]
pub struct OracleObservation {
  pub bump: u8,
  pub source: OracleSource,
  pub price: i64,
  pub exponent: i16,
  pub confidence: u64,
  pub price_timestamp_us: u64,
  pub posted_slot: u64,
  pub posted_unix_timestamp: i64,
  _reserved: [u8; 64],
}

#[cfg(any(test, feature = "test-support"))]
impl Default for OracleObservation {
  fn default() -> Self {
    Self {
      bump: 0,
      source: OracleSource::PythLazer,
      price: 0,
      exponent: 0,
      confidence: 0,
      price_timestamp_us: 0,
      posted_slot: 0,
      posted_unix_timestamp: 0,
      _reserved: [0; 64],
    }
  }
}
