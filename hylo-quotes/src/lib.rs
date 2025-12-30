use anchor_client::solana_sdk::instruction::Instruction;
use anchor_lang::prelude::Pubkey;

mod lst_provider;
mod protocol_state_strategy;
mod quote_metadata;
mod quote_provider;
mod quote_strategy;
mod simulation_strategy;

pub use hylo_clients::util::LST;
pub(crate) use lst_provider::LstProvider;
pub use protocol_state_strategy::ProtocolStateStrategy;
pub use quote_metadata::{Operation, QuoteMetadata};
pub use quote_provider::QuoteProvider;
pub use quote_strategy::QuoteStrategy;
pub use simulation_strategy::SimulationStrategy;

const MAX_COMPUTE_UNITS: u64 = 1_400_000;

/// Quote amounts computed from the protocol state
#[derive(Clone, Debug)]
pub struct Quote {
  /// Amount of input tokens (in base units)
  pub amount_in: u64,

  /// Amount of output tokens (in base units)
  pub amount_out: u64,

  /// Compute units required
  pub compute_units: u64,

  /// Compute unit strategy
  pub compute_unit_strategy: ComputeUnitStrategy,

  /// Fee amount (in input token base units)
  pub fee_amount: u64,

  /// Fee mint (which token the fee is denominated in)
  pub fee_mint: Pubkey,

  /// Transaction instructions ready for signing
  pub instructions: Vec<Instruction>,

  /// Address lookup table pubkeys required for this transaction
  pub address_lookup_tables: Vec<Pubkey>,
}

#[derive(Clone, Debug)]
pub enum ComputeUnitStrategy {
  /// Esimated compute units based on historical data
  Estimated,

  /// Compute units returned from simulation results
  Simulated,
}
