use std::sync::Arc;

use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use hylo_idl::earn_pool::client::args;
use hylo_idl::earn_pool::instruction_builders;
use hylo_idl::earn_pool::types::TokenMetadata;

use crate::memo::build_memo;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::signing::{SignedTransaction, SigningMethod};

/// Admin client for the Hylo earn pool program. Manages pool
/// initialization, rebalancing, fee configuration, and stats.
/// User-facing deposit/withdraw goes through
/// [`crate::router_client::RouterClient`].
pub struct EarnPoolClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
  signing_method: SigningMethod,
}

impl ProgramClient for EarnPoolClient {
  const PROGRAM_ID: Pubkey = hylo_idl::earn_pool::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> EarnPoolClient {
    EarnPoolClient {
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

impl EarnPoolClient {
  /// Selects the signing method for privileged and initializer instructions.
  #[must_use]
  pub fn with_signing_method(mut self, signing_method: SigningMethod) -> Self {
    self.signing_method = signing_method;
    self
  }

  fn signer(&self) -> Pubkey {
    self.signing_method.authority(self.program.payer())
  }

  async fn sign(
    &self,
    inner: VersionedTransactionData,
    memo: String,
  ) -> Result<SignedTransaction> {
    self
      .signing_method
      .prepare(&self.program.rpc(), self.program.payer(), inner, memo)
      .await
  }

  /// Initializes the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_earn_pool(
    &self,
    upgrade_authority: Pubkey,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::initialize_earn_pool(
      self.signer(),
      upgrade_authority,
    );
    let memo = build_memo("initialize_earn_pool", &instruction);
    self
      .sign(VersionedTransactionData::one(instruction), memo)
      .await
  }

  /// Initializes the LP token mint for the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_lp_token_mint(
    &self,
    lp_token_metadata: TokenMetadata,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::initialize_lp_token_mint(
      self.signer(),
      lp_token_metadata,
    );
    let memo = build_memo("initialize_lp_token_mint", &instruction);
    self
      .sign(VersionedTransactionData::one(instruction), memo)
      .await
  }

  /// Deprecates the levercoin pool via Squads proposal.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn deprecate_levercoin_pool(&self) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::deprecate_levercoin_pool(self.signer());
    let memo = build_memo("deprecate_levercoin_pool", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the withdrawal fee.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_withdrawal_fee(
    &self,
    args: &args::UpdateWithdrawalFee,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_withdrawal_fee(self.signer(), args);
    let memo = build_memo("update_withdrawal_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the withdrawal limit.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_withdrawal_limit(
    &self,
    args: &args::UpdateWithdrawalLimit,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_withdrawal_limit(self.signer(), args);
    let memo = build_memo("update_withdrawal_limit", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the deposit limit.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_deposit_limit(
    &self,
    args: &args::UpdateDepositLimit,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_deposit_limit(self.signer(), args);
    let memo = build_memo("update_deposit_limit", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Pauses the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn pause_earn_pool(&self) -> Result<SignedTransaction> {
    let instruction = instruction_builders::pause_earn_pool(self.signer());
    let memo = build_memo("pause_earn_pool", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Unpauses the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn unpause_earn_pool(&self) -> Result<SignedTransaction> {
    let instruction = instruction_builders::unpause_earn_pool(self.signer());
    let memo = build_memo("unpause_earn_pool", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }
}
