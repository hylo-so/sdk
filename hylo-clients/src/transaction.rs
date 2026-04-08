//! Transaction building traits and argument types.

use anchor_client::solana_sdk::signature::Signature;
use anchor_lang::prelude::Pubkey;
use anchor_lang::AnchorDeserialize;
use anyhow::Result;
use hylo_core::slippage_config::SlippageConfig;

use crate::program_client::{ProgramClient, VersionedTransactionData};

/// Arguments for all router-based token operations.
pub struct RouterArgs {
  pub amount: u64,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

/// Builds transaction data for a token pair operation.
#[async_trait::async_trait]
pub trait BuildTransactionData<I, O> {
  type Inputs: Send + Sync + 'static;

  /// # Errors
  /// Returns error if transaction building fails.
  async fn build(
    &self,
    inputs: Self::Inputs,
  ) -> Result<VersionedTransactionData>;
}

/// High-level API for transaction operations.
#[async_trait::async_trait]
pub trait TransactionSyntax {
  /// Executes transaction by building and sending it.
  async fn run_transaction<I, O>(
    &self,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<Signature>
  where
    Self: BuildTransactionData<I, O> + ProgramClient,
  {
    let args = self.build(inputs).await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Builds transaction data without executing.
  async fn build_transaction_data<I, O>(
    &self,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<VersionedTransactionData>
  where
    Self: BuildTransactionData<I, O>,
  {
    self.build(inputs).await
  }

  /// Simulates transaction and returns parsed return data.
  async fn simulate_event<I, O, E>(
    &self,
    user: Pubkey,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<E>
  where
    Self: BuildTransactionData<I, O> + ProgramClient,
    E: AnchorDeserialize,
  {
    let args = self.build(inputs).await?;
    let tx = self.build_simulation_transaction(&user, &args).await?;
    self.simulate_transaction_return::<E>(&tx).await
  }

  /// Simulates transaction and returns parsed return data with
  /// compute units.
  async fn simulate_event_with_cus<I, O, E>(
    &self,
    user: Pubkey,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<(E, Option<u64>)>
  where
    Self: BuildTransactionData<I, O> + ProgramClient,
    E: AnchorDeserialize,
  {
    let args = self.build(inputs).await?;
    let tx = self.build_simulation_transaction(&user, &args).await?;
    self.simulate_transaction_return_with_cus::<E>(&tx).await
  }
}
