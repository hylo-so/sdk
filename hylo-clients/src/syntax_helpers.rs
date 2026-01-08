//! Helper functions for cleaner static dispatch syntax.

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use hylo_idl::tokens::TokenMint;

use crate::instructions::InstructionBuilder;
use crate::transaction::{BuildTransactionData, QuoteInput, SimulatePrice};

/// Extension trait for ergonomic instruction builder method calls.
///
/// Provides `build_instructions` and `lookup_tables` methods that can be called
/// with turbofish syntax on any type implementing [`InstructionBuilder`].
///
/// # Example
///
/// ```rust,ignore
/// use hylo_clients::prelude::*;
///
/// let instructions = ExchangeInstructionBuilder::build_instructions::<JITOSOL, HYUSD>(args)?;
/// let luts = ExchangeInstructionBuilder::lookup_tables::<JITOSOL, HYUSD>();
/// ```
pub trait InstructionBuilderExt {
  /// Builds instructions for the token pair operation.
  ///
  /// # Errors
  /// Returns error if instruction building fails.
  fn build_instructions<IN, OUT>(
    inputs: <Self as InstructionBuilder<IN, OUT>>::Inputs,
  ) -> Result<Vec<Instruction>>
  where
    Self: InstructionBuilder<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint;

  /// Returns the required address lookup tables for the token pair operation.
  fn lookup_tables<IN, OUT>() -> &'static [Pubkey]
  where
    Self: InstructionBuilder<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint;
}

impl<X> InstructionBuilderExt for X {
  fn build_instructions<IN, OUT>(
    inputs: <Self as InstructionBuilder<IN, OUT>>::Inputs,
  ) -> Result<Vec<Instruction>>
  where
    Self: InstructionBuilder<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint,
  {
    <Self as InstructionBuilder<IN, OUT>>::build(inputs)
  }

  fn lookup_tables<IN, OUT>() -> &'static [Pubkey]
  where
    Self: InstructionBuilder<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint,
  {
    <Self as InstructionBuilder<IN, OUT>>::REQUIRED_LOOKUP_TABLES
  }
}

/// Helper for simulating events with compute units using cleaner syntax.
///
/// # Errors
/// Returns error if simulation fails.
pub async fn simulate_event_with_cus<Client, I, O>(
  client: &Client,
  user: Pubkey,
  inputs: <Client as BuildTransactionData<I, O>>::Inputs,
) -> Result<(<Client as SimulatePrice<I, O>>::Event, Option<u64>)>
where
  Client: SimulatePrice<I, O> + Send + Sync,
  <Client as BuildTransactionData<I, O>>::Inputs: QuoteInput,
  I: TokenMint,
  O: TokenMint,
{
  Client::simulate_event_with_cus(client, user, inputs).await
}
