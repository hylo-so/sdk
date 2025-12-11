//! Example: Direct usage of `QuoteBuilder`
//!
//! This example demonstrates direct quote building without mint pair matching.
//! This is useful when you know the exact token types at compile time and don't
//! need the convenience of `QuoteProvider`'s mint pair matching.
//!
//! Run with:
//! ```bash
//! RPC_URL=https://api.mainnet-beta.solana.com cargo run --example quote_builder_direct
//! ```

use std::env;
use std::sync::Arc;

use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use hylo_clients::prelude::CommitmentConfig;
use hylo_clients::protocol_state::RpcStateProvider;
use hylo_clients::util::REFERENCE_WALLET;
use hylo_idl::tokens::{HYUSD, JITOSOL};
use hylo_quotes::{ExecutableQuote, QuoteAmounts, QuoteBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let rpc_url = env::var("RPC_URL")
    .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

  let rpc_client = Arc::new(RpcClient::new_with_commitment(
    rpc_url,
    CommitmentConfig::confirmed(),
  ));
  let state_provider = Arc::new(RpcStateProvider::new(rpc_client));
  let builder = QuoteBuilder::new(state_provider);

  let user = REFERENCE_WALLET; // Reference wallet with proper accounts for testing
  let amount = 1_000_000_000; // 1 JitoSOL
  let slippage_bps = 50; // 0.5%
  let slippage_pct = f64::from(slippage_bps) / 100.0;

  println!("Building quote directly: JITOSOL -> HYUSD");
  println!("Amount: {amount} JitoSOL");
  println!("User: {user}");
  println!("Slippage: {slippage_bps} bps ({slippage_pct:.2}%)");

  // Direct quote building with type parameters
  let quote = builder
    .build_quote::<JITOSOL, HYUSD>(amount, user, slippage_bps)
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

  println!("✓ Quote built successfully!");
  println!("\nQuote Details:");
  println!("  Input:  {amount_in} JitoSOL");
  println!("  Output: {amount_out} hyUSD");
  println!("  Fee: {fee_amount} {fee_mint:?}");
  println!("\nTransaction:");
  println!("  Compute units: {compute_units} (safe: {compute_units_safe})",);
  println!("  Method: {compute_unit_method:?}");
  println!("  Instructions: {} instructions", instructions.len());
  println!("\n✓ Ready to sign and send transaction!");

  Ok(())
}
