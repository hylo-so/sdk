//! Helper functions for cleaner static dispatch syntax.
//!
//! These functions provide a more readable alternative to fully qualified
//! trait syntax while maintaining the same compile-time guarantees.
//!
//! # Example
//!
//! Instead of:
//! ```rust,no_run
//! <S as QuoteStrategy<JITOSOL, HYUSD, Clock>>::get_quote(
//!   &self.strategy,
//!   amount,
//!   user,
//!   slippage_tolerance,
//! )
//! .await
//! ```
//!
//! You can write:
//! ```rust,no_run
//! get_quote::<S, JITOSOL, HYUSD, Clock>(
//!   &self.strategy,
//!   amount,
//!   user,
//!   slippage_tolerance,
//! )
//! .await
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

/// Helper function for cleaner QuoteStrategy calls.
///
/// # Example
///
/// ```rust,no_run
/// use hylo_quotes::syntax_helpers::get_quote;
/// use hylo_quotes::QuoteStrategy;
/// use hylo_idl::tokens::{JITOSOL, HYUSD};
///
/// // Instead of:
/// // <S as QuoteStrategy<JITOSOL, HYUSD, Clock>>::get_quote(...)
/// // You write:
/// get_quote::<S, JITOSOL, HYUSD, Clock>(strategy, amount, user, slippage).await
/// ```
pub async fn get_quote<Strategy, IN, OUT, C>(
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
  <Strategy as QuoteStrategy<IN, OUT, C>>::get_quote(strategy, amount, user, slippage_tolerance).await
}

/// Helper function for building instructions.
///
/// # Example
///
/// ```rust,no_run
/// use hylo_quotes::syntax_helpers::build_instructions;
/// use hylo_clients::instructions::{InstructionBuilder, StabilityPoolInstructionBuilder};
/// use hylo_idl::tokens::{HYUSD, SHYUSD};
///
/// // Instead of:
/// // <StabilityPoolInstructionBuilder as InstructionBuilder<HYUSD, SHYUSD>>::build_instructions(...)
/// // You write:
/// build_instructions::<StabilityPoolInstructionBuilder, HYUSD, SHYUSD>(args)?
/// ```
pub fn build_instructions<Builder, IN, OUT>(
  inputs: <Builder as InstructionBuilder<IN, OUT>>::Inputs,
) -> anyhow::Result<Vec<Instruction>>
where
  Builder: InstructionBuilder<IN, OUT>,
  IN: TokenMint,
  OUT: TokenMint,
{
  <Builder as InstructionBuilder<IN, OUT>>::build_instructions(inputs)
}

/// Helper function for getting lookup tables.
///
/// # Example
///
/// ```rust,no_run
/// use hylo_quotes::syntax_helpers::lookup_tables;
/// use hylo_clients::instructions::{InstructionBuilder, StabilityPoolInstructionBuilder};
/// use hylo_idl::tokens::{HYUSD, SHYUSD};
///
/// // Instead of:
/// // <StabilityPoolInstructionBuilder as InstructionBuilder<HYUSD, SHYUSD>>::REQUIRED_LOOKUP_TABLES
/// // You write:
/// lookup_tables::<StabilityPoolInstructionBuilder, HYUSD, SHYUSD>()
/// ```
pub fn lookup_tables<Builder, IN, OUT>() -> &'static [Pubkey]
where
  Builder: InstructionBuilder<IN, OUT>,
  IN: TokenMint,
  OUT: TokenMint,
{
  <Builder as InstructionBuilder<IN, OUT>>::REQUIRED_LOOKUP_TABLES
}

/// Helper function for simulating events with compute units.
///
/// # Example
///
/// ```rust,no_run
/// use hylo_quotes::syntax_helpers::simulate_event_with_cus;
/// use hylo_clients::prelude::{ExchangeClient, SimulatePrice};
/// use hylo_idl::tokens::{JITOSOL, HYUSD};
///
/// // Instead of:
/// // <ExchangeClient as SimulatePrice<JITOSOL, HYUSD>>::simulate_event_with_cus(...)
/// // You write:
/// simulate_event_with_cus::<ExchangeClient, JITOSOL, HYUSD>(client, user, args).await
/// ```
pub async fn simulate_event_with_cus<Client, I, O>(
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
  <Client as SimulatePrice<I, O>>::simulate_event_with_cus(client, user, inputs).await
}

