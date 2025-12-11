//! Calibrate compute unit defaults by simulating with real wallet
//!
//! This measures actual compute units for all routes using the type-safe API.
//! Run multiple times and average the results.
//!
//! Run with:
//! ```
//! RPC_URL=<your-rpc> cargo run --bin calibrate_compute_units --package hylo-quotes
//! ```

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_config::RpcSimulateTransactionConfig;
use anchor_client::solana_sdk::message::v0::Message;
use anchor_client::solana_sdk::message::VersionedMessage;
use anchor_client::solana_sdk::transaction::VersionedTransaction;
use anchor_lang::prelude::Pubkey;
use anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use anchor_spl::token;
use hylo_clients::prelude::{CommitmentConfig, Signature};
use hylo_clients::protocol_state::RpcStateProvider;
use hylo_idl::tokens::{HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_quotes::{ComputeUnitMethod, QuoteSimulator, SolanaRpcProvider};

#[allow(
  clippy::cast_possible_truncation,
  clippy::cast_precision_loss,
  clippy::cast_sign_loss,
  clippy::if_not_else,
  clippy::items_after_statements,
  clippy::too_many_lines
)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
  println!("=== Hylo Compute Unit Calibration ===\n");

  let rpc_url = std::env::var("RPC_URL")
    .expect("RPC_URL must be set to a mainnet RPC endpoint");

  println!("📡 RPC: {rpc_url}\n");

  // Use wallet with actual balances for accurate simulation
  let reference_wallet: Pubkey = "ENdBxMzhnCfnK4eNinKsoTQFXjjBkhiWTFTivUaWugKJ"
    .parse()
    .unwrap();

  println!("🔑 Reference wallet: {reference_wallet}\n");
  println!("   (Using wallet with real balances for accurate simulation)\n");

  // Create nonblocking RPC client
  let rpc_client = RpcClient::new_with_commitment(
    rpc_url.clone(),
    CommitmentConfig::confirmed(),
  );

  // Create shared RPC client to reuse connection pools
  let shared_rpc_client = Arc::new(rpc_client);

  // ========================================================================
  // Step 1: Measure pure ATA creation cost with reference wallet
  // ========================================================================

  println!("📏 Step 1: Measuring ATA creation overhead\n");
  println!("   Simulating USDC ATA creation for reference wallet...\n");

  let mut ata_creation_costs = Vec::new();

  // Use USDC - a standard token the wallet probably doesn't have an ATA for yet
  let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    .parse::<Pubkey>()
    .unwrap();

  // Take 10 samples
  for _ in 0..10 {
    // Build the idempotent ATA creation instruction
    let create_ata_ix = create_associated_token_account_idempotent(
      &reference_wallet, // payer (exists, has SOL)
      &reference_wallet, // owner
      &usdc_mint,        // USDC mint
      &token::ID,
    );

    // Get recent blockhash
    let recent_blockhash = match shared_rpc_client.get_latest_blockhash().await
    {
      Ok(hash) => hash,
      Err(e) => {
        eprintln!("   Failed to get blockhash: {e}");
        continue;
      }
    };

    // Build transaction for simulation
    let message = match Message::try_compile(
      &reference_wallet,
      &[create_ata_ix],
      &[],
      recent_blockhash,
    ) {
      Ok(msg) => msg,
      Err(e) => {
        eprintln!("   Failed to compile message: {e}");
        continue;
      }
    };

    let versioned_tx = VersionedTransaction {
      signatures: vec![Signature::default()],
      message: VersionedMessage::V0(message),
    };

    // Simulate
    if let Ok(response) = shared_rpc_client
      .simulate_transaction_with_config(
        &versioned_tx,
        RpcSimulateTransactionConfig {
          sig_verify: false,
          replace_recent_blockhash: true,
          commitment: Some(CommitmentConfig::confirmed()),
          ..Default::default()
        },
      )
      .await
    {
      // Extract CU even if transaction would fail (e.g., ATA already exists)
      if let Some(cu) = response.value.units_consumed {
        if cu > 0 {
          ata_creation_costs.push(cu);
          print!(".");
        }
      }
    }
  }

  println!("\n");

  let avg_ata_cost = if ata_creation_costs.is_empty() {
    println!(
      "   ⚠️  Could not measure ATA creation, using estimated 6000 CU\n"
    );
    6_000
  } else {
    let sum: u64 = ata_creation_costs.iter().sum();
    let avg = sum / ata_creation_costs.len() as u64;
    let min = *ata_creation_costs.iter().min().unwrap();
    let max = *ata_creation_costs.iter().max().unwrap();
    println!("   ✅ ATA Creation Cost:");
    println!("      Samples: {}", ata_creation_costs.len());
    println!("      Min:     {min} CU");
    println!("      Max:     {max} CU");
    println!("      Average: {avg} CU\n");
    avg
  };

  // ========================================================================
  // Step 2: Measure full route operations with existing ATAs
  // ========================================================================

  println!("📏 Step 2: Measuring route operations (with existing ATAs)\n");
  println!("   Running 5 samples per route...\n");

  // Create simulator with type-safe API (shares RPC client)
  let state_provider =
    Arc::new(RpcStateProvider::new(shared_rpc_client.clone()));
  let rpc_provider = SolanaRpcProvider::new(shared_rpc_client);
  let simulator = QuoteSimulator::new(state_provider, rpc_provider);

  // Amounts based on wallet balances:
  // - JitoSOL: 0.016616 (16,616,000 base units)
  // - hyloSOL: 0.048975 (48,975,000 base units)
  // - HYUSD: 1.6032 (1,603,200 base units)
  // - XSOL: 4.126 (4,126,000 base units)

  type RouteSamples = std::collections::HashMap<String, Vec<u64>>;
  let mut route_samples: RouteSamples = std::collections::HashMap::new();

  // Helper to print results for a route
  fn print_route_results(samples: &[u64]) {
    if !samples.is_empty() {
      let avg = samples.iter().sum::<u64>() / samples.len() as u64;
      let min = *samples.iter().min().unwrap();
      let max = *samples.iter().max().unwrap();
      println!("✅ avg={avg} min={min} max={max}");
    } else {
      println!("❌ All samples failed");
    }
  }

  // Stablecoin mints (use half of available balance)
  {
    print!("   Mint HYUSD from JitoSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<JITOSOL, HYUSD>(8_000_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Mint HYUSD from JitoSOL".to_string(), samples);
  }

  {
    print!("   Mint HYUSD from hyloSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<HYLOSOL, HYUSD>(20_000_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Mint HYUSD from hyloSOL".to_string(), samples);
  }

  // Stablecoin redeems (use half of available HYUSD)
  {
    print!("   Redeem HYUSD to JitoSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<HYUSD, JITOSOL>(800_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Redeem HYUSD to JitoSOL".to_string(), samples);
  }

  {
    print!("   Redeem HYUSD to hyloSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<HYUSD, HYLOSOL>(800_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Redeem HYUSD to hyloSOL".to_string(), samples);
  }

  // Levercoin mints (use half of available balance)
  {
    print!("   Mint XSOL from JitoSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<JITOSOL, XSOL>(8_000_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Mint XSOL from JitoSOL".to_string(), samples);
  }

  {
    print!("   Mint XSOL from hyloSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<HYLOSOL, XSOL>(20_000_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Mint XSOL from hyloSOL".to_string(), samples);
  }

  // Levercoin redeems (use half of available XSOL)
  {
    print!("   Redeem XSOL to JitoSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<XSOL, JITOSOL>(2_000_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Redeem XSOL to JitoSOL".to_string(), samples);
  }

  {
    print!("   Redeem XSOL to hyloSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<XSOL, HYLOSOL>(2_000_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Redeem XSOL to hyloSOL".to_string(), samples);
  }

  // Swaps
  {
    print!("   Swap HYUSD to XSOL... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<HYUSD, XSOL>(500_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Swap HYUSD to XSOL".to_string(), samples);
  }

  {
    print!("   Swap XSOL to HYUSD... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<XSOL, HYUSD>(1_000_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Swap XSOL to HYUSD".to_string(), samples);
  }

  // Stability pool
  {
    print!("   Deposit HYUSD... ");
    let mut samples = Vec::new();
    for _ in 0..5 {
      if let Ok(quote) = simulator
        .simulate_quote::<HYUSD, SHYUSD>(500_000, reference_wallet, 50)
        .await
      {
        if matches!(quote.compute_unit_method, ComputeUnitMethod::Simulated) {
          samples.push(quote.compute_units);
        }
      }
    }
    print_route_results(&samples);
    route_samples.insert("Deposit HYUSD".to_string(), samples);
  }

  println!();

  println!("\n=== Calibrated Values ===\n");
  println!("// Compute unit defaults from calibration");
  let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs();
  println!("// Generated: {timestamp} (Unix timestamp)");
  println!("//");
  println!("// Measured ATA creation cost: {avg_ata_cost} CU");
  println!("// Route operations measured with existing ATAs (warm path)");
  println!(
    "// For production, add ATA buffer to ensure first-time transactions \
     succeed"
  );
  println!();

  // Group by operation type and calculate averages
  let mut mint_stable = Vec::new();
  let mut redeem_stable = Vec::new();
  let mut mint_lever = Vec::new();
  let mut redeem_lever = Vec::new();
  let mut swaps = Vec::new();
  let mut stability = Vec::new();

  for (description, samples) in &route_samples {
    if samples.is_empty() {
      continue;
    }
    let avg = samples.iter().sum::<u64>() / samples.len() as u64;

    if description.contains("Mint HYUSD") {
      mint_stable.push(avg);
    } else if description.contains("Redeem HYUSD") {
      redeem_stable.push(avg);
    } else if description.contains("Mint XSOL") {
      mint_lever.push(avg);
    } else if description.contains("Redeem XSOL") {
      redeem_lever.push(avg);
    } else if description.contains("Swap") {
      swaps.push(avg);
    } else if description.contains("Deposit") {
      stability.push(avg);
    }
  }

  fn avg(values: &[u64]) -> u64 {
    if values.is_empty() {
      return 0;
    }
    values.iter().sum::<u64>() / values.len() as u64
  }

  println!("// Base values (warm path - ATAs exist):\n");

  let mint_stable_avg = avg(&mint_stable);
  let redeem_stable_avg = avg(&redeem_stable);
  let mint_lever_avg = avg(&mint_lever);
  let redeem_lever_avg = avg(&redeem_lever);
  let swap_avg = avg(&swaps);
  let stability_avg = avg(&stability);

  if mint_stable_avg > 0 {
    println!("Mint Stablecoin:   {mint_stable_avg} CU");
  }
  if redeem_stable_avg > 0 {
    println!("Redeem Stablecoin: {redeem_stable_avg} CU");
  }
  if mint_lever_avg > 0 {
    println!("Mint Levercoin:    {mint_lever_avg} CU");
  }
  if redeem_lever_avg > 0 {
    println!("Redeem Levercoin:  {redeem_lever_avg} CU");
  }
  if swap_avg > 0 {
    println!("Swap:              {swap_avg} CU");
  }
  if stability_avg > 0 {
    println!("Stability Deposit: {stability_avg} CU");
  }

  println!("\n// Recommended production values:\n");
  println!("// const ATA_CREATION_CU: u64 = {avg_ata_cost};");
  println!("//");
  println!("// For each route:");
  println!("//   const OPERATION_BASE_CU: u64 = <measured_value>;");
  println!("//   const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;");
  println!(
    "//   const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;  // 1.1x"
  );
  println!(
    "//   const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;  // \
     1.5x"
  );
  println!();

  if mint_stable_avg > 0 {
    let base_total = mint_stable_avg + avg_ata_cost;
    let base_with_margin = (base_total * 110) / 100;
    let safe_with_margin = (base_with_margin * 150) / 100;
    println!("Mint Stablecoin:");
    println!("  OPERATION_BASE_CU: {mint_stable_avg}");
    println!("  BASE_WITH_MARGIN:  {base_with_margin}");
    println!("  SAFE_WITH_MARGIN:  {safe_with_margin}");
  }
  if redeem_stable_avg > 0 {
    let base_total = redeem_stable_avg + avg_ata_cost;
    let base_with_margin = (base_total * 110) / 100;
    let safe_with_margin = (base_with_margin * 150) / 100;
    println!("Redeem Stablecoin:");
    println!("  OPERATION_BASE_CU: {redeem_stable_avg}");
    println!("  BASE_WITH_MARGIN:  {base_with_margin}");
    println!("  SAFE_WITH_MARGIN:  {safe_with_margin}");
  }
  if mint_lever_avg > 0 {
    let base_total = mint_lever_avg + avg_ata_cost;
    let base_with_margin = (base_total * 110) / 100;
    let safe_with_margin = (base_with_margin * 150) / 100;
    println!("Mint Levercoin:");
    println!("  OPERATION_BASE_CU: {mint_lever_avg}");
    println!("  BASE_WITH_MARGIN:  {base_with_margin}");
    println!("  SAFE_WITH_MARGIN:  {safe_with_margin}");
  }
  if redeem_lever_avg > 0 {
    let base_total = redeem_lever_avg + avg_ata_cost;
    let base_with_margin = (base_total * 110) / 100;
    let safe_with_margin = (base_with_margin * 150) / 100;
    println!("Redeem Levercoin:");
    println!("  OPERATION_BASE_CU: {redeem_lever_avg}");
    println!("  BASE_WITH_MARGIN:  {base_with_margin}");
    println!("  SAFE_WITH_MARGIN:  {safe_with_margin}");
  }
  if swap_avg > 0 {
    let base_total = swap_avg + avg_ata_cost;
    let base_with_margin = (base_total * 110) / 100;
    let safe_with_margin = (base_with_margin * 150) / 100;
    println!("Swap:");
    println!("  OPERATION_BASE_CU: {swap_avg}");
    println!("  BASE_WITH_MARGIN:  {base_with_margin}");
    println!("  SAFE_WITH_MARGIN:  {safe_with_margin}");
  }
  if stability_avg > 0 {
    let base_total = stability_avg + avg_ata_cost;
    let base_with_margin = (base_total * 110) / 100;
    let safe_with_margin = (base_with_margin * 150) / 100;
    println!("Stability Deposit:");
    println!("  OPERATION_BASE_CU: {stability_avg}");
    println!("  BASE_WITH_MARGIN:  {base_with_margin}");
    println!("  SAFE_WITH_MARGIN:  {safe_with_margin}");
  }

  println!("\n=== Individual Route Details ===\n");

  let mut sorted_routes: Vec<_> = route_samples.iter().collect();
  sorted_routes.sort_by_key(|(desc, _)| desc.as_str());

  for (description, samples) in sorted_routes {
    if samples.is_empty() {
      println!("{description}:");
      println!("  Samples: 0 (all failed)");
      println!();
      continue;
    }
    let avg = samples.iter().sum::<u64>() / samples.len() as u64;
    let min = *samples.iter().min().unwrap();
    let max = *samples.iter().max().unwrap();

    println!("{description}:");
    println!("  Samples: {}", samples.len());
    println!("  Min:     {min} CU");
    println!("  Max:     {max} CU");
    println!("  Average: {avg} CU");
    println!();
  }

  println!("✅ Calibration complete!");
  println!("\nUpdate quote_computer with the production values above.");
  println!("These include the {avg_ata_cost} CU buffer for ATA creation.");

  Ok(())
}
