//! # Hylo Clients
//!
//! Offchain clients for Hylo protocol transactions and quotes.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use hylo_clients::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! // Create Hylo exchange client
//! let client = ExchangeClient::new_random_keypair(
//!     Cluster::Mainnet,
//!     CommitmentConfig::confirmed(),
//! )?;
//!
//! // Get a price quote for JITOSOL → hyUSD
//! let price = client.quote::<JITOSOL, HYUSD>().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Clients
//!
//! - [`exchange_client::ExchangeClient`] - Mint/redeem/swap operations for
//!   hyUSD and xSOL
//! - [`stability_pool_client::StabilityPoolClient`] - Deposit/withdraw
//!   operations for sHYUSD

pub mod exchange_client;
pub mod prelude;
pub mod program_client;
pub mod stability_pool_client;
pub mod transaction;
pub mod util;
