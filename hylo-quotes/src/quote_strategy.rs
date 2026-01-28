use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::typenum::Integer;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::TokenMint;

use crate::ExecutableQuote;

/// Trait for strategies that compute quotes for token pair operations.
#[async_trait]
pub trait QuoteStrategy<IN: TokenMint, OUT: TokenMint, C: SolanaClock> {
  type FeeExp: Integer;

  /// Compute a quote for the token pair operation.
  ///
  /// # Errors
  /// Returns error if quote computation fails.
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<ExecutableQuote<IN::Exp, OUT::Exp, Self::FeeExp>>;
}
