mod instructions;
mod transaction_data;

use std::sync::Arc;

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anchor_lang::AccountDeserialize;
use anyhow::{Context, Result};
use async_trait::async_trait;
use hylo_core::slippage_config::SlippageConfig;
use hylo_idl::pda;
use hylo_idl::router::accounts::ExoRegistry;
use hylo_idl::router::instruction_builders;
use hylo_idl::tokens::TokenMint;

use crate::memo::build_memo;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::signing::{SignedTransaction, SigningClient, SigningMethod};
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

/// Builds and executes transactions through the Hylo router program:
/// mint, redeem, swap, earn pool deposit and withdraw, and exo registry
/// administration.
pub struct RouterClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
  signing_method: SigningMethod,
}

impl ProgramClient for RouterClient {
  const PROGRAM_ID: Pubkey = hylo_idl::router::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> RouterClient {
    RouterClient {
      program,
      keypair,
      signing_method: SigningMethod::Direct,
    }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl SigningClient for RouterClient {
  fn signing_method(&self) -> SigningMethod {
    self.signing_method
  }

  fn with_signing_method(mut self, signing_method: SigningMethod) -> Self {
    self.signing_method = signing_method;
    self
  }
}

#[async_trait]
impl TransactionSyntax for RouterClient {}

impl RouterClient {
  /// Fetches the registry of routable exo pairs.
  ///
  /// # Errors
  /// * Failed to fetch the account
  /// * Failed to deserialize account data
  pub async fn exo_registry(&self) -> Result<ExoRegistry> {
    let account = self.program().rpc().get_account(&pda::EXO_REGISTRY).await?;
    ExoRegistry::try_deserialize(&mut account.data.as_slice())
      .context("Exo registry deserialization")
  }

  /// Initializes the registry of routable exo pairs.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_exo_registry(&self) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::initialize_exo_registry(self.signer());
    let memo = build_memo("initialize_exo_registry", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Registers the exo pair for the given collateral mint with the router.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn register_exo_entry(
    &self,
    collateral_mint: Pubkey,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::register_exo_entry(self.signer(), collateral_mint);
    let memo = build_memo("register_exo_entry", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }
}
