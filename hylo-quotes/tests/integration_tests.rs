//! Integration tests for quote provider against mainnet
//!
//! These tests validate quote computation across different strategies:
//! - `ProtocolStateStrategy`: Uses protocol state and SDK math
//! - `SimulationStrategy`: Uses transaction simulation
//!
//! Set `RPC_URL` environment variable to run against mainnet.
//! Tests fail if `RPC_URL` is not set.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::Cluster;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::clock::Clock;
use anchor_lang::AccountDeserialize;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::TokenAccount;
use anyhow::{anyhow, Context, Result};
use flaky_test::flaky_test;
use hylo_clients::prelude::{
  ExchangeClient, ProgramClient, StabilityPoolClient,
};
use hylo_clients::util::REFERENCE_WALLET;
use hylo_core::stability_mode::StabilityMode;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_quotes::protocol_state::{
  ProtocolState, RpcStateProvider, StateProvider,
};
use hylo_quotes::{
  ProtocolStateStrategy, Quote, QuoteMetadata, RuntimeQuoteStrategy,
  SimulationStrategy,
};

/// Test context with shared setup for integration tests
struct TestContext {
  /// Protocol state loaded upfront
  protocol_state: ProtocolState<Clock>,
  /// Token balances for reference wallet, keyed by mint pubkey
  balances: HashMap<Pubkey, u64>,
  /// Protocol state strategy provider
  protocol_state_strategy: ProtocolStateStrategy<Arc<RpcStateProvider>>,
  /// Simulation strategy provider
  simulation_strategy: SimulationStrategy,
  /// Reference wallet pubkey
  reference_wallet: Pubkey,
}

impl TestContext {
  /// Create test context from environment
  ///
  /// Loads protocol state and wallet balances upfront.
  ///
  /// # Errors
  /// Returns error if `RPC_URL` environment variable is not set or if
  /// state/balance loading fails
  async fn new() -> Result<Self> {
    let rpc_url = std::env::var("RPC_URL")
      .map_err(|_| anyhow!("RPC_URL environment variable not set"))?;

    let rpc_client = Arc::new(RpcClient::new_with_commitment(
      rpc_url.clone(),
      CommitmentConfig::confirmed(),
    ));

    // Load protocol state upfront
    let state_provider = Arc::new(RpcStateProvider::new(rpc_client.clone()));

    let protocol_state = state_provider
      .fetch_state()
      .await
      .context("Failed to fetch protocol state")?;

    // Load token balances for reference wallet upfront
    let balances =
      Self::load_wallet_balances(&rpc_client, REFERENCE_WALLET).await?;

    // Create protocol state strategy
    let protocol_state_strategy =
      ProtocolStateStrategy::new(state_provider.clone());

    // Create simulation strategy
    let cluster = Cluster::from_str(&rpc_url)
      .context("Failed to parse RPC_URL as Cluster")?;

    let exchange_client = ExchangeClient::new_random_keypair(
      cluster.clone(),
      CommitmentConfig::confirmed(),
    )?;

    let stability_pool_client = StabilityPoolClient::new_random_keypair(
      cluster,
      CommitmentConfig::confirmed(),
    )?;

    let simulation_strategy =
      SimulationStrategy::new(exchange_client, stability_pool_client);

    Ok(Self {
      protocol_state,
      balances,
      protocol_state_strategy,
      simulation_strategy,
      reference_wallet: REFERENCE_WALLET,
    })
  }

  /// Load token account balances for a wallet
  ///
  /// # Errors
  /// Returns error if RPC calls fail
  async fn load_wallet_balances(
    rpc_client: &RpcClient,
    wallet: Pubkey,
  ) -> Result<HashMap<Pubkey, u64>> {
    let mints = [
      JITOSOL::MINT,
      HYLOSOL::MINT,
      HYUSD::MINT,
      XSOL::MINT,
      SHYUSD::MINT,
    ];

    let ata_pubkeys: Vec<Pubkey> = mints
      .iter()
      .map(|mint| get_associated_token_address(&wallet, mint))
      .collect();

    let accounts = rpc_client
      .get_multiple_accounts(&ata_pubkeys)
      .await
      .context("Failed to fetch token accounts")?;

    let mut balances = HashMap::new();
    for (mint, account_opt) in mints.iter().zip(accounts.iter()) {
      if let Some(account) = account_opt {
        let token_account =
          TokenAccount::try_deserialize(&mut account.data.as_slice())
            .context("Failed to deserialize token account")?;
        balances.insert(*mint, token_account.amount);
      } else {
        // Account doesn't exist, balance is 0
        balances.insert(*mint, 0);
      }
    }

    Ok(balances)
  }

  /// Get balance for a specific mint
  #[must_use]
  fn get_balance(&self, mint: &Pubkey) -> u64 {
    self.balances.get(mint).copied().unwrap_or(0)
  }

  /// Get current stability mode
  #[must_use]
  fn stability_mode(&self) -> StabilityMode {
    self.protocol_state.exchange_context.stability_mode
  }

  /// Check if XSOL is present in stability pool
  #[must_use]
  fn has_xsol_in_pool(&self) -> bool {
    self.protocol_state.xsol_pool.amount > 0
  }
}

/// Test case for quote operations
struct QuoteTestCase {
  name: &'static str,
  description: &'static str,
  input_mint: Pubkey,
  output_mint: Pubkey,
  amount: u64,
  expected: ExpectedResult,
}

/// Expected result for a test case
enum ExpectedResult {
  /// Quote should succeed
  Success,
  /// Quote should fail with error containing this string
  ErrorContains(&'static str),
  /// Quote should fail due to insufficient balance
  InsufficientBalance,
  /// Quote should fail due to protocol mode restrictions
  ModeRestricted,
  /// Quote should fail due to stability pool state (e.g., XSOL present)
  PoolStateRestricted,
}

impl QuoteTestCase {
  #[must_use]
  fn should_skip(&self, ctx: &TestContext) -> bool {
    match self.expected {
      ExpectedResult::PoolStateRestricted => {
        if self.input_mint == SHYUSD::MINT && self.output_mint == HYUSD::MINT {
          !ctx.has_xsol_in_pool()
        } else {
          false
        }
      }
      _ => false,
    }
  }

  #[must_use]
  fn has_insufficient_balance(&self, ctx: &TestContext) -> bool {
    self.amount > ctx.get_balance(&self.input_mint)
  }

  fn expected_error_protocol_state(
    &self,
    ctx: &TestContext,
  ) -> Option<&'static str> {
    match self.expected {
      ExpectedResult::ModeRestricted => {
        match (ctx.stability_mode(), self.input_mint, self.output_mint) {
          (mode, _, HYUSD::MINT) if mode > StabilityMode::Mode1 => {
            Some("Mint operations disabled in current stability mode")
          }
          (mode, XSOL::MINT, JITOSOL::MINT) if mode > StabilityMode::Mode1 => {
            Some(
              "Cannot redeem levercoin due to stability mode. NAV would be 0 \
               or lower.",
            )
          }
          (mode, XSOL::MINT, HYUSD::MINT) if mode > StabilityMode::Mode1 => {
            Some("Levercoin to stablecoin swap disabled due to stability mode.")
          }
          (mode, HYUSD::MINT, XSOL::MINT) if mode > StabilityMode::Mode1 => {
            Some("Stablecoin to levercoin swap disabled due to stability mode.")
          }
          _ => None,
        }
      }
      ExpectedResult::PoolStateRestricted => {
        match (self.input_mint, self.output_mint) {
          (SHYUSD::MINT, HYUSD::MINT) if ctx.has_xsol_in_pool() => {
            Some("SHYUSD → HYUSD not possible: levercoin present in pool")
          }
          _ => None,
        }
      }
      _ => None,
    }
  }

  fn expected_error_simulation(
    &self,
    ctx: &TestContext,
  ) -> Option<&'static str> {
    match self.expected {
      ExpectedResult::InsufficientBalance => {
        if self.has_insufficient_balance(ctx) {
          Some("Simulation failed")
        } else {
          None
        }
      }
      ExpectedResult::ModeRestricted => {
        match (ctx.stability_mode(), self.input_mint, self.output_mint) {
          (mode, _, HYUSD::MINT) if mode > StabilityMode::Mode1 => {
            Some("Stablecoin mint disabled. Protocol is in Mode2 or Depeg.")
          }
          (mode, XSOL::MINT, JITOSOL::MINT) if mode > StabilityMode::Mode1 => {
            Some(
              "Cannot redeem levercoin due to stability mode. NAV would be 0 \
               or lower.",
            )
          }
          (mode, XSOL::MINT, HYUSD::MINT) if mode > StabilityMode::Mode1 => {
            Some("Levercoin to stablecoin swap disabled due to stability mode.")
          }
          (mode, HYUSD::MINT, XSOL::MINT) if mode > StabilityMode::Mode1 => {
            Some("Stablecoin to levercoin swap disabled due to stability mode.")
          }
          _ => None,
        }
      }
      ExpectedResult::PoolStateRestricted => {
        match (self.input_mint, self.output_mint) {
          (SHYUSD::MINT, HYUSD::MINT) if ctx.has_xsol_in_pool() => {
            Some("SHYUSD → HYUSD not possible: levercoin present in pool")
          }
          _ => None,
        }
      }
      _ => None,
    }
  }
}

// ============================================================================
// Test Cases
// ============================================================================

const TEST_CASES: &[QuoteTestCase] = &[
  QuoteTestCase {
    name: "Mint hyUSD with JitoSOL",
    description: "Success case: mint stablecoin with sufficient JitoSOL \
                  balance",
    input_mint: JITOSOL::MINT,
    output_mint: HYUSD::MINT,
    amount: 100_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Redeem hyUSD for JitoSOL",
    description: "Success case: redeem stablecoin for JitoSOL",
    input_mint: HYUSD::MINT,
    output_mint: JITOSOL::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Mint hyUSD with hyloSOL",
    description: "Insufficient balance: wallet has 0 hyloSOL, \
                  ProtocolStateStrategy succeeds (no balance check), \
                  SimulationStrategy fails",
    input_mint: HYLOSOL::MINT,
    output_mint: HYUSD::MINT,
    amount: 100_000_000,
    expected: ExpectedResult::InsufficientBalance,
  },
  QuoteTestCase {
    name: "Redeem hyUSD for hyloSOL",
    description: "Success case: redeem stablecoin for hyloSOL",
    input_mint: HYUSD::MINT,
    output_mint: HYLOSOL::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Mint xSOL with JitoSOL",
    description: "Success case: mint levercoin with sufficient JitoSOL balance",
    input_mint: JITOSOL::MINT,
    output_mint: XSOL::MINT,
    amount: 100_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Redeem xSOL for JitoSOL",
    description: "Success case: redeem levercoin for JitoSOL",
    input_mint: XSOL::MINT,
    output_mint: JITOSOL::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Mint xSOL with hyloSOL",
    description: "Insufficient balance: wallet has 0 hyloSOL, \
                  ProtocolStateStrategy succeeds (no balance check), \
                  SimulationStrategy fails",
    input_mint: HYLOSOL::MINT,
    output_mint: XSOL::MINT,
    amount: 100_000_000,
    expected: ExpectedResult::InsufficientBalance,
  },
  QuoteTestCase {
    name: "Redeem xSOL for hyloSOL",
    description: "Success case: redeem levercoin for hyloSOL",
    input_mint: XSOL::MINT,
    output_mint: HYLOSOL::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Swap hyUSD to xSOL",
    description: "Success case: swap stablecoin to levercoin",
    input_mint: HYUSD::MINT,
    output_mint: XSOL::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Swap xSOL to hyUSD",
    description: "Success case: swap levercoin to stablecoin",
    input_mint: XSOL::MINT,
    output_mint: HYUSD::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Deposit hyUSD to Stability Pool",
    description: "Success case: deposit stablecoin to stability pool",
    input_mint: HYUSD::MINT,
    output_mint: SHYUSD::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Withdraw hyUSD from Stability Pool",
    description: "Success case: withdraw stablecoin from stability pool (only \
                  when XSOL not in pool)",
    input_mint: SHYUSD::MINT,
    output_mint: HYUSD::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::Success,
  },
  QuoteTestCase {
    name: "Mint hyUSD with excessive JitoSOL",
    description: "Insufficient balance: amount (1.2B) exceeds wallet balance \
                  (1.1B), ProtocolStateStrategy succeeds (no balance check), \
                  SimulationStrategy fails",
    input_mint: JITOSOL::MINT,
    output_mint: HYUSD::MINT,
    amount: 1_200_000_000,
    expected: ExpectedResult::InsufficientBalance,
  },
  QuoteTestCase {
    name: "Redeem excessive hyUSD for JitoSOL",
    description: "Insufficient balance: amount (30M) exceeds wallet balance \
                  (29M), ProtocolStateStrategy succeeds (no balance check), \
                  SimulationStrategy fails",
    input_mint: HYUSD::MINT,
    output_mint: JITOSOL::MINT,
    amount: 30_000_000,
    expected: ExpectedResult::InsufficientBalance,
  },
  QuoteTestCase {
    name: "Mint xSOL with excessive JitoSOL",
    description: "Insufficient balance: amount (1.2B) exceeds wallet balance \
                  (1.1B), ProtocolStateStrategy succeeds (no balance check), \
                  SimulationStrategy fails",
    input_mint: JITOSOL::MINT,
    output_mint: XSOL::MINT,
    amount: 1_200_000_000,
    expected: ExpectedResult::InsufficientBalance,
  },
  QuoteTestCase {
    name: "Redeem excessive xSOL for JitoSOL",
    description: "Insufficient balance: amount (25M) exceeds wallet balance \
                  (24M), ProtocolStateStrategy succeeds (no balance check), \
                  SimulationStrategy fails",
    input_mint: XSOL::MINT,
    output_mint: JITOSOL::MINT,
    amount: 25_000_000,
    expected: ExpectedResult::InsufficientBalance,
  },
  QuoteTestCase {
    name: "Deposit excessive hyUSD to Stability Pool",
    description: "Insufficient balance: amount (30M) exceeds wallet balance \
                  (29M), ProtocolStateStrategy succeeds (no balance check), \
                  SimulationStrategy fails",
    input_mint: HYUSD::MINT,
    output_mint: SHYUSD::MINT,
    amount: 30_000_000,
    expected: ExpectedResult::InsufficientBalance,
  },
  QuoteTestCase {
    name: "Withdraw excessive sHYUSD from Stability Pool",
    description: "Insufficient balance: amount (19M) exceeds wallet balance \
                  (18M), ProtocolStateStrategy succeeds (no balance check), \
                  SimulationStrategy fails",
    input_mint: SHYUSD::MINT,
    output_mint: HYUSD::MINT,
    amount: 19_000_000,
    expected: ExpectedResult::InsufficientBalance,
  },
  QuoteTestCase {
    name: "Mint hyUSD when mode restricts minting",
    description: "Mode restriction: minting disabled when stability mode > \
                  Mode1 (currently Mode1, so succeeds; would fail in \
                  Mode2/Depeg)",
    input_mint: JITOSOL::MINT,
    output_mint: HYUSD::MINT,
    amount: 100_000_000,
    expected: ExpectedResult::ModeRestricted,
  },
  QuoteTestCase {
    name: "Withdraw hyUSD when XSOL in pool",
    description: "Pool state restriction: SHYUSD → HYUSD withdrawal fails \
                  when levercoin present in stability pool",
    input_mint: SHYUSD::MINT,
    output_mint: HYUSD::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::PoolStateRestricted,
  },
  QuoteTestCase {
    name: "Unsupported pair (XSOL to SHYUSD)",
    description: "Unsupported operation: XSOL → SHYUSD is not a valid quote \
                  operation",
    input_mint: XSOL::MINT,
    output_mint: SHYUSD::MINT,
    amount: 1_000_000,
    expected: ExpectedResult::ErrorContains("Unsupported pair"),
  },
];

// ============================================================================
// Test Functions
// ============================================================================

#[flaky_test(tokio, times = 3)]
#[allow(clippy::too_many_lines)]
async fn test_quote_provider_protocol_state() -> Result<()> {
  let ctx = TestContext::new().await?;

  for test_case in TEST_CASES {
    if test_case.should_skip(&ctx) {
      eprintln!(
        "Skipping {}: {} - conditions not met",
        test_case.name, test_case.description
      );
      continue;
    }

    let result = ctx
      .protocol_state_strategy
      .runtime_quote_with_metadata(
        test_case.input_mint,
        test_case.output_mint,
        test_case.amount,
        ctx.reference_wallet,
        50, // 0.5% slippage tolerance
      )
      .await;

    match &test_case.expected {
      ExpectedResult::Success => {
        let (quote, metadata) = result.with_context(|| {
          format!("{}: Expected success but got error", test_case.name)
        })?;
        validate_quote(&quote, &metadata, test_case.name);
      }
      ExpectedResult::ErrorContains(expected_msg) => {
        let err = result.expect_err(&format!(
          "{}: Expected error but got success",
          test_case.name
        ));
        assert!(
          err.to_string().contains(expected_msg),
          "{}: Error message '{}' does not contain '{}'",
          test_case.name,
          err,
          expected_msg
        );
      }
      ExpectedResult::InsufficientBalance => {
        if !test_case.has_insufficient_balance(&ctx) {
          eprintln!(
            "{}: {} - Skipping: wallet has sufficient balance",
            test_case.name, test_case.description
          );
          continue;
        }
        match result {
          Ok((quote, _)) => {
            eprintln!(
              "{}: {} - ProtocolStateStrategy does not check balances, got \
               quote: amount_in={}, amount_out={}",
              test_case.name,
              test_case.description,
              quote.amount_in,
              quote.amount_out
            );
          }
          Err(err) => {
            panic!(
              "{}: {} - ProtocolStateStrategy should succeed (doesn't check \
               balances) but got error: {}",
              test_case.name, test_case.description, err
            );
          }
        }
      }
      ExpectedResult::ModeRestricted => {
        let mode = ctx.stability_mode();
        if mode > StabilityMode::Mode1 {
          let expected_err =
            test_case.expected_error_protocol_state(&ctx).expect(
              "ModeRestricted test case must have expected error in \
               restricted mode",
            );

          let err = result.expect_err(&format!(
            "{}: {} - Expected mode restriction error in {:?}",
            test_case.name, test_case.description, mode
          ));
          let err_str = err.to_string();

          assert!(
            err_str.contains(expected_err),
            "{}: {} - Expected error containing '{}', got: {}",
            test_case.name,
            test_case.description,
            expected_err,
            err_str
          );
        } else {
          let (quote, metadata) = result.with_context(|| {
            format!(
              "{}: {} - Expected success in {:?} mode",
              test_case.name, test_case.description, mode
            )
          })?;
          validate_quote(&quote, &metadata, test_case.name);
        }
      }
      ExpectedResult::PoolStateRestricted => {
        if ctx.has_xsol_in_pool() {
          let expected_err =
            test_case.expected_error_protocol_state(&ctx).expect(
              "PoolStateRestricted test case must have expected error when \
               XSOL in pool",
            );

          let err = result.expect_err(&format!(
            "{}: {} - Expected pool state restriction error",
            test_case.name, test_case.description
          ));
          let err_str = err.to_string();

          assert!(
            err_str.contains(expected_err),
            "{}: {} - Expected error containing '{}', got: {}",
            test_case.name,
            test_case.description,
            expected_err,
            err_str
          );
        } else {
          let (quote, metadata) = result.with_context(|| {
            format!(
              "{}: {} - Expected success: pool state allows",
              test_case.name, test_case.description
            )
          })?;
          validate_quote(&quote, &metadata, test_case.name);
        }
      }
    }
  }

  Ok(())
}

#[flaky_test(tokio, times = 3)]
#[allow(clippy::too_many_lines)]
async fn test_quote_provider_simulation() -> Result<()> {
  let ctx = TestContext::new().await?;

  for test_case in TEST_CASES {
    if test_case.should_skip(&ctx) {
      eprintln!(
        "Skipping {}: {} - conditions not met",
        test_case.name, test_case.description
      );
      continue;
    }

    let result = ctx
      .simulation_strategy
      .runtime_quote_with_metadata(
        test_case.input_mint,
        test_case.output_mint,
        test_case.amount,
        ctx.reference_wallet,
        50, // 0.5% slippage tolerance
      )
      .await;

    match &test_case.expected {
      ExpectedResult::Success => {
        let (quote, metadata) = result.with_context(|| {
          format!(
            "{}: {} - Expected success but got error",
            test_case.name, test_case.description
          )
        })?;
        validate_quote(&quote, &metadata, test_case.name);
      }
      ExpectedResult::ErrorContains(expected_msg) => {
        let err = result.expect_err(&format!(
          "{}: {} - Expected error but got success",
          test_case.name, test_case.description
        ));
        assert!(
          err.to_string().contains(expected_msg),
          "{}: {} - Error message '{}' does not contain '{}'",
          test_case.name,
          test_case.description,
          err,
          expected_msg
        );
      }
      ExpectedResult::InsufficientBalance => {
        if !test_case.has_insufficient_balance(&ctx) {
          eprintln!(
            "{}: {} - Skipping: wallet has sufficient balance",
            test_case.name, test_case.description
          );
          continue;
        }
        let expected_err = test_case
          .expected_error_simulation(&ctx)
          .expect("InsufficientBalance test case must have expected error");

        let err = result.expect_err(&format!(
          "{}: {} - SimulationStrategy should fail with insufficient balance",
          test_case.name, test_case.description
        ));
        let err_str = err.to_string();

        assert!(
          err_str.contains(expected_err),
          "{}: {} - Expected error containing '{}', got: {}",
          test_case.name,
          test_case.description,
          expected_err,
          err_str
        );
      }
      ExpectedResult::ModeRestricted => {
        let mode = ctx.stability_mode();
        if mode > StabilityMode::Mode1 {
          let expected_err = test_case.expected_error_simulation(&ctx).expect(
            "ModeRestricted test case must have expected error in restricted \
             mode",
          );

          let err = result.expect_err(&format!(
            "{}: {} - Expected mode restriction error in {:?}",
            test_case.name, test_case.description, mode
          ));
          let err_str = err.to_string();

          assert!(
            err_str.contains(expected_err),
            "{}: {} - Expected error containing '{}', got: {}",
            test_case.name,
            test_case.description,
            expected_err,
            err_str
          );
        } else {
          let (quote, metadata) = result.with_context(|| {
            format!(
              "{}: {} - Expected success in {:?} mode",
              test_case.name, test_case.description, mode
            )
          })?;
          validate_quote(&quote, &metadata, test_case.name);
        }
      }
      ExpectedResult::PoolStateRestricted => {
        if ctx.has_xsol_in_pool() {
          let expected_err = test_case.expected_error_simulation(&ctx).expect(
            "PoolStateRestricted test case must have expected error when XSOL \
             in pool",
          );

          let err = result.expect_err(&format!(
            "{}: {} - Expected pool state restriction error",
            test_case.name, test_case.description
          ));
          let err_str = err.to_string();

          assert!(
            err_str.contains(expected_err),
            "{}: {} - Expected error containing '{}', got: {}",
            test_case.name,
            test_case.description,
            expected_err,
            err_str
          );
        } else {
          let (quote, metadata) = result.with_context(|| {
            format!(
              "{}: {} - Expected success: pool state allows",
              test_case.name, test_case.description
            )
          })?;
          validate_quote(&quote, &metadata, test_case.name);
        }
      }
    }
  }

  Ok(())
}

/// Validate that a quote has reasonable values
fn validate_quote(quote: &Quote, metadata: &QuoteMetadata, test_name: &str) {
  assert!(
    quote.amount_in > 0,
    "{test_name}: amount_in should be > 0, got {}",
    quote.amount_in
  );
  assert!(
    quote.amount_out > 0,
    "{test_name}: amount_out should be > 0, got {}",
    quote.amount_out
  );
  assert!(
    quote.compute_units > 0,
    "{test_name}: compute_units should be > 0, got {}",
    quote.compute_units
  );
  assert!(
    !quote.instructions.is_empty(),
    "{test_name}: instructions should not be empty"
  );
  assert!(
    !metadata.description.is_empty(),
    "{test_name}: metadata description should not be empty"
  );
}

#[flaky_test(tokio, times = 3)]
async fn test_reference_wallet_state() -> Result<()> {
  let ctx = TestContext::new().await?;

  assert_eq!(
    ctx.reference_wallet, REFERENCE_WALLET,
    "Reference wallet should match expected"
  );

  let expected_balances: HashMap<Pubkey, (&'static str, u64)> = [
    (JITOSOL::MINT, ("JITOSOL", 1_118_607_723)),
    (HYLOSOL::MINT, ("HYLOSOL", 0)),
    (HYUSD::MINT, ("HYUSD", 29_019_779)),
    (XSOL::MINT, ("XSOL", 24_291_587)),
    (SHYUSD::MINT, ("SHYUSD", 18_196_458)),
  ]
  .into_iter()
  .collect();

  let mut mismatches = Vec::new();
  for (mint, (token_name, expected_balance)) in &expected_balances {
    let actual_balance = ctx.get_balance(mint);
    if actual_balance != *expected_balance {
      mismatches.push(format!(
        "  {token_name} ({mint}): expected {expected_balance}, got \
         {actual_balance}"
      ));
    }
  }

  assert!(
    !mismatches.is_empty(),
    "Reference wallet balance mismatches:\n{}",
    mismatches.join("\n")
  );

  Ok(())
}
