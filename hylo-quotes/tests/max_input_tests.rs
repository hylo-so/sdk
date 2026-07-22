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

#[tokio::test]
async fn runtime_min_input_dispatch() -> Result<()> {
  let Some(state) = live_state().await else {
    return Ok(());
  };
  let typed = TokenOperation::<HYUSD, JITOSOL>::min_input(&state);
  let dispatched = state.runtime_min_input(HYUSD::MINT, JITOSOL::MINT);
  match (typed, dispatched) {
    (Ok(min), Ok(bits)) => assert_eq!(min.bits, bits),
    (Err(_), Err(_)) => {}
    (typed, dispatched) => {
      panic!("dispatch mismatch: typed={typed:?} runtime={dispatched:?}")
    }
  }
  assert!(state.runtime_min_input(HYUSD::MINT, HYUSD::MINT).is_err());
  Ok(())
}

/// Parity between `min_input` and `compute_output`: the reported min
/// yields at least one output atom and one input atom less yields none.
fn assert_min_input_parity<IN, OUT>(state: &ProtocolState<Clock>, route: &str)
where
  IN: TokenMint,
  OUT: TokenMint,
  ProtocolState<Clock>: TokenOperation<IN, OUT>,
  <IN as TokenMint>::Exp: Integer,
{
  let min = match TokenOperation::<IN, OUT>::min_input(state) {
    Ok(min) => min,
    Err(gate_error) => {
      let quote = state.output::<IN, OUT>(UFix64::new(1_000_000));
      assert!(
        quote.is_err(),
        "{route}: min_input gated with {gate_error} but quote succeeded"
      );
      return;
    }
  };
  let quotable =
    TokenOperation::<IN, OUT>::max_input(state).is_ok_and(|max| min <= max);
  if quotable {
    match state.output::<IN, OUT>(min) {
      Ok(op) => assert!(
        op.out_amount.bits >= 1,
        "{route}: quote at min_input {} yields no output",
        min.bits
      ),
      Err(error) => {
        panic!("{route}: quote at min_input {} failed: {error}", min.bits)
      }
    }
  }
  let below = state.output::<IN, OUT>(UFix64::new(min.bits - 1));
  assert!(
    below.map_or(true, |op| op.out_amount.bits == 0),
    "{route}: quote below min_input {} yields output",
    min.bits
  );
}

/// Every size on a geometric grid across `[min_input, max_input]`
/// quotes without error.
fn assert_range_soundness<IN, OUT>(state: &ProtocolState<Clock>, route: &str)
where
  IN: TokenMint,
  OUT: TokenMint,
  ProtocolState<Clock>: TokenOperation<IN, OUT>,
  <IN as TokenMint>::Exp: Integer,
{
  let (Ok(min), Ok(max)) = (
    TokenOperation::<IN, OUT>::min_input(state),
    TokenOperation::<IN, OUT>::max_input(state),
  ) else {
    return;
  };
  if min > max {
    return;
  }
  #[allow(clippy::cast_precision_loss)]
  let ratio = (max.bits as f64 / min.bits as f64).powf(1.0 / 63.0);
  (0..64u32).for_each(|i| {
    #[allow(
      clippy::cast_precision_loss,
      clippy::cast_possible_truncation,
      clippy::cast_sign_loss
    )]
    let x = ((min.bits as f64) * ratio.powi(i.cast_signed())) as u64;
    let x = x.clamp(min.bits, max.bits);
    assert!(
      state.output::<IN, OUT>(UFix64::new(x)).is_ok(),
      "{route}: quote failed at {x} inside [{}, {}]",
      min.bits,
      max.bits
    );
  });
}

#[tokio::test]
async fn range_soundness_all_routes() -> Result<()> {
  let Some(state) = live_state().await else {
    return Ok(());
  };
  assert_range_soundness::<JITOSOL, HYUSD>(&state, "JITOSOL->HYUSD");
  assert_range_soundness::<HYUSD, JITOSOL>(&state, "HYUSD->JITOSOL");
  assert_range_soundness::<HYLOSOL, HYUSD>(&state, "HYLOSOL->HYUSD");
  assert_range_soundness::<HYUSD, HYLOSOL>(&state, "HYUSD->HYLOSOL");
  assert_range_soundness::<JITOSOL, XSOL>(&state, "JITOSOL->XSOL");
  assert_range_soundness::<XSOL, JITOSOL>(&state, "XSOL->JITOSOL");
  assert_range_soundness::<HYLOSOL, XSOL>(&state, "HYLOSOL->XSOL");
  assert_range_soundness::<XSOL, HYLOSOL>(&state, "XSOL->HYLOSOL");
  assert_range_soundness::<HYUSD, XSOL>(&state, "HYUSD->XSOL");
  assert_range_soundness::<XSOL, HYUSD>(&state, "XSOL->HYUSD");
  assert_range_soundness::<JITOSOL, HYLOSOL>(&state, "JITOSOL->HYLOSOL");
  assert_range_soundness::<HYLOSOL, JITOSOL>(&state, "HYLOSOL->JITOSOL");
  assert_range_soundness::<JITOSOL, USDC>(&state, "JITOSOL->USDC");
  assert_range_soundness::<HYLOSOL, USDC>(&state, "HYLOSOL->USDC");
  assert_range_soundness::<USDC, JITOSOL>(&state, "USDC->JITOSOL");
  assert_range_soundness::<USDC, HYLOSOL>(&state, "USDC->HYLOSOL");
  assert_range_soundness::<CBBTC, USDC>(&state, "CBBTC->USDC");
  assert_range_soundness::<USDC, CBBTC>(&state, "USDC->CBBTC");
  assert_range_soundness::<HYUSD, SHYUSD>(&state, "HYUSD->SHYUSD");
  assert_range_soundness::<SHYUSD, HYUSD>(&state, "SHYUSD->HYUSD");
  assert_range_soundness::<USDC, HYUSD>(&state, "USDC->HYUSD");
  assert_range_soundness::<HYUSD, USDC>(&state, "HYUSD->USDC");
  assert_range_soundness::<CBBTC, HYUSD>(&state, "CBBTC->HYUSD");
  assert_range_soundness::<HYUSD, CBBTC>(&state, "HYUSD->CBBTC");
  assert_range_soundness::<CBBTC, XBTC>(&state, "CBBTC->XBTC");
  assert_range_soundness::<XBTC, CBBTC>(&state, "XBTC->CBBTC");
  assert_range_soundness::<HYUSD, XBTC>(&state, "HYUSD->XBTC");
  assert_range_soundness::<XBTC, HYUSD>(&state, "XBTC->HYUSD");
  Ok(())
}

#[tokio::test]
async fn min_input_parity_all_routes() -> Result<()> {
  let Some(state) = live_state().await else {
    return Ok(());
  };
  assert_min_input_parity::<JITOSOL, HYUSD>(&state, "JITOSOL->HYUSD");
  assert_min_input_parity::<HYUSD, JITOSOL>(&state, "HYUSD->JITOSOL");
  assert_min_input_parity::<HYLOSOL, HYUSD>(&state, "HYLOSOL->HYUSD");
  assert_min_input_parity::<HYUSD, HYLOSOL>(&state, "HYUSD->HYLOSOL");
  assert_min_input_parity::<JITOSOL, XSOL>(&state, "JITOSOL->XSOL");
  assert_min_input_parity::<XSOL, JITOSOL>(&state, "XSOL->JITOSOL");
  assert_min_input_parity::<HYLOSOL, XSOL>(&state, "HYLOSOL->XSOL");
  assert_min_input_parity::<XSOL, HYLOSOL>(&state, "XSOL->HYLOSOL");
  assert_min_input_parity::<HYUSD, XSOL>(&state, "HYUSD->XSOL");
  assert_min_input_parity::<XSOL, HYUSD>(&state, "XSOL->HYUSD");
  assert_min_input_parity::<JITOSOL, HYLOSOL>(&state, "JITOSOL->HYLOSOL");
  assert_min_input_parity::<HYLOSOL, JITOSOL>(&state, "HYLOSOL->JITOSOL");
  assert_min_input_parity::<JITOSOL, USDC>(&state, "JITOSOL->USDC");
  assert_min_input_parity::<HYLOSOL, USDC>(&state, "HYLOSOL->USDC");
  assert_min_input_parity::<USDC, JITOSOL>(&state, "USDC->JITOSOL");
  assert_min_input_parity::<USDC, HYLOSOL>(&state, "USDC->HYLOSOL");
  assert_min_input_parity::<CBBTC, USDC>(&state, "CBBTC->USDC");
  assert_min_input_parity::<USDC, CBBTC>(&state, "USDC->CBBTC");
  assert_min_input_parity::<HYUSD, SHYUSD>(&state, "HYUSD->SHYUSD");
  assert_min_input_parity::<SHYUSD, HYUSD>(&state, "SHYUSD->HYUSD");
  assert_min_input_parity::<USDC, HYUSD>(&state, "USDC->HYUSD");
  assert_min_input_parity::<HYUSD, USDC>(&state, "HYUSD->USDC");
  assert_min_input_parity::<CBBTC, HYUSD>(&state, "CBBTC->HYUSD");
  assert_min_input_parity::<HYUSD, CBBTC>(&state, "HYUSD->CBBTC");
  assert_min_input_parity::<CBBTC, XBTC>(&state, "CBBTC->XBTC");
  assert_min_input_parity::<XBTC, CBBTC>(&state, "XBTC->CBBTC");
  assert_min_input_parity::<HYUSD, XBTC>(&state, "HYUSD->XBTC");
  assert_min_input_parity::<XBTC, HYUSD>(&state, "XBTC->HYUSD");
  Ok(())
}

/// Marginal rate stays finite, positive, and within 1% of a central
/// finite difference at interior points of the quotable range.
fn assert_marginal_tracks_quotes<IN, OUT>(
  state: &ProtocolState<Clock>,
  route: &str,
) where
  IN: TokenMint,
  OUT: TokenMint,
  ProtocolState<Clock>: TokenOperation<IN, OUT>,
  <IN as TokenMint>::Exp: Integer,
{
  let Ok(max) = TokenOperation::<IN, OUT>::max_input(state) else {
    return;
  };
  let max = max.bits;
  if max == 0 || max == u64::MAX {
    return;
  }
  let quote = |x: u64| {
    state
      .output::<IN, OUT>(UFix64::new(x))
      .map(|op| (op.out_amount.bits, op.marginal_rate))
  };
  let delta = (max / 100_000).max(1_000);
  (1..=8u64).for_each(|i| {
    let x = max / 10 * i;
    let (
      Ok((_, marginal)),
      Ok((out_lo, marginal_lo)),
      Ok((out_hi, marginal_hi)),
    ) = (quote(x), quote(x - delta), quote(x + delta))
    else {
      return;
    };
    assert!(
      marginal.is_finite() && marginal > 0.0,
      "{route}: bad marginal {marginal} at {x}"
    );
    #[allow(clippy::cast_precision_loss)]
    let fd = (out_hi.saturating_sub(out_lo)) as f64 / (2 * delta) as f64;
    // Windows straddling a fee-curve knot make the central difference
    // average two segment slopes; only smooth points are comparable.
    if fd > 0.0 && (marginal_hi - marginal_lo).abs() / marginal < 1e-3 {
      let rel = (marginal - fd).abs() / fd;
      assert!(
        rel < 0.01,
        "{route}: marginal {marginal} vs finite difference {fd} at {x} (rel \
         {rel})"
      );
    }
  });
}

#[tokio::test]
async fn marginal_matches_finite_difference() -> Result<()> {
  let Some(state) = live_state().await else {
    return Ok(());
  };
  assert_marginal_tracks_quotes::<JITOSOL, HYUSD>(&state, "JITOSOL->HYUSD");
  assert_marginal_tracks_quotes::<HYUSD, JITOSOL>(&state, "HYUSD->JITOSOL");
  assert_marginal_tracks_quotes::<HYLOSOL, HYUSD>(&state, "HYLOSOL->HYUSD");
  assert_marginal_tracks_quotes::<HYUSD, HYLOSOL>(&state, "HYUSD->HYLOSOL");
  assert_marginal_tracks_quotes::<JITOSOL, XSOL>(&state, "JITOSOL->XSOL");
  assert_marginal_tracks_quotes::<XSOL, JITOSOL>(&state, "XSOL->JITOSOL");
  assert_marginal_tracks_quotes::<HYLOSOL, XSOL>(&state, "HYLOSOL->XSOL");
  assert_marginal_tracks_quotes::<XSOL, HYLOSOL>(&state, "XSOL->HYLOSOL");
  assert_marginal_tracks_quotes::<HYUSD, XSOL>(&state, "HYUSD->XSOL");
  assert_marginal_tracks_quotes::<XSOL, HYUSD>(&state, "XSOL->HYUSD");
  assert_marginal_tracks_quotes::<JITOSOL, HYLOSOL>(&state, "JITOSOL->HYLOSOL");
  assert_marginal_tracks_quotes::<HYLOSOL, JITOSOL>(&state, "HYLOSOL->JITOSOL");
  assert_marginal_tracks_quotes::<JITOSOL, USDC>(&state, "JITOSOL->USDC");
  assert_marginal_tracks_quotes::<HYLOSOL, USDC>(&state, "HYLOSOL->USDC");
  assert_marginal_tracks_quotes::<USDC, JITOSOL>(&state, "USDC->JITOSOL");
  assert_marginal_tracks_quotes::<USDC, HYLOSOL>(&state, "USDC->HYLOSOL");
  assert_marginal_tracks_quotes::<CBBTC, USDC>(&state, "CBBTC->USDC");
  assert_marginal_tracks_quotes::<USDC, CBBTC>(&state, "USDC->CBBTC");
  assert_marginal_tracks_quotes::<HYUSD, SHYUSD>(&state, "HYUSD->SHYUSD");
  assert_marginal_tracks_quotes::<SHYUSD, HYUSD>(&state, "SHYUSD->HYUSD");
  assert_marginal_tracks_quotes::<USDC, HYUSD>(&state, "USDC->HYUSD");
  assert_marginal_tracks_quotes::<HYUSD, USDC>(&state, "HYUSD->USDC");
  assert_marginal_tracks_quotes::<CBBTC, HYUSD>(&state, "CBBTC->HYUSD");
  assert_marginal_tracks_quotes::<HYUSD, CBBTC>(&state, "HYUSD->CBBTC");
  assert_marginal_tracks_quotes::<CBBTC, XBTC>(&state, "CBBTC->XBTC");
  assert_marginal_tracks_quotes::<XBTC, CBBTC>(&state, "XBTC->CBBTC");
  assert_marginal_tracks_quotes::<HYUSD, XBTC>(&state, "HYUSD->XBTC");
  assert_marginal_tracks_quotes::<XBTC, HYUSD>(&state, "XBTC->HYUSD");
  Ok(())
}
