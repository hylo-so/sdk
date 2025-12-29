// use anchor_lang::prelude::Pubkey;
// use anyhow::Result;
// use async_trait::async_trait;
// use hylo_clients::protocol_state::StateProvider;
// use hylo_idl::tokens::TokenMint;

// use crate::{QuotablePair, QuoteAmounts, QuoteStrategy};

// /// Quote strategy that computes quotes from protocol state
// pub struct ProtocolStateQuoteStrategy<S: StateProvider> {
//   state_provider: S,
// }

// impl<S: StateProvider> ProtocolStateQuoteStrategy<S> {
//   /// Create a new protocol state quote strategy
//   #[must_use]
//   pub fn new(state_provider: S) -> Self {
//     Self { state_provider }
//   }
// }

// impl<S: StateProvider> ProtocolStateQuoteStrategy<S> {
//   fn compute_quote_amounts<IN: TokenMint, OUT: TokenMint>(
//     state: &ProtocolState<Clock>,
//     amount_in: u64,
//   ) -> Result<QuoteAmounts>
//   where
//     HyloQuoteComputer: QuoteComputer<IN, OUT, Clock>,
//     (IN, OUT): QuotablePair<IN, OUT>,
//   {
//     let computer = HyloQuoteComputer::new();
//     computer.compute_quote(state, amount_in)
//   }
// }

// #[async_trait]
// impl<S: StateProvider> QuoteStrategy for ProtocolStateQuoteStrategy<S> {
//   async fn fetch_quote_amounts<IN: TokenMint, OUT: TokenMint>(
//     &self,
//     amount_in: u64,
//     _user_wallet: Pubkey,
//     _slippage_bps: u32,
//   ) -> Result<QuoteAmounts>
//   where
//     (IN, OUT): crate::QuotablePair<IN, OUT>,
//     HyloQuoteComputer: QuoteComputer<IN, OUT, Clock>,
//   {
//     let state = self.state_provider.fetch_state().await?;
//     Self::compute_quote_amounts(&state, amount_in)
//   }
// }
