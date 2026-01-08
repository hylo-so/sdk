//! Extension traits for cleaner static dispatch syntax.

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_lang::{AnchorDeserialize, Discriminator};
use anyhow::Result;
use async_trait::async_trait;
use hylo_idl::tokens::TokenMint;

use crate::instructions::InstructionBuilder;
use crate::transaction::{BuildTransactionData, QuoteInput, SimulatePrice};

/// Turbofish syntax for [`InstructionBuilder`].
///
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # fn example() -> Result<()> {
/// let user = Pubkey::new_unique();
/// let args = MintArgs { amount: UFix64::one(), user, slippage_config: None };
/// let instructions = ExchangeInstructionBuilder::build_instructions::<JITOSOL, HYUSD>(args)?;
/// let luts = ExchangeInstructionBuilder::lookup_tables::<JITOSOL, HYUSD>();
/// # Ok(())
/// # }
/// ```
pub trait InstructionBuilderExt {
  /// Builds instructions for a token pair operation.
  ///
  /// # Errors
  /// * Underlying builder errors.
  fn build_instructions<IN, OUT>(
    inputs: <Self as InstructionBuilder<IN, OUT>>::Inputs,
  ) -> Result<Vec<Instruction>>
  where
    Self: InstructionBuilder<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint;

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

/// Turbofish syntax for [`SimulatePrice`](crate::transaction::SimulatePrice).
///
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # async fn example(client: ExchangeClient) -> Result<()> {
/// let user = Pubkey::new_unique();
/// let args = MintArgs { amount: UFix64::one(), user, slippage_config: None };
/// let (event, cus) = client.simulate_event_with_cus::<JITOSOL, HYUSD>(user, args).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait SimulatePriceExt {
  async fn simulate_event_with_cus<I, O>(
    &self,
    user: Pubkey,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<(<Self as SimulatePrice<I, O>>::Event, Option<u64>)>
  where
    Self: SimulatePrice<I, O> + Send + Sync,
    <Self as BuildTransactionData<I, O>>::Inputs: QuoteInput,
    <Self as SimulatePrice<I, O>>::Event: AnchorDeserialize + Discriminator,
    I: TokenMint,
    O: TokenMint;
}

#[async_trait]
impl<X> SimulatePriceExt for X {
  async fn simulate_event_with_cus<I, O>(
    &self,
    user: Pubkey,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<(<Self as SimulatePrice<I, O>>::Event, Option<u64>)>
  where
    Self: SimulatePrice<I, O> + Send + Sync,
    <Self as BuildTransactionData<I, O>>::Inputs: QuoteInput,
    <Self as SimulatePrice<I, O>>::Event: AnchorDeserialize + Discriminator,
    I: TokenMint,
    O: TokenMint,
  {
    <Self as SimulatePrice<I, O>>::simulate_event_with_cus(self, user, inputs)
      .await
  }
}
