//! Quote builder that constructs quotes with instructions

use anchor_lang::prelude::{Clock, Pubkey};
use async_trait::async_trait;
use hylo_clients::protocol_state::{ProtocolState, StateProvider};
use hylo_idl::tokens::TokenMint;

use crate::instruction_builder::build_instructions;
use crate::{
  ComputeUnitMethod, ComputeUnitProvider, ExecutableQuote,
  HyloComputeUnitProvider, HyloInstructionBuilder, HyloQuoteComputer,
  InstructionBuilder, QuoteComputer, QuoteStrategy, SupportedPair,
};

/// Builds quotes with transaction instructions
pub struct QuoteBuilder<S: StateProvider> {
  state_provider: S,
  computer: HyloQuoteComputer,
}

impl<S: StateProvider> QuoteBuilder<S> {
  pub fn new(state_provider: S) -> Self {
    Self {
      state_provider,
      computer: HyloQuoteComputer::new(),
    }
  }

  /// Build a quote with instructions and estimated compute units
  ///
  /// # Errors
  /// Returns error if state fetch, quote computation, or instruction building
  /// fails.
  pub async fn build_quote<IN: TokenMint, OUT: TokenMint>(
    &self,
    amount: u64,
    user_wallet: Pubkey,
    slippage_bps: u16,
  ) -> anyhow::Result<ExecutableQuote>
  where
    (IN, OUT): SupportedPair<IN, OUT>,
    HyloQuoteComputer: QuoteComputer<IN, OUT, Clock>,
    HyloComputeUnitProvider: ComputeUnitProvider<IN, OUT>,
    HyloInstructionBuilder: InstructionBuilder<IN, OUT>,
  {
    let state: ProtocolState<Clock> = self.state_provider.fetch_state().await?;
    let quote_amounts = QuoteComputer::<IN, OUT, Clock>::compute_quote(
      &self.computer,
      &state,
      amount,
    )?;

    let instructions =
      build_instructions::<IN, OUT>(&quote_amounts, user_wallet, slippage_bps);

    let (compute_units, compute_units_safe) =
      <HyloComputeUnitProvider as ComputeUnitProvider<IN, OUT>>::default_compute_units();

    Ok(ExecutableQuote {
      amounts: quote_amounts,
      compute_units,
      compute_units_safe,
      instructions,
      compute_unit_method: ComputeUnitMethod::Estimated,
    })
  }
}

#[async_trait]
impl<S: StateProvider> QuoteStrategy for QuoteBuilder<S> {
  async fn fetch_quote<IN: TokenMint, OUT: TokenMint>(
    &self,
    amount: u64,
    user_wallet: Pubkey,
    slippage_bps: u16,
  ) -> anyhow::Result<ExecutableQuote>
  where
    (IN, OUT): SupportedPair<IN, OUT>,
    HyloQuoteComputer: QuoteComputer<IN, OUT, Clock>,
    HyloComputeUnitProvider: ComputeUnitProvider<IN, OUT>,
    HyloInstructionBuilder: InstructionBuilder<IN, OUT>,
  {
    self
      .build_quote::<IN, OUT>(amount, user_wallet, slippage_bps)
      .await
  }
}
