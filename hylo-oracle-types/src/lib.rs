//! Shared on-chain contract for hylo-oracle observations.
//!
//! Canonical source of the `OracleObservation` account layout, the `FEEDS`
//! registry, the observation PDA seed, and shared price bounds, so hylo-core,
//! the exchange, and the SDK read observations from one source of truth. Kept
//! lean (few deps) because both on-chain and off-chain consumers depend on it.

#![allow(clippy::missing_errors_doc)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::module_name_repetitions)]

pub mod constants;
pub mod feeds;
pub mod seeds;
pub mod state;

pub use constants::{MAX_EXPONENT, MICROS_PER_SECOND, MIN_EXPONENT};
pub use feeds::{
  ChainlinkDataStreamsFeedId, FeedConfig, OracleSource, PythCoreFeedId,
  PythLazerFeedId, BTC_USD_FEED_ID, FEEDS, LAZER_CHANNEL, SOL_USD_FEED_ID,
  USDC_USD_FEED_ID,
};
pub use seeds::OBSERVATION;
pub use state::OracleObservation;

// The hylo-oracle program that owns `OracleObservation` accounts. Present so
// the `#[account]`-generated `Owner` impl resolves `crate::ID`; consumers still
// pin observations by PDA address. Mirrors the oracle's main/shadow ids so a
// shadow-built consumer's owner check matches shadow-deployed observations.
#[cfg(not(feature = "shadow"))]
anchor_lang::declare_id!("hyorViHghDYU4Yd15DZFHDQXJBtzXqVHuypcvN3Gj7t");
#[cfg(feature = "shadow")]
anchor_lang::declare_id!("HySHy1kwAfZbPAfB3zutXgrBXsXQHdtqA1GJu24W4b9F");
