//! # Hylo Clients
//!
//! Offchain clients for Hylo protocol transactions and quotes.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use hylo_clients::prelude::*;
//! use hylo_idl::tokens::{HYUSD, JITOSOL};
//!
//! # async fn example() -> Result<()> {
//! let client = RouterClient::new_random_keypair(
//!   Cluster::Mainnet,
//!   CommitmentConfig::confirmed(),
//! )?;
//!
//! // Mint JITOSOL -> hyUSD via router
//! let user = Pubkey::new_unique();
//! let signature = client.run_transaction::<JITOSOL, HYUSD>(
//!   RouterArgs { amount: 1_000_000_000, user, slippage_config: None },
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Clients
//!
//! - [`router_client::RouterClient`] — All user-facing token operations (mint,
//!   redeem, swap, stability pool) via the router program
//! - [`exchange_client::ExchangeClient`] — Admin operations for the exchange
//!   program
//! - [`stability_pool_client::StabilityPoolClient`] — Admin operations for the
//!   stability pool program

pub mod exchange_client;
pub mod prelude;
pub mod program_client;
pub mod router_client;
pub mod squads;
pub mod stability_pool_client;
pub mod transaction;
pub mod util;
