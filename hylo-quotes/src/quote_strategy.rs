use anchor_lang::prelude::Pubkey;
use async_trait::async_trait;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::TokenMint;

use crate::QuoteAmounts;

#[async_trait]
pub trait QuoteStrategy<IN: TokenMint, OUT: TokenMint, C: SolanaClock> {
  async fn fetch_quote_amounts(
    &self,
    amount_in: u64,
    user: Pubkey,
  ) -> anyhow::Result<QuoteAmounts>;
}
