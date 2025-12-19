//! Quote strategy trait

use anchor_lang::prelude::{Clock, Pubkey};
use async_trait::async_trait;
use hylo_idl::tokens::TokenMint;

use crate::{
  ComputeUnitProvider, ExecutableQuote, HyloComputeUnitProvider,
  HyloInstructionBuilder, HyloQuoteComputer, InstructionBuilder, QuoteComputer,
  SupportedPair,
};

/// Trait for strategies that can fetch quotes
#[async_trait]
pub trait QuoteStrategy: Send + Sync {
  /// Fetch a quote for a token pair
  ///
  /// # Errors
  /// Returns error if quote computation or instruction building fails.
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
    HyloInstructionBuilder: InstructionBuilder<IN, OUT>;
}
