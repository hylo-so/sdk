//! `max_input` parity tests against live protocol state.
//!
//! Requires `RPC_URL` environment variable.

use std::sync::Arc;

use anchor_lang::prelude::Clock;
use anyhow::Result;
use fix::prelude::*;
use fix::typenum::Integer;
use hylo_idl::tokens::{
  TokenMint, CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC, XSOL,
};
use hylo_quotes::prelude::{ProtocolState, RpcStateProvider, StateProvider};
use hylo_quotes::token_operation::{TokenOperation, TokenOperationExt};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

/// Live protocol state, or `None` when the deployment is not quotable
/// (missing env, stale shadow harvest, oracle outage).
async fn live_state() -> Option<ProtocolState<Clock>> {
  let url = std::env::var("RPC_URL").ok()?;
  let provider = RpcStateProvider::new(Arc::new(RpcClient::new(url)));
  provider.fetch_state().await.ok()
}

/// Parity between `max_input` and `compute_output`: the reported max
/// quotes successfully, one atom more fails, and a gated route reports
/// the gate error for any size.
fn assert_max_input_parity<IN, OUT>(state: &ProtocolState<Clock>, route: &str)
where
  IN: TokenMint,
  OUT: TokenMint,
  ProtocolState<Clock>: TokenOperation<IN, OUT>,
  <IN as TokenMint>::Exp: Integer,
{
  match TokenOperation::<IN, OUT>::max_input(state) {
    Ok(max) => {
      assert!(
        state.output::<IN, OUT>(max).is_ok(),
        "{route}: quote at max_input {} failed",
        max.bits
      );
      assert!(
        max.bits == u64::MAX
          || state.output::<IN, OUT>(UFix64::new(max.bits + 1)).is_err(),
        "{route}: quote at max_input + 1 succeeded ({})",
        max.bits + 1
      );
    }
    Err(gate_error) => {
      let quote = state.output::<IN, OUT>(UFix64::new(1_000_000));
      assert!(
        quote.is_err(),
        "{route}: max_input gated with {gate_error} but quote succeeded"
      );
    }
  }
}

#[tokio::test]
async fn max_input_parity_all_routes() -> Result<()> {
  let Some(state) = live_state().await else {
    return Ok(());
  };
  assert_max_input_parity::<JITOSOL, HYUSD>(&state, "JITOSOL->HYUSD");
  assert_max_input_parity::<HYUSD, JITOSOL>(&state, "HYUSD->JITOSOL");
  assert_max_input_parity::<HYLOSOL, HYUSD>(&state, "HYLOSOL->HYUSD");
  assert_max_input_parity::<HYUSD, HYLOSOL>(&state, "HYUSD->HYLOSOL");
  assert_max_input_parity::<JITOSOL, XSOL>(&state, "JITOSOL->XSOL");
  assert_max_input_parity::<XSOL, JITOSOL>(&state, "XSOL->JITOSOL");
  assert_max_input_parity::<HYLOSOL, XSOL>(&state, "HYLOSOL->XSOL");
  assert_max_input_parity::<XSOL, HYLOSOL>(&state, "XSOL->HYLOSOL");
  assert_max_input_parity::<HYUSD, XSOL>(&state, "HYUSD->XSOL");
  assert_max_input_parity::<XSOL, HYUSD>(&state, "XSOL->HYUSD");
  assert_max_input_parity::<JITOSOL, HYLOSOL>(&state, "JITOSOL->HYLOSOL");
  assert_max_input_parity::<HYLOSOL, JITOSOL>(&state, "HYLOSOL->JITOSOL");
  assert_max_input_parity::<JITOSOL, USDC>(&state, "JITOSOL->USDC");
  assert_max_input_parity::<HYLOSOL, USDC>(&state, "HYLOSOL->USDC");
  assert_max_input_parity::<USDC, JITOSOL>(&state, "USDC->JITOSOL");
  assert_max_input_parity::<USDC, HYLOSOL>(&state, "USDC->HYLOSOL");
  assert_max_input_parity::<CBBTC, USDC>(&state, "CBBTC->USDC");
  assert_max_input_parity::<USDC, CBBTC>(&state, "USDC->CBBTC");
  assert_max_input_parity::<HYUSD, SHYUSD>(&state, "HYUSD->SHYUSD");
  assert_max_input_parity::<SHYUSD, HYUSD>(&state, "SHYUSD->HYUSD");
  assert_max_input_parity::<USDC, HYUSD>(&state, "USDC->HYUSD");
  assert_max_input_parity::<HYUSD, USDC>(&state, "HYUSD->USDC");
  assert_max_input_parity::<CBBTC, HYUSD>(&state, "CBBTC->HYUSD");
  assert_max_input_parity::<HYUSD, CBBTC>(&state, "HYUSD->CBBTC");
  assert_max_input_parity::<CBBTC, XBTC>(&state, "CBBTC->XBTC");
  assert_max_input_parity::<XBTC, CBBTC>(&state, "XBTC->CBBTC");
  assert_max_input_parity::<HYUSD, XBTC>(&state, "HYUSD->XBTC");
  assert_max_input_parity::<XBTC, HYUSD>(&state, "XBTC->HYUSD");
  Ok(())
}

#[tokio::test]
async fn runtime_max_input_dispatch() -> Result<()> {
  let Some(state) = live_state().await else {
    return Ok(());
  };
  let typed = TokenOperation::<HYUSD, JITOSOL>::max_input(&state);
  let dispatched = state.runtime_max_input(HYUSD::MINT, JITOSOL::MINT);
  match (typed, dispatched) {
    (Ok(max), Ok(bits)) => assert_eq!(max.bits, bits),
    (Err(_), Err(_)) => {}
    (typed, dispatched) => {
      panic!("dispatch mismatch: typed={typed:?} runtime={dispatched:?}")
    }
  }
  assert!(state.runtime_max_input(HYUSD::MINT, HYUSD::MINT).is_err());
  Ok(())
}

/// Marginal rate stays finite, positive, and within 1% of a central
/// finite difference at interior points of the quotable range.
#[tokio::test]
async fn marginal_matches_finite_difference() -> Result<()> {
  let Some(state) = live_state().await else {
    return Ok(());
  };
  let max = match TokenOperation::<HYUSD, JITOSOL>::max_input(&state) {
    Ok(max) => max.bits,
    Err(_) => return Ok(()),
  };
  let quote = |x: u64| {
    state
      .output::<HYUSD, JITOSOL>(UFix64::<N6>::new(x))
      .map(|op| (op.out_amount.bits, op.marginal_rate))
  };
  let delta = (max / 100_000).max(1_000);
  (1..=8u64).try_for_each(|i| -> Result<()> {
    let x = max / 10 * i;
    let (_, marginal) = quote(x)?;
    let (out_lo, _) = quote(x - delta)?;
    let (out_hi, _) = quote(x + delta)?;
    #[allow(clippy::cast_precision_loss)]
    let fd = (out_hi - out_lo) as f64 / (2 * delta) as f64;
    assert!(
      marginal.is_finite() && marginal > 0.0,
      "bad marginal {marginal} at {x}"
    );
    // Windows straddling a fee-curve knot make the central difference
    // average two segment slopes; only smooth points are comparable.
    let (_, marginal_lo) = quote(x - delta)?;
    let (_, marginal_hi) = quote(x + delta)?;
    if (marginal_hi - marginal_lo).abs() / marginal < 1e-3 {
      let rel = (marginal - fd).abs() / fd;
      assert!(
        rel < 0.01,
        "marginal {marginal} vs finite difference {fd} at {x} (rel {rel})"
      );
    }
    Ok(())
  })
}
