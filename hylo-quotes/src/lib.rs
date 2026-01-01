//! Type-safe quote computation and transaction building for the Hylo protocol.
//!
//! Provides strategies for computing exchange rates, building Solana
//! instructions, and estimating compute units using either protocol state or
//! transaction simulation.
//!
//! # Strategies
//!
//! Two quote strategies are available:
//!
//! - **`ProtocolStateStrategy`**: Computes quotes using protocol state and SDK
//!   math. Fast and doesn't require transaction simulation, but doesn't check
//!   wallet balances.
//! - **`SimulationStrategy`**: Computes quotes by simulating transactions.
//!   Slower but validates that transactions would actually succeed (e.g., checks
//!   wallet balances).
//!
//! # Examples
//!
//! ## Using ProtocolStateStrategy
//!
//! ```rust,no_run
//! use hylo_clients::protocol_state::RpcStateProvider;
//! use hylo_quotes::{ProtocolStateStrategy, QuoteProvider};
//! use hylo_idl::tokens::{TokenMint, HYUSD, JITOSOL};
//! use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
//! use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
//! use anchor_lang::prelude::Pubkey;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let rpc_client = Arc::new(RpcClient::new_with_commitment(
//!   "https://api.mainnet-beta.solana.com".to_string(),
//!   CommitmentConfig::confirmed(),
//! ));
//! let state_provider = Arc::new(RpcStateProvider::new(rpc_client));
//!
//! let strategy = ProtocolStateStrategy::new(state_provider);
//! let provider = QuoteProvider::new(strategy);
//!
//! let user = Pubkey::new_unique();
//! let amount_in = 1_000_000_000; // 1 JitoSOL (9 decimals)
//! let slippage_tolerance = 50; // 0.5%
//!
//! let (quote, metadata) = provider
//!   .fetch_quote(JITOSOL::MINT, HYUSD::MINT, amount_in, user, slippage_tolerance)
//!   .await?;
//!
//! println!("Operation: {:?}", metadata.operation);
//! println!("Description: {}", metadata.description);
//! println!("Amount in: {}", quote.amount_in);
//! println!("Amount out: {}", quote.amount_out);
//! println!("Fee: {} (mint: {})", quote.fee_amount, quote.fee_mint);
//! println!("Compute units: {} ({:?})", quote.compute_units, quote.compute_unit_strategy);
//! # Ok(())
//! # }
//! ```
//!
//! ## Using SimulationStrategy
//!
//! ```rust,no_run
//! use hylo_clients::prelude::*;
//! use hylo_quotes::{QuoteProvider, SimulationStrategy};
//! use hylo_idl::tokens::{TokenMint, HYUSD, JITOSOL};
//! use anchor_lang::prelude::Pubkey;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let exchange_client = ExchangeClient::new_random_keypair(
//!   Cluster::Mainnet,
//!   CommitmentConfig::confirmed(),
//! )?;
//! let stability_pool_client = StabilityPoolClient::new_random_keypair(
//!   Cluster::Mainnet,
//!   CommitmentConfig::confirmed(),
//! )?;
//!
//! let strategy = SimulationStrategy::new(exchange_client, stability_pool_client);
//! let provider = QuoteProvider::new(strategy);
//!
//! let user = Pubkey::new_unique();
//! let amount_in = 1_000_000_000; // 1 JitoSOL (9 decimals)
//! let slippage_tolerance = 50; // 0.5%
//!
//! let (quote, metadata) = provider
//!   .fetch_quote(JITOSOL::MINT, HYUSD::MINT, amount_in, user, slippage_tolerance)
//!   .await?;
//!
//! // SimulationStrategy validates balances, so if we get a quote, the transaction would succeed
//! println!("Quote successful: {} hyUSD for {} JitoSOL", quote.amount_out, quote.amount_in);
//! # Ok(())
//! # }
//! ```

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_lang::prelude::Pubkey;

mod lst_provider;
mod protocol_state_strategy;
mod quote_metadata;
mod quote_provider;
mod quote_strategy;
mod simulation_strategy;
mod syntax_helpers;

pub use hylo_clients::util::LST;
pub(crate) use lst_provider::LstProvider;
pub use protocol_state_strategy::ProtocolStateStrategy;
pub use quote_metadata::{Operation, QuoteMetadata};
pub use quote_provider::QuoteProvider;
pub use quote_strategy::QuoteStrategy;
pub use simulation_strategy::SimulationStrategy;

/// Default buffered compute units for all exchange operations.
///
/// This is a buffered estimate (higher than measured values ~74k-97k CU) that
/// provides a safe default for all current quote operations. Measured values
/// came from calibration tool, but this value includes additional buffer for
/// safety across all operation types.
///
/// In the future, this could be replaced with per-instruction defaults based
/// on more comprehensive statistical analysis.
pub const DEFAULT_CUS_WITH_BUFFER: u64 = 100_000;

/// Quote with computed amounts, instructions, and compute units.
#[derive(Clone, Debug)]
pub struct Quote {
  pub amount_in: u64,
  pub amount_out: u64,
  pub compute_units: u64,
  pub compute_unit_strategy: ComputeUnitStrategy,
  pub fee_amount: u64,
  pub fee_mint: Pubkey,
  pub instructions: Vec<Instruction>,
  pub address_lookup_tables: Vec<Pubkey>,
}

#[derive(Clone, Debug)]
pub enum ComputeUnitStrategy {
  /// Estimated compute units based on historical data
  Estimated,

  /// Compute units returned from simulation results
  Simulated,
}
