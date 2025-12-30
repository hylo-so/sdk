//! Type-safe quote computation and transaction building for the Hylo protocol.
//!
//! Provides strategies for computing exchange rates, building Solana instructions,
//! and estimating compute units using either protocol state or transaction simulation.

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
