use std::sync::Arc;

use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use hylo_idl::stability_pool::client::args;
use hylo_idl::stability_pool::instruction_builders;
use hylo_idl::stability_pool::types::TokenMetadata;

use crate::memo::build_memo;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::squads::{SquadsContext, SquadsTransactionData};

/// Admin client for the Hylo stability pool program. Manages pool
/// initialization, rebalancing, fee configuration, and stats.
/// User-facing deposit/withdraw goes through
/// [`crate::router_client::RouterClient`].
pub struct StabilityPoolClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for StabilityPoolClient {
  const PROGRAM_ID: Pubkey = hylo_idl::stability_pool::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> StabilityPoolClient {
    StabilityPoolClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl StabilityPoolClient {
  /// Initializes the stability pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn initialize_stability_pool(
    &self,
    upgrade_authority: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::initialize_stability_pool(
      self.program.payer(),
      upgrade_authority,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Initializes the LP token mint for the stability pool.
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
}
