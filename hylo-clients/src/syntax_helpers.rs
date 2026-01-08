//! Extension traits for cleaner static dispatch syntax.

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_lang::{AnchorDeserialize, Discriminator};
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

/// Extension trait for ergonomic simulate price method calls.
///
/// Provides `simulate_event_with_cus` method that can be called with turbofish
/// syntax on any type implementing [`SimulatePrice`].
///
/// # Example
///
/// ```rust,ignore
/// use hylo_clients::prelude::*;
///
/// let (event, cus) = ExchangeClient::simulate_event_with_cus::<JITOSOL, HYUSD>(
///   &client,
///   user,
///   args,
/// ).await?;
/// ```
pub trait SimulatePriceExt {
  /// Simulates transaction and returns the event and compute units consumed.
  ///
  /// # Errors
  /// Returns error if simulation fails.
  fn simulate_event_with_cus<I, O>(
    &self,
    user: Pubkey,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> impl std::future::Future<
    Output = Result<(<Self as SimulatePrice<I, O>>::Event, Option<u64>)>,
  > + Send
  where
    Self: SimulatePrice<I, O> + Send + Sync,
    <Self as BuildTransactionData<I, O>>::Inputs: QuoteInput,
    <Self as SimulatePrice<I, O>>::Event: AnchorDeserialize + Discriminator,
    I: TokenMint,
    O: TokenMint;
}

impl<X> SimulatePriceExt for X {
  fn simulate_event_with_cus<I, O>(
    &self,
    user: Pubkey,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> impl std::future::Future<
    Output = Result<(<Self as SimulatePrice<I, O>>::Event, Option<u64>)>,
  > + Send
  where
    Self: SimulatePrice<I, O> + Send + Sync,
    <Self as BuildTransactionData<I, O>>::Inputs: QuoteInput,
    <Self as SimulatePrice<I, O>>::Event: AnchorDeserialize + Discriminator,
    I: TokenMint,
    O: TokenMint,
  {
    <Self as SimulatePrice<I, O>>::simulate_event_with_cus(self, user, inputs)
  }
}
