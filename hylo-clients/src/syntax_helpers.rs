//! Helper functions for cleaner static dispatch syntax.

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use hylo_idl::tokens::TokenMint;

use crate::instructions::InstructionBuilder;
use crate::transaction::{BuildTransactionData, QuoteInput, SimulatePrice};

/// Helper for building instructions with cleaner syntax.
///
/// # Errors
/// Returns error if instruction building fails.
pub fn build_instructions<Builder, IN, OUT>(
  inputs: <Builder as InstructionBuilder<IN, OUT>>::Inputs,
) -> Result<Vec<Instruction>>
where
  Builder: InstructionBuilder<IN, OUT>,
  IN: TokenMint,
  OUT: TokenMint,
{
  Builder::build_instructions(inputs)
}

/// Helper for getting lookup tables with cleaner syntax.
#[must_use]
pub fn lookup_tables<Builder, IN, OUT>() -> &'static [Pubkey]
where
  Builder: InstructionBuilder<IN, OUT>,
  IN: TokenMint,
  OUT: TokenMint,
{
  Builder::REQUIRED_LOOKUP_TABLES
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
