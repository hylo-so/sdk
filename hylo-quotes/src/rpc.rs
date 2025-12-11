//! RPC provider abstraction (enables testing)

use std::sync::Arc;

use anchor_client::solana_client::client_error::Result;
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_config::RpcSimulateTransactionConfig;
use anchor_client::solana_client::rpc_response::{
  Response, RpcSimulateTransactionResult,
};
use anchor_client::solana_sdk::hash::Hash;
use anchor_client::solana_sdk::transaction::VersionedTransaction;
use async_trait::async_trait;

/// Abstraction over RPC operations needed for quote simulation
#[async_trait]
#[allow(clippy::result_large_err)]
pub trait RpcProvider: Send + Sync {
  /// # Errors
  /// Returns error if RPC call fails.
  async fn get_latest_blockhash(&self) -> Result<Hash>;

  /// # Errors
  /// Returns error if RPC simulation fails.
  async fn simulate_transaction_with_config(
    &self,
    transaction: VersionedTransaction,
    config: RpcSimulateTransactionConfig,
  ) -> Result<Response<RpcSimulateTransactionResult>>;
}

/// Real RPC provider wrapping Solana's `RpcClient`
pub struct SolanaRpcProvider {
  client: Arc<RpcClient>,
}

impl SolanaRpcProvider {
  /// Create a new RPC provider
  #[must_use]
  pub fn new(client: Arc<RpcClient>) -> Self {
    Self { client }
  }
}

#[async_trait]
impl RpcProvider for SolanaRpcProvider {
  async fn get_latest_blockhash(&self) -> Result<Hash> {
    self.client.get_latest_blockhash().await
  }

  async fn simulate_transaction_with_config(
    &self,
    transaction: VersionedTransaction,
    config: RpcSimulateTransactionConfig,
  ) -> Result<Response<RpcSimulateTransactionResult>> {
    self
      .client
      .simulate_transaction_with_config(&transaction, config)
      .await
  }
}
