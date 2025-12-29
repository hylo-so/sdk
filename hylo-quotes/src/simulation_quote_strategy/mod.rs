use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use hylo_clients::prelude::{ExchangeClient, StabilityPoolClient};
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::TokenMint;

use crate::{QuotablePair, QuoteAmounts, QuoteStrategy};

#[allow(dead_code)]
pub struct SimulationQuoteStrategy {
  exchange_client: ExchangeClient,
  stability_pool_client: StabilityPoolClient,
}

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<C> for SimulationQuoteStrategy {
  async fn fetch_quote_amounts<IN: TokenMint, OUT: TokenMint>(
    &self,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts>
  where
    (IN, OUT): QuotablePair<IN, OUT, C>,
  {
    <(IN, OUT) as QuotablePair<IN, OUT, C>>::simulate_quote(
      &self.exchange_client,
      &self.stability_pool_client,
      amount_in,
      user,
    )
    .await
  }
}
