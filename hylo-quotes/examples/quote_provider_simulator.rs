//! Example: Using `QuoteProvider` with `QuoteSimulator`
//!
//! This example demonstrates using `QuoteSimulator` for more accurate compute
//! units by actually simulating the transaction on-chain. This is recommended
//! for production use.
//!
//! Run with:
//! ```bash
//! RPC_URL=https://api.mainnet-beta.solana.com cargo run --example quote_provider_simulator
//! ```

use std::env;
use std::sync::Arc;

use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use hylo_clients::prelude::CommitmentConfig;
use hylo_clients::protocol_state::RpcStateProvider;
use hylo_clients::util::REFERENCE_WALLET;
use hylo_idl::tokens::{TokenMint, HYUSD, JITOSOL};
use hylo_quotes::{
  ComputeUnitMethod, ExecutableQuote, QuoteAmounts, QuoteMetadata,
  QuoteProvider, QuoteSimulator, SolanaRpcProvider,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let rpc_url = env::var("RPC_URL")
    .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

  // Create a shared RPC client to reuse connection pools and reduce file
  // descriptor usage
  let shared_rpc_client = Arc::new(RpcClient::new_with_commitment(
    rpc_url.clone(),
    CommitmentConfig::confirmed(),
  ));

  // Set up state provider (shares the RPC client)
  let state_provider =
    Arc::new(RpcStateProvider::new(shared_rpc_client.clone()));

  // Set up RPC provider for simulation (shares the same RPC client)
  let rpc_provider = SolanaRpcProvider::new(shared_rpc_client);

  // Create simulator (uses simulated compute units)
  let simulator = QuoteSimulator::new(state_provider, rpc_provider);

  // Wrap in QuoteProvider
  let provider = QuoteProvider::new(simulator);

  let user = REFERENCE_WALLET; // Reference wallet with proper accounts for testing
  let amount = 1_000_000_000; // 1 JitoSOL
  let slippage_bps = 50; // 0.5%
  let slippage_pct = f64::from(slippage_bps) / 100.0;

  println!("Fetching quote with simulation: {amount} JitoSOL -> hyUSD",);
  println!("User: {user}");
  println!("Slippage: {slippage_bps} bps ({slippage_pct:.2}%)");

  let (quote, metadata) = provider
    .fetch_quote(JITOSOL::MINT, HYUSD::MINT, amount, user, slippage_bps)
    .await?;

  let ExecutableQuote {
    amounts,
    compute_units,
    compute_units_safe,
    compute_unit_method,
    instructions,
  } = quote;

  let QuoteAmounts {
    amount_in,
    amount_out,
    fee_amount,
    fee_mint,
  } = amounts;

  let QuoteMetadata {
    operation,
    description,
  } = metadata;

  println!("✓ Quote fetched successfully!");
  println!("\nQuote Details:");
  println!("  Input:  {amount_in} JitoSOL");
  println!("  Output: {amount_out} hyUSD");
  println!("  Fee: {fee_amount} {fee_mint:?}");
  println!("\nTransaction:");
  println!("  Operation: {operation:?}");
  println!("  Description: {description}");
  println!("  Compute units: {compute_units} (safe: {compute_units_safe})",);
  println!("  Method: {compute_unit_method:?}");
  println!("  Instructions: {} instructions", instructions.len());

  match compute_unit_method {
    ComputeUnitMethod::Simulated => {
      println!("\n✓ Compute units were simulated on-chain (most accurate)");
    }
    ComputeUnitMethod::Estimated => {
      println!("\n⚠ Compute units were estimated (simulation may have failed)");
    }
  }

  Ok(())
}
