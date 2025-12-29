use anchor_lang::prelude::Pubkey;

mod lst_provider;
mod protocol_state_quote_strategy;
mod quote_strategy;
mod simulation_quote_strategy;

pub use hylo_clients::util::LST;
pub(crate) use lst_provider::LstProvider;
pub use protocol_state_quote_strategy::ProtocolStateQuoteStrategy;
pub use quote_strategy::QuoteStrategy;
pub use simulation_quote_strategy::SimulationQuoteStrategy;

/// Quote amounts computed from the protocol state
#[derive(Clone, Debug)]
pub struct QuoteAmounts {
  /// Amount of input tokens (in base units) - matches the `amount_in` field
  /// from the quote request
  pub amount_in: u64,

  /// Amount of output tokens (in base units)
  pub amount_out: u64,

  /// Fee amount (in input token base units)
  pub fee_amount: u64,

  /// Fee mint (which token the fee is denominated in)
  pub fee_mint: Pubkey,
}

// /// Executable quote with instructions and compute units, ready for signing
// pub struct ExecutableQuote {
//   /// Quote amounts
//   pub amounts: QuoteAmounts,

//   /// Compute units required
//   pub compute_units: u64,

// Compute unit strategy
// pub compute_unit_strategy: ComputeUnitStrategy,

//   /// Transaction instructions ready for signing
//   pub instructions: Vec<Instruction>,
// }
