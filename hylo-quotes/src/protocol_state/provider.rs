//! State provider trait and implementations
//!
//! Provides abstractions for fetching Hylo protocol state from various sources.

use std::sync::Arc;

use anchor_lang::prelude::Clock;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hylo_core::solana_clock::SolanaClock;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::protocol_state::{ProtocolAccounts, ProtocolState};

/// Trait for fetching protocol state from a data source
#[async_trait]
pub trait StateProvider<C: SolanaClock>: Send + Sync {
  /// Fetch the current protocol state
  ///
  /// # Errors
  /// Returns error if state fetching fails.
  async fn fetch_state(&self) -> Result<ProtocolState<C>>;
}

// Implement StateProvider for Arc<T> where T: StateProvider
#[async_trait]
impl<T: StateProvider<C>, C: SolanaClock> StateProvider<C>
  for std::sync::Arc<T>
{
  async fn fetch_state(&self) -> Result<ProtocolState<C>> {
    (**self).fetch_state().await
  }
}

// ============================================================================
// RPC STATE PROVIDER
// ============================================================================

/// State provider that fetches protocol state via Solana RPC
pub struct RpcStateProvider {
  rpc_client: Arc<RpcClient>,
}

impl RpcStateProvider {
  /// Create a new RPC state provider
  ///
  /// # Arguments
  /// * `rpc_client` - Solana RPC client for fetching account data
  #[must_use]
  pub fn new(rpc_client: Arc<RpcClient>) -> Self {
    Self { rpc_client }
  }
}

#[async_trait]
impl StateProvider<Clock> for RpcStateProvider {
  async fn fetch_state(&self) -> Result<ProtocolState<Clock>> {
    let pubkeys = ProtocolAccounts::pubkeys();
    let account_data = self
      .rpc_client
      .get_multiple_accounts(&pubkeys)
      .await
      .map_err(|e| anyhow!("Failed to fetch accounts from RPC: {e}"))?;
    let accounts = ProtocolAccounts::try_from((
      pubkeys.as_slice(),
      account_data.as_slice(),
    ))?;
    ProtocolState::try_from(&accounts)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use fix::prelude::*;
  use fix::typenum::{N8, N9};
  use hylo_core::solana_clock::SolanaClock;
  use solana_rpc_client::nonblocking::rpc_client::RpcClient;

  use super::*;

  fn build_test_rpc_client() -> Arc<RpcClient> {
    // Use RPC_URL env var if set, otherwise default to mainnet
    let rpc_url = std::env::var("RPC_URL")
      .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    Arc::new(RpcClient::new(rpc_url))
  }

  #[tokio::test]
  #[ignore = "requires lst_swap_fee on mainnet"]
  async fn test_fetch_state() {
    let rpc_client = build_test_rpc_client();
    let provider = RpcStateProvider::new(rpc_client);
    let state = provider
      .fetch_state()
      .await
      .expect("Failed to fetch protocol state");

    // Verify timestamp is set and matches clock
    assert!(state.fetched_at > 0);
    let clock_timestamp = state.exchange_context.clock.unix_timestamp();
    assert_eq!(state.fetched_at, clock_timestamp);

    // Verify exchange context has valid data
    assert!(state.exchange_context.total_sol > UFix64::<N9>::zero());
    assert!(state.exchange_context.collateral_ratio > UFix64::<N9>::zero());
    assert!(state.exchange_context.sol_usd_price.lower > UFix64::<N8>::zero());
    assert!(
      state.exchange_context.sol_usd_price.upper
        >= state.exchange_context.sol_usd_price.lower
    );

    // Verify mint accounts have valid data
    assert!(state.hyusd_mint.decimals > 0);
    assert!(state.xsol_mint.decimals > 0);
    assert!(state.shyusd_mint.decimals > 0);

    // Verify clock has reasonable values (slot is u64, so just check it's set)
    assert!(state.exchange_context.clock.slot() > 0);
  }
}
