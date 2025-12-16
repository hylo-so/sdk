//! # Hylo Quotes
//!
//! Type-safe quote computation and transaction building for the Hylo protocol.
//! Computes token exchange rates, builds Solana instructions, and estimates
//! compute units.
//!
//! ## Architecture
//!
//! - `ExecutableQuote`: Final quote ready for signing (instructions + compute
//!   units)
//! - `QuoteBuilder`: Builds quotes with instructions (no RPC required)
//! - `QuoteProvider`: Matches mint pairs and fetches quotes via a
//!   `QuoteStrategy`, returns `(ExecutableQuote, QuoteMetadata)`
//! - `QuoteSimulator`: Simulates transactions to extract compute units
//! - `QuoteStrategy`: Trait for quote strategies (implemented by `QuoteBuilder`
//!   and `QuoteSimulator`)
//!
//! ## Quick Start
//!
//! ### Using `QuoteProvider` with `QuoteBuilder`
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
//! use anchor_lang::prelude::Pubkey;
//! use hylo_clients::prelude::CommitmentConfig;
//! use hylo_clients::protocol_state::RpcStateProvider;
//! use hylo_idl::tokens::{TokenMint, HYUSD, JITOSOL};
//! use hylo_quotes::{QuoteBuilder, QuoteProvider};
//!
//! # async fn example() -> anyhow::Result<()> {
//! # let rpc_client = Arc::new(RpcClient::new_with_commitment("".to_string(), CommitmentConfig::confirmed()));
//! let state_provider = RpcStateProvider::new(rpc_client);
//! let provider = QuoteProvider::new(QuoteBuilder::new(state_provider));
//! # let user = Pubkey::default();
//! let (quote, metadata) = provider
//!     .fetch_quote(JITOSOL::MINT, HYUSD::MINT, 1_000_000_000, user, 50)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Using `QuoteProvider` with `QuoteSimulator` (Production)
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
//! use anchor_lang::prelude::Pubkey;
//! use hylo_clients::prelude::CommitmentConfig;
//! use hylo_clients::protocol_state::RpcStateProvider;
//! use hylo_idl::tokens::{TokenMint, HYUSD, JITOSOL};
//! use hylo_quotes::{QuoteProvider, QuoteSimulator, SolanaRpcProvider};
//!
//! # async fn example() -> anyhow::Result<()> {
//! # let rpc_client = Arc::new(RpcClient::new_with_commitment("".to_string(), CommitmentConfig::confirmed()));
//! let state_provider = RpcStateProvider::new(rpc_client.clone());
//! let rpc_provider = SolanaRpcProvider::new(rpc_client);
//! let provider = QuoteProvider::new(QuoteSimulator::new(state_provider, rpc_provider));
//! # let user = Pubkey::default();
//! let (quote, metadata) = provider
//!     .fetch_quote(JITOSOL::MINT, HYUSD::MINT, 1_000_000_000, user, 50)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Direct `QuoteBuilder` Usage
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
//! use anchor_lang::prelude::Pubkey;
//! use hylo_clients::prelude::CommitmentConfig;
//! use hylo_clients::protocol_state::RpcStateProvider;
//! use hylo_idl::tokens::{TokenMint, HYUSD, JITOSOL};
//! use hylo_quotes::QuoteBuilder;
//!
//! # async fn example() -> anyhow::Result<()> {
//! # let rpc_client = Arc::new(RpcClient::new_with_commitment("".to_string(), CommitmentConfig::confirmed()));
//! let builder = QuoteBuilder::new(RpcStateProvider::new(rpc_client));
//! # let user = Pubkey::default();
//! let quote = builder
//!     .build_quote::<JITOSOL, HYUSD>(1_000_000_000, user, 50)
//!     .await?;
//! # Ok(())
//! # }
//! ```

mod instruction_builder;
mod quote_builder;
mod quote_computer;
mod quote_metadata;
mod quote_provider;
mod quote_simulator;
mod quote_strategy;
mod rpc;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
pub use quote_builder::QuoteBuilder;
pub use quote_metadata::{Operation, QuoteMetadata};
pub use quote_provider::QuoteProvider;
pub use quote_simulator::QuoteSimulator;
pub use quote_strategy::QuoteStrategy;
pub use rpc::{RpcProvider, SolanaRpcProvider};

/// Quote amounts computed from the protocol state
#[derive(Clone, Debug)]
pub struct QuoteAmounts {
  /// Amount of input tokens (in base units) - matches the `amount` field from
  /// the quote request
  pub amount_in: u64,

  /// Amount of output tokens (in base units)
  pub amount_out: u64,

  /// Fee amount (in input token base units)
  pub fee_amount: u64,

  /// Fee mint (which token the fee is denominated in)
  pub fee_mint: Pubkey,
}

/// Executable quote with instructions and compute units, ready for signing
///
/// Returned by `QuoteBuilder` and `QuoteSimulator`. When returned from
/// `QuoteProvider`, it is paired with `QuoteMetadata` as a tuple:
/// `(ExecutableQuote, QuoteMetadata)`.
pub struct ExecutableQuote {
  /// Quote amounts
  pub amounts: QuoteAmounts,

  /// Compute units required
  pub compute_units: u64,

  /// Compute units with safety buffer (1.5x)
  pub compute_units_safe: u64,

  /// How compute units were determined
  pub compute_unit_method: ComputeUnitMethod,

  /// Transaction instructions ready for signing
  pub instructions: Vec<Instruction>,
}

/// Method used to determine compute units
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComputeUnitMethod {
  /// Compute units were estimated using defaults
  Estimated,
  /// Compute units were determined via transaction simulation
  Simulated,
}
