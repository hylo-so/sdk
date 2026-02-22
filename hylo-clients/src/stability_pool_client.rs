use std::sync::Arc;

use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::solana_sdk::transaction::VersionedTransaction;
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use hylo_idl::stability_pool::client::args;
use hylo_idl::stability_pool::events::StabilityPoolStats;
use hylo_idl::stability_pool::instruction_builders;
use hylo_idl::tokens::{HYUSD, SHYUSD};

use crate::instructions::StabilityPoolInstructionBuilder as StabilityPoolIB;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::syntax_helpers::InstructionBuilderExt;
use crate::transaction::{
  BuildTransactionData, StabilityPoolArgs, TransactionSyntax,
};
use crate::util::{EXCHANGE_LOOKUP_TABLE, STABILITY_POOL_LOOKUP_TABLE};

/// Client for interacting with the Hylo Stability Pool program.
///
/// Provides functionality for depositing and withdrawing sHYUSD from the
/// stability pool. Supports transaction execution and price simulation for
/// offchain quoting.
///
/// # Examples
///
/// ## Setup
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # fn setup_client() -> Result<StabilityPoolClient> {
/// let client = StabilityPoolClient::new_random_keypair(
///   Cluster::Mainnet,
///   CommitmentConfig::confirmed(),
/// )?;
/// # Ok(client)
/// # }
/// ```
///
/// ## Transaction Execution
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # async fn execute_transaction(client: StabilityPoolClient) -> Result<Signature> {
/// // Deposit HYUSD → sHYUSD
/// let user = Pubkey::new_unique();
/// let signature = client.run_transaction::<HYUSD, SHYUSD>(StabilityPoolArgs {
///   amount: UFix64::new(100),
///   user,
/// }).await?;
/// # Ok(signature)
/// # }
/// ```
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
  /// Rebalances levercoin from the stability pool back to stablecoin.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn rebalance_lever_to_stable(&self) -> Result<Signature> {
    let instruction =
      instruction_builders::rebalance_lever_to_stable(self.program.payer());
    let instructions = vec![instruction];
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    let tx_args = VersionedTransactionData::new(instructions, lookup_tables);
    let sig = self.send_v0_transaction(&tx_args).await?;
    Ok(sig)
  }

  /// Simulates the `get_stats` instruction on the stability pool.
  ///
  /// # Errors
  /// - Simulation failure
  /// - Return data access or deserialization
  pub async fn get_stats(&self) -> Result<StabilityPoolStats> {
    let instruction = instruction_builders::get_stats();
    let tx = self
      .program
      .request()
      .instruction(instruction)
      .signed_transaction()
      .await?;
    let tx: VersionedTransaction = tx.into();
    let stats = self.simulate_transaction_return(&tx).await?;
    Ok(stats)
  }

  /// Initializes the stability pool.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
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
  /// - Failed to build transaction instructions
  pub fn initialize_lp_token_mint(&self) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::initialize_lp_token_mint(self.program.payer());
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the withdrawal fee for the stability pool.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_withdrawal_fee(
    &self,
    args: &args::UpdateWithdrawalFee,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_withdrawal_fee(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<HYUSD, SHYUSD> for StabilityPoolClient {
  type Inputs = StabilityPoolArgs;

  async fn build(
    &self,
    inputs: StabilityPoolArgs,
  ) -> Result<VersionedTransactionData> {
    let instructions =
      StabilityPoolIB::build_instructions::<HYUSD, SHYUSD>(inputs)?;
    let lut_addresses = StabilityPoolIB::lookup_tables::<HYUSD, SHYUSD>();
    let lookup_tables = self.load_multiple_lookup_tables(lut_addresses).await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<SHYUSD, HYUSD> for StabilityPoolClient {
  type Inputs = StabilityPoolArgs;

  async fn build(
    &self,
    inputs: StabilityPoolArgs,
  ) -> Result<VersionedTransactionData> {
    let instructions =
      StabilityPoolIB::build_instructions::<SHYUSD, HYUSD>(inputs)?;
    let lut_addresses = StabilityPoolIB::lookup_tables::<SHYUSD, HYUSD>();
    let lookup_tables = self.load_multiple_lookup_tables(lut_addresses).await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

#[async_trait::async_trait]
impl TransactionSyntax for StabilityPoolClient {}
