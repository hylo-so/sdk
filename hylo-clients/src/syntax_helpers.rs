//! Extension traits for cleaner static dispatch syntax.

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use hylo_idl::tokens::TokenMint;

use crate::instructions::InstructionBuilder;

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
