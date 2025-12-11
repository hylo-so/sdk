//! Integration tests for quote execution against mainnet
//!
//! These tests validate the full quote execution flow:
//! - State fetching
//! - Quote computation
//! - Transaction building
//! - Simulation
//!
//! Set `RPC_URL` environment variable to run against mainnet.
//! Tests are skipped if `RPC_URL` is not set.

use std::sync::Arc;

use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_lang::prelude::Pubkey;
use hylo_clients::prelude::CommitmentConfig;
use hylo_clients::protocol_state::RpcStateProvider;
use hylo_idl::tokens::{HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_quotes::{ExecutableQuote, QuoteSimulator, SolanaRpcProvider};

fn prepare_simulator(
) -> Option<QuoteSimulator<Arc<RpcStateProvider>, SolanaRpcProvider>> {
  let rpc_url = std::env::var("RPC_URL").ok()?;

  // Share RPC client to reuse connection pools
  let shared_rpc_client = Arc::new(RpcClient::new_with_commitment(
    rpc_url.clone(),
    CommitmentConfig::confirmed(),
  ));

  let state_provider =
    Arc::new(RpcStateProvider::new(shared_rpc_client.clone()));

  let rpc_provider = SolanaRpcProvider::new(shared_rpc_client);

  Some(QuoteSimulator::new(state_provider, rpc_provider))
}

fn get_test_wallet() -> Pubkey {
  /// Test wallet with real mainnet balances for accurate simulation
  const TEST_WALLET: &str = "GUX587fnbnZmqmq2hnav8r6siLczKS8wrp9QZRhuWeai";
  TEST_WALLET.parse().expect("Invalid test wallet")
}

#[tokio::test]
async fn test_mint_hyusd_from_jitosol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<JITOSOL, HYUSD>(1_000_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Mint hyUSD with JitoSOL");
}

#[tokio::test]
async fn test_redeem_hyusd_to_jitosol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<HYUSD, JITOSOL>(1_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Redeem hyUSD for JitoSOL");
}

#[tokio::test]
async fn test_mint_hyusd_from_hylosol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<HYLOSOL, HYUSD>(1_000_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Mint hyUSD with hyloSOL");
}

#[tokio::test]
async fn test_redeem_hyusd_to_hylosol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<HYUSD, HYLOSOL>(1_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Redeem hyUSD for hyloSOL");
}

#[tokio::test]
async fn test_mint_xsol_from_jitosol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<JITOSOL, XSOL>(1_000_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Mint xSOL with JitoSOL");
}

#[tokio::test]
async fn test_redeem_xsol_to_jitosol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<XSOL, JITOSOL>(1_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Redeem xSOL for JitoSOL");
}

#[tokio::test]
async fn test_mint_xsol_from_hylosol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<HYLOSOL, XSOL>(1_000_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Mint xSOL with hyloSOL");
}

#[tokio::test]
async fn test_redeem_xsol_to_hylosol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<XSOL, HYLOSOL>(1_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Redeem xSOL for hyloSOL");
}

#[tokio::test]
async fn test_swap_hyusd_to_xsol() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<HYUSD, XSOL>(1_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Swap hyUSD to xSOL");
}

#[tokio::test]
async fn test_swap_xsol_to_hyusd() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<XSOL, HYUSD>(1_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Swap xSOL to hyUSD");
}

#[tokio::test]
async fn test_deposit_hyusd_to_stability_pool() {
  let Some(simulator) = prepare_simulator() else {
    eprintln!("Skipping test: RPC_URL not set");
    return;
  };
  let wallet = get_test_wallet();

  let quote = simulator
    .simulate_quote::<HYUSD, SHYUSD>(1_000_000, wallet, 50)
    .await
    .expect("Quote simulation should succeed");

  validate_executable_quote(&quote, "Deposit hyUSD to Stability Pool");
}

fn validate_executable_quote(quote: &ExecutableQuote, test_name: &str) {
  assert!(
    quote.amounts.amount_in > 0,
    "{test_name}: amount_in should be > 0"
  );
  assert!(
    quote.amounts.amount_out > 0,
    "{test_name}: amount_out should be > 0"
  );
  assert!(
    quote.compute_units > 0,
    "{test_name}: compute_units should be > 0"
  );
  assert!(
    quote.compute_units_safe > quote.compute_units,
    "{test_name}: compute_units_safe should be > compute_units"
  );
  assert!(
    !quote.instructions.is_empty(),
    "{test_name}: instructions should not be empty"
  );
}
