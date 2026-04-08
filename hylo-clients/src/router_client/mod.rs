mod instructions;
mod transaction_data;

use std::sync::Arc;

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::Result;
use async_trait::async_trait;
use hylo_core::slippage_config::SlippageConfig;
use hylo_idl::tokens::TokenMint;

use crate::program_client::ProgramClient;
use crate::transaction::TransactionSyntax;

/// Arguments for all router-based token operations.
pub struct RouterArgs {
  pub amount: u64,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

pub trait InstructionBuilder<IN: TokenMint, OUT: TokenMint> {
  type Inputs;
  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey];

  /// Static type-safe instruction builder for token pair operations.
  ///
  /// # Errors
  /// Returns error if instruction building fails.
  fn build(inputs: Self::Inputs) -> Result<Vec<Instruction>>;
}

pub trait InstructionBuilderExt {
  /// Turbofish syntax for [`InstructionBuilder`].
  ///
  /// # Errors
  /// * Instruction building fails.
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

/// Builds and executes transactions through the Hylo router program.
/// Handles all user-facing token operations: mint, redeem, swap, and
/// stability pool deposit/withdraw.
pub struct RouterClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for RouterClient {
  const PROGRAM_ID: Pubkey = hylo_idl::router::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> RouterClient {
    RouterClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

#[async_trait]
impl TransactionSyntax for RouterClient {}
