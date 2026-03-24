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
//!   Slower but validates that transactions would actually succeed (e.g.,
//!   checks wallet balances).
//!
//! # Examples
//!
//! ## Using `ProtocolStateStrategy`
//!
//! ```rust,no_run
//! use hylo_quotes::prelude::*;
//! use solana_rpc_client::nonblocking::rpc_client::RpcClient;
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
//!
//! let user = Pubkey::new_unique();
//! let amount_in = 1_000_000_000; // 1 JitoSOL (9 decimals)
//! let slippage_tolerance = 50; // 0.5%
//!
//! // Generates a tagged quote from runtime `Pubkeys`
//! let (quote, metadata) = strategy
//!   .runtime_quote_with_metadata(JITOSOL::MINT, HYUSD::MINT, amount_in, user, slippage_tolerance)
//!   .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using `SimulationStrategy`
//!
//! ```rust,no_run
//! use hylo_clients::prelude::*;
//! use hylo_quotes::prelude::*;
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
//!
//! let user = Pubkey::new_unique();
//! let amount_in = 1_000_000_000; // 1 JitoSOL (9 decimals)
//! let slippage_tolerance = 50; // 0.5%
//!
//! // SimulationStrategy validates balances, so if we get a quote, the transaction would succeed
//! let (quote, metadata) = strategy
//!   .runtime_quote_with_metadata(JITOSOL::MINT, HYUSD::MINT, amount_in, user, slippage_tolerance)
//!   .await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Low-level output with `TokenOperationExt`
//!
//! For direct access to protocol math without transaction building, use
//! [`token_operation::TokenOperationExt`]. The `output` method provides
//! turbofish syntax for specifying token pairs:
//!
//! ```rust,ignore
//! use hylo_quotes::prelude::*;
//! use solana_rpc_client::nonblocking::rpc_client::RpcClient;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".into()));
//! let provider = RpcStateProvider::new(rpc_client);
//! let state = provider.fetch_state().await?;
//!
//! let amount_in = UFix64::new(1_000_000_000); // 1 JITOSOL
//! let output = state.output::<JITOSOL, HYUSD>(amount_in)?;
//! # Ok(())
//! # }
//! ```

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_lang::prelude::Pubkey;
use fix::prelude::{UFix64, UFixValue64};
use fix::typenum::Integer;
use hylo_idl::tokens::{HYLOSOL, JITOSOL};

pub mod prelude;
pub mod protocol_state;
mod protocol_state_strategy;
mod quote_metadata;
mod quote_strategy;
mod runtime_quote_strategy;
pub mod simulated_operation;
mod simulation_strategy;
pub mod token_operation;

pub use hylo_clients::util::LST;
pub use protocol_state_strategy::ProtocolStateStrategy;
pub use quote_metadata::{Operation, QuoteMetadata};
pub use quote_strategy::QuoteStrategy;
pub use runtime_quote_strategy::RuntimeQuoteStrategy;
pub use simulated_operation::ComputeUnitInfo;
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
pub const DEFAULT_CUS_WITH_BUFFER_X3: u64 = 300_000;

/// Typed executable quote with amounts, instructions, and compute units.
#[derive(Clone, Debug)]
pub struct ExecutableQuote<InExp: Integer, OutExp: Integer, FeeExp: Integer> {
  pub amount_in: UFix64<InExp>,
  pub amount_out: UFix64<OutExp>,
  pub compute_units: u64,
  pub compute_unit_strategy: ComputeUnitStrategy,
  pub fee_amount: UFix64<FeeExp>,
  pub fee_mint: Pubkey,
  pub instructions: Vec<Instruction>,
  pub address_lookup_tables: Vec<Pubkey>,
}

/// Executable quote with runtime exponent information.
#[derive(Clone, Debug)]
pub struct ExecutableQuoteValue {
  pub amount_in: UFixValue64,
  pub amount_out: UFixValue64,
  pub compute_units: u64,
  pub compute_unit_strategy: ComputeUnitStrategy,
  pub fee_amount: UFixValue64,
  pub fee_mint: Pubkey,
  pub instructions: Vec<Instruction>,
  pub address_lookup_tables: Vec<Pubkey>,
}

impl<InExp: Integer, OutExp: Integer, FeeExp: Integer>
  From<ExecutableQuote<InExp, OutExp, FeeExp>> for ExecutableQuoteValue
{
  fn from(
    quote: ExecutableQuote<InExp, OutExp, FeeExp>,
  ) -> ExecutableQuoteValue {
    ExecutableQuoteValue {
      amount_in: quote.amount_in.into(),
      amount_out: quote.amount_out.into(),
      compute_units: quote.compute_units,
      compute_unit_strategy: quote.compute_unit_strategy,
      fee_amount: quote.fee_amount.into(),
      fee_mint: quote.fee_mint,
      instructions: quote.instructions,
      address_lookup_tables: quote.address_lookup_tables,
    }
  }
}

#[derive(Clone, Debug)]
pub enum ComputeUnitStrategy {
  /// Estimated compute units based on historical data
  Estimated,
  /// Compute units returned from simulation results
  Simulated,
}

/// This crate builds on [`hylo_clients::util::LST`] in core traits like
/// [`QuoteStrategy<L, OUT>`].
///
/// The [`Local`] marker allows us to use [`LST`] in trait bound position while
/// telling the compiler that changes in `hylo-clients` won't affect local
/// impls.
pub(crate) trait Local {}
impl Local for JITOSOL {}
impl Local for HYLOSOL {}
