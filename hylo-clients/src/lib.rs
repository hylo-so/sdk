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
//! // Mint JITOSOL → hyUSD
//! let user = Pubkey::new_unique();
//! let signature = client.run_transaction::<JITOSOL, HYUSD>(MintArgs {
//!     amount: UFix64::one(),
//!     user,
//!     slippage_config: None,
//! }).await?;
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
pub mod instructions;
pub mod prelude;
pub mod program_client;
pub mod stability_pool_client;
pub mod syntax_helpers;
pub mod transaction;
pub mod util;
