use anchor_lang::prelude::Pubkey;
use async_trait::async_trait;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::TokenMint;

use crate::{QuotablePair, QuoteAmounts};

#[async_trait]
pub trait QuoteStrategy<C: SolanaClock> {
  async fn fetch_quote_amounts<IN: TokenMint, OUT: TokenMint>(
    &self,
    amount_in: u64,
    user_wallet: Pubkey,
  ) -> anyhow::Result<QuoteAmounts>
  where
    (IN, OUT): QuotablePair<IN, OUT, C>;
}
