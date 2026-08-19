//! Jupiter AMM integration for Hylo assets.
//!
//! [`HyloJupiterPair`] supports both directions of the following pairs:
//!
//! - `JITOSOL <-> HYUSD`
//! - `HYLOSOL <-> HYUSD`
//! - `JITOSOL <-> XSOL`
//! - `HYLOSOL <-> XSOL`
//! - `HYUSD <-> XSOL`
//! - `HYUSD <-> SHYUSD`
//! - `JITOSOL <-> HYLOSOL`
//! - `JITOSOL <-> USDC`
//! - `HYLOSOL <-> USDC`
//! - `USDC <-> HYUSD`
//! - `CBBTC <-> USDC`
//! - `CBBTC <-> HYUSD`
//! - `CBBTC <-> XBTC`
//! - `HYUSD <-> XBTC`
//! - `HYPE <-> USDC`
//! - `HYPE <-> HYUSD`
//! - `HYPE <-> XHYPE`
//! - `HYUSD <-> XHYPE`
//!
//! The pair implementations reject other mint combinations with an
//! `Invalid mint pair` error.

pub mod account_metas;
pub mod jupiter;
pub mod util;

pub use jupiter::{HyloJupiterPair, PairConfig};
