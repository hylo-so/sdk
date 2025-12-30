//! Helper functions for cleaner static dispatch syntax.

use anchor_lang::prelude::Pubkey;
// Re-export helpers from hylo-clients for convenience
pub(crate) use hylo_clients::syntax_helpers::{
  build_instructions, lookup_tables, simulate_event_with_cus,
};
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::TokenMint;

use crate::quote_strategy::QuoteStrategy;
use crate::Quote;

/// Helper for `QuoteStrategy` calls with cleaner syntax.
pub(crate) async fn get_quote<Strategy, IN, OUT, C>(
  strategy: &Strategy,
  amount: u64,
  user: Pubkey,
  slippage_tolerance: u64,
) -> anyhow::Result<Quote>
where
  Strategy: QuoteStrategy<IN, OUT, C>,
  IN: TokenMint,
  OUT: TokenMint,
  C: SolanaClock,
{
  <Strategy as QuoteStrategy<IN, OUT, C>>::get_quote(
    strategy,
    amount,
    user,
    slippage_tolerance,
  )
  .await
}
