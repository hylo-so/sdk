//! # Hylo Stats
//!
//! Offchain yield statistics for the Hylo protocol.
//!
//! - [`client`] — Read-only fetch layer for earn pool stats (no keypair
//!   required)
//! - [`earn_pool_stats`] — Yield statistics computation for sHYUSD
//! - [`earn_pool_yield_math`] — Pure math for realized and projected earn pool
//!   yield
//! - [`types`] — Data types for stats inputs and results

pub mod client;
pub mod earn_pool_stats;
pub mod earn_pool_yield_math;
pub mod error;
pub mod types;
