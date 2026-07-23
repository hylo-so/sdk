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
use crate::squads::{SquadsContext, SquadsTransactionData};

/// Admin client for the Hylo earn pool program. Manages pool
/// initialization, rebalancing, fee configuration, and stats.
/// User-facing deposit/withdraw goes through
/// [`crate::router_client::RouterClient`].
pub struct EarnPoolClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for EarnPoolClient {
  const PROGRAM_ID: Pubkey = hylo_idl::earn_pool::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> EarnPoolClient {
    EarnPoolClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl EarnPoolClient {
  /// Initializes the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn initialize_earn_pool(
    &self,
    upgrade_authority: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::initialize_earn_pool(
      self.program.payer(),
      upgrade_authority,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Initializes the LP token mint for the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn initialize_lp_token_mint(
    &self,
    lp_token_metadata: TokenMetadata,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::initialize_lp_token_mint(
      self.program.payer(),
      lp_token_metadata,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Deprecates the levercoin pool via Squads proposal.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn deprecate_levercoin_pool(
    &self,
    squads: &SquadsContext,
  ) -> Result<SquadsTransactionData> {
    let instruction =
      instruction_builders::deprecate_levercoin_pool(squads.vault_pda());
    let memo = build_memo("deprecate_levercoin_pool", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.build_proposal(&inner, self.program.payer(), memo)
  }

  /// Updates the withdrawal fee.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn update_withdrawal_fee(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateWithdrawalFee,
  ) -> Result<SquadsTransactionData> {
    let instruction =
      instruction_builders::update_withdrawal_fee(squads.vault_pda(), args);
    let memo = build_memo("update_withdrawal_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.build_proposal(&inner, self.program.payer(), memo)
  }

  /// Updates the withdrawal limit.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn update_withdrawal_limit(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateWithdrawalLimit,
  ) -> Result<SquadsTransactionData> {
    let instruction =
      instruction_builders::update_withdrawal_limit(squads.vault_pda(), args);
    let memo = build_memo("update_withdrawal_limit", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.build_proposal(&inner, self.program.payer(), memo)
  }

  /// Updates the deposit limit.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn update_deposit_limit(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateDepositLimit,
  ) -> Result<SquadsTransactionData> {
    let instruction =
      instruction_builders::update_deposit_limit(squads.vault_pda(), args);
    let memo = build_memo("update_deposit_limit", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.build_proposal(&inner, self.program.payer(), memo)
  }

  /// Pauses the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn pause_earn_pool(
    &self,
    squads: &SquadsContext,
  ) -> Result<SquadsTransactionData> {
    let instruction = instruction_builders::pause_earn_pool(squads.vault_pda());
    let memo = build_memo("pause_earn_pool", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.build_proposal(&inner, self.program.payer(), memo)
  }

  /// Unpauses the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn unpause_earn_pool(
    &self,
    squads: &SquadsContext,
  ) -> Result<SquadsTransactionData> {
    let instruction =
      instruction_builders::unpause_earn_pool(squads.vault_pda());
    let memo = build_memo("unpause_earn_pool", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.build_proposal(&inner, self.program.payer(), memo)
  }
}
