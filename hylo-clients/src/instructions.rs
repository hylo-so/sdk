//! Instruction building for Hylo protocol operations.
//!
//! [`RouterClient`] implements [`InstructionBuilder`] for all supported
//! token pairs, building router `Route` instructions that wrap the
//! appropriate exchange or stability pool account context.
//!
//! [`RouterClient`]: crate::router_client::RouterClient
//!
//! # Example
//!
//! ```rust,no_run
//! use hylo_clients::prelude::*;
//! use hylo_idl::tokens::{HYUSD, JITOSOL};
//!
//! # fn main() -> anyhow::Result<()> {
//! let user = Pubkey::new_unique();
//! let args = RouterArgs {
//!   amount: 1_000_000_000,
//!   user,
//!   slippage_config: None,
//! };
//!
//! let instructions =
//!   RouterClient::build_instructions::<JITOSOL, HYUSD>(args)?;
//! let luts =
//!   RouterClient::lookup_tables::<JITOSOL, HYUSD>();
//! # Ok(())
//! # }
//! ```

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use hylo_idl::tokens::TokenMint;

/// Statically type-safe instruction builder for token pair operations.
pub trait InstructionBuilder<IN: TokenMint, OUT: TokenMint> {
  type Inputs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey];

  /// Builds instructions for the token pair operation.
  ///
  /// # Errors
  /// Returns error if instruction building fails.
  fn build(inputs: Self::Inputs) -> Result<Vec<Instruction>>;
}
