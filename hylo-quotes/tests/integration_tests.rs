//! Integration tests comparing ProtocolStateStrategy vs SimulationStrategy.
//!
//! Requires `RPC_URL` environment variable.

use std::str::FromStr;
use std::sync::Arc;

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::Cluster;
use anyhow::{Context, Result};
use hylo_clients::prelude::{
  ExchangeClient, ProgramClient, StabilityPoolClient,
};
use hylo_clients::util::REFERENCE_WALLET;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_quotes::prelude::{
  ProtocolStateStrategy, RpcStateProvider, RuntimeQuoteStrategy,
  SimulationStrategy,
};
use hylo_quotes::ExecutableQuoteValue;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use test_context::{test_context, AsyncTestContext};
use tokio::join;

const SLIPPAGE_BPS: u64 = 50;
const ONE_LST: u64 = 1_000_000_000;
const ONE_TOKEN: u64 = 1_000_000;

struct QuoteStrategyTestContext {
  protocol_state_strategy: ProtocolStateStrategy<Arc<RpcStateProvider>>,
  simulation_strategy: SimulationStrategy,
}

impl AsyncTestContext for QuoteStrategyTestContext {
  async fn setup() -> Self {
    Self::new().expect("Failed to create QuoteStrategyTestContext")
  }
}

impl QuoteStrategyTestContext {
  fn new() -> Result<Self> {
    let rpc_url = std::env::var("RPC_URL")?;

    let rpc_client = Arc::new(RpcClient::new_with_commitment(
      rpc_url.clone(),
      CommitmentConfig::confirmed(),
    ));

    let state_provider = Arc::new(RpcStateProvider::new(rpc_client));
    let protocol_state_strategy = ProtocolStateStrategy::new(state_provider);

    let cluster =
      Cluster::from_str(&rpc_url).context("Failed to parse RPC_URL")?;

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
      protocol_state_strategy,
      simulation_strategy,
    })
  }
}

fn assert_quotes_match(
  state_result: Result<ExecutableQuoteValue>,
  sim_result: Result<ExecutableQuoteValue>,
) {
  match (state_result, sim_result) {
    (Ok(state), Ok(sim)) => assert_eq!(state.amount_out, sim.amount_out),
    (Err(_), Err(_)) => (),
    (Ok(_), Err(e)) => {
      panic!("simulation failed but state succeeded: {e}");
    }
    (Err(e), Ok(_)) => {
      panic!("state failed but simulation succeeded: {e}");
    }
  }
}

#[test_context(QuoteStrategyTestContext)]
#[tokio::test]
async fn jitosol_to_hyusd(ctx: &QuoteStrategyTestContext) {
  let (state, sim) = join!(
    ctx.protocol_state_strategy.runtime_quote(
      JITOSOL::MINT,
      HYUSD::MINT,
      ONE_LST,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
    ctx.simulation_strategy.runtime_quote(
      JITOSOL::MINT,
      HYUSD::MINT,
      ONE_LST,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
  );
  assert_quotes_match(state, sim);
}

#[test_context(QuoteStrategyTestContext)]
#[tokio::test]
async fn hyusd_to_jitosol(ctx: &QuoteStrategyTestContext) {
  let (state, sim) = join!(
    ctx.protocol_state_strategy.runtime_quote(
      HYUSD::MINT,
      JITOSOL::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
    ctx.simulation_strategy.runtime_quote(
      HYUSD::MINT,
      JITOSOL::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
  );
  assert_quotes_match(state, sim);
}

#[test_context(QuoteStrategyTestContext)]
#[tokio::test]
async fn jitosol_to_xsol(ctx: &QuoteStrategyTestContext) {
  let (state, sim) = join!(
    ctx.protocol_state_strategy.runtime_quote(
      JITOSOL::MINT,
      XSOL::MINT,
      ONE_LST,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
    ctx.simulation_strategy.runtime_quote(
      JITOSOL::MINT,
      XSOL::MINT,
      ONE_LST,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
  );
  assert_quotes_match(state, sim);
}

#[test_context(QuoteStrategyTestContext)]
#[tokio::test]
async fn xsol_to_jitosol(ctx: &QuoteStrategyTestContext) {
  let (state, sim) = join!(
    ctx.protocol_state_strategy.runtime_quote(
      XSOL::MINT,
      JITOSOL::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
    ctx.simulation_strategy.runtime_quote(
      XSOL::MINT,
      JITOSOL::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
  );
  assert_quotes_match(state, sim);
}

#[test_context(QuoteStrategyTestContext)]
#[tokio::test]
async fn hyusd_to_xsol(ctx: &QuoteStrategyTestContext) {
  let (state, sim) = join!(
    ctx.protocol_state_strategy.runtime_quote(
      HYUSD::MINT,
      XSOL::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
    ctx.simulation_strategy.runtime_quote(
      HYUSD::MINT,
      XSOL::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
  );
  assert_quotes_match(state, sim);
}

#[test_context(QuoteStrategyTestContext)]
#[tokio::test]
async fn xsol_to_hyusd(ctx: &QuoteStrategyTestContext) {
  let (state, sim) = join!(
    ctx.protocol_state_strategy.runtime_quote(
      XSOL::MINT,
      HYUSD::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
    ctx.simulation_strategy.runtime_quote(
      XSOL::MINT,
      HYUSD::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
  );
  assert_quotes_match(state, sim);
}

#[test_context(QuoteStrategyTestContext)]
#[tokio::test]
async fn jitosol_to_hylosol(ctx: &QuoteStrategyTestContext) {
  let (state, sim) = join!(
    ctx.protocol_state_strategy.runtime_quote(
      JITOSOL::MINT,
      HYLOSOL::MINT,
      ONE_LST,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
    ctx.simulation_strategy.runtime_quote(
      JITOSOL::MINT,
      HYLOSOL::MINT,
      ONE_LST,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
  );
  assert_quotes_match(state, sim);
}

#[test_context(QuoteStrategyTestContext)]
#[tokio::test]
async fn hyusd_to_shyusd(ctx: &QuoteStrategyTestContext) {
  let (state, sim) = join!(
    ctx.protocol_state_strategy.runtime_quote(
      HYUSD::MINT,
      SHYUSD::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
    ctx.simulation_strategy.runtime_quote(
      HYUSD::MINT,
      SHYUSD::MINT,
      ONE_TOKEN,
      REFERENCE_WALLET,
      SLIPPAGE_BPS
    ),
  );
  assert_quotes_match(state, sim);
}
