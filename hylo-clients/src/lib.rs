//! # Hylo Clients
//!
//! Offchain Rust clients for interacting with Hylo protocol programs on Solana.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use hylo_clients::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! // Create an exchange client
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
//! - [`ExchangeClient`] - Mint/redeem/swap operations for hyUSD and xSOL
//! - [`StabilityPoolClient`] - Deposit/withdraw operations for sHYUSD

pub mod exchange_client;
pub mod prelude;
pub mod program_client;
pub mod stability_pool_client;
pub mod transaction;
pub mod util;
