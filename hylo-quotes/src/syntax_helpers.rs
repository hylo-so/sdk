//! Helper functions for cleaner static dispatch syntax.
//!
//! These functions provide a more readable alternative to fully qualified
//! trait syntax while maintaining the same compile-time guarantees.
//!
//! # Example
//!
//! Instead of:
//! ```rust,no_run
//! <S as QuoteStrategy<JITOSOL, HYUSD, Clock>>::get_quote(&strategy, amount, user, slippage_tolerance).await
//! ```
//!
//! You can write:
//! ```rust,no_run
//! get_quote::<S, JITOSOL, HYUSD, Clock>(&strategy, amount, user, slippage).await
//! ```

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_lang::prelude::Pubkey;
use hylo_clients::instructions::InstructionBuilder;
use hylo_clients::prelude::SimulatePrice;
use hylo_clients::transaction::{BuildTransactionData, QuoteInput};
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::TokenMint;

use crate::quote_strategy::QuoteStrategy;
use crate::Quote;

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

pub(crate) fn build_instructions<Builder, IN, OUT>(
  inputs: <Builder as InstructionBuilder<IN, OUT>>::Inputs,
) -> anyhow::Result<Vec<Instruction>>
where
  Builder: InstructionBuilder<IN, OUT>,
  IN: TokenMint,
  OUT: TokenMint,
{
  <Builder as InstructionBuilder<IN, OUT>>::build_instructions(inputs)
}

pub(crate) fn lookup_tables<Builder, IN, OUT>() -> &'static [Pubkey]
where
  Builder: InstructionBuilder<IN, OUT>,
  IN: TokenMint,
  OUT: TokenMint,
{
  <Builder as InstructionBuilder<IN, OUT>>::REQUIRED_LOOKUP_TABLES
}

pub(crate) async fn simulate_event_with_cus<Client, I, O>(
  client: &Client,
  user: Pubkey,
  inputs: <Client as BuildTransactionData<I, O>>::Inputs,
) -> anyhow::Result<(<Client as SimulatePrice<I, O>>::Event, Option<u64>)>
where
  Client: SimulatePrice<I, O> + Send + Sync,
  <Client as BuildTransactionData<I, O>>::Inputs: QuoteInput,
  I: TokenMint,
  O: TokenMint,
{
  <Client as SimulatePrice<I, O>>::simulate_event_with_cus(client, user, inputs)
    .await
}
