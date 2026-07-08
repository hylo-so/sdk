//! # Hylo Stats
//!
//! Offchain yield statistics for the Hylo protocol.
//!
//! - [`earn_pool_stats`] — Read-only earn pool yield statistics for sHYUSD (no
//!   keypair required)
//! - [`earn_pool_yield_math`] — Pure math for realized and projected earn pool
//!   yield

pub mod earn_pool_stats;
pub mod earn_pool_yield_math;
pub mod error;
