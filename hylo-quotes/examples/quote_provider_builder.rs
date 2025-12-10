//! Example: Using `QuoteProvider` with `QuoteBuilder`
//!
//! This example demonstrates the recommended way to fetch quotes using
//! `QuoteProvider` with `QuoteBuilder`. The `QuoteProvider` handles mint pair
//! matching and returns both the quote and metadata.
//!
//! Run with:
//! ```bash
//! RPC_URL=https://api.mainnet-beta.solana.com cargo run --example quote_provider_builder
//! ```

use std::env;
use std::sync::Arc;

use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use hylo_clients::prelude::CommitmentConfig;
use hylo_clients::protocol_state::RpcStateProvider;
use hylo_clients::util::REFERENCE_WALLET;
use hylo_idl::tokens::{TokenMint, HYUSD, JITOSOL};
use hylo_quotes::{
  ExecutableQuote, QuoteAmounts, QuoteBuilder, QuoteMetadata, QuoteProvider,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let rpc_url = env::var("RPC_URL")
    .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

  let rpc_client = Arc::new(RpcClient::new_with_commitment(
    rpc_url,
    CommitmentConfig::confirmed(),
  ));
  let state_provider = Arc::new(RpcStateProvider::new(rpc_client));

  // Create a quote builder (uses estimated compute units)
  let builder = QuoteBuilder::new(state_provider);

  // Wrap in QuoteProvider for mint pair matching
  let provider = QuoteProvider::new(builder);

  // Fetch a quote for minting hyUSD with JitoSOL
  let user = REFERENCE_WALLET; // Reference wallet with proper accounts for testing
  let amount = 1_000_000_000; // 1 JitoSOL (9 decimals)
  let slippage_bps = 50; // 0.5% slippage tolerance
  let slippage_pct = f64::from(slippage_bps) / 100.0;

  println!("Fetching quote: {amount} JitoSOL -> hyUSD");
  println!("User: {user}");
  println!("Slippage: {slippage_bps} bps ({slippage_pct:.2}%)");

  match provider
    .fetch_quote(JITOSOL::MINT, HYUSD::MINT, amount, user, slippage_bps)
    .await?
  {
    Some((quote, metadata)) => {
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
      println!("\n✓ Ready to sign and send transaction!");
    }
    None => {
      println!("✗ Quote not available for this mint pair");
    }
  }

  Ok(())
}
