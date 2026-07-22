//! Diagnostic probe for the titan `mean_value_theorem` failure.
//!
//! Requires `RPC_URL`. Run with `--ignored --nocapture`.

use std::sync::Arc;

use anchor_lang::prelude::Clock;
use anyhow::Result;
use fix::prelude::*;
use hylo_idl::tokens::{HYUSD, JITOSOL};
use hylo_quotes::prelude::{ProtocolState, RpcStateProvider, StateProvider};
use hylo_quotes::token_operation::{TokenOperation, TokenOperationExt};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

const REL_TOL: f64 = 1e-5;
const OUT_QUANTUM: f64 = 2.0;

async fn live_state() -> Option<ProtocolState<Clock>> {
  let url = std::env::var("RPC_URL").ok()?;
  let provider = RpcStateProvider::new(Arc::new(RpcClient::new(url)));
  provider.fetch_state().await.ok()
}

fn geometric_grid(lb: u64, ub: u64, n: usize) -> Vec<u64> {
  #[allow(clippy::cast_precision_loss)]
  let ratio = (ub as f64 / lb as f64).powf(1.0 / (n as f64 - 1.0));
  #[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
  )]
  (0..n)
    .map(|i| ((lb as f64) * ratio.powi(i32::try_from(i).unwrap_or(0))) as u64)
    .collect()
}

#[tokio::test]
#[ignore = "diagnostic; requires RPC_URL"]
async fn mvt_probe_hyusd_jitosol() -> Result<()> {
  let Some(state) = live_state().await else {
    return Ok(());
  };
  let max = TokenOperation::<HYUSD, JITOSOL>::max_input(&state)?.bits;
  let quote = |x: u64| {
    state
      .output::<HYUSD, JITOSOL>(UFix64::<N6>::new(x))
      .map(|op| {
        (
          op.out_amount.bits,
          op.marginal_rate,
          op.fee_amount.bits,
          op.fee_base.bits,
        )
      })
  };
  let grid = geometric_grid(1_000_000, max, 64);
  #[allow(clippy::cast_precision_loss)]
  grid.windows(2).for_each(|w| {
    let (a, b) = (w[0], w[1]);
    if b <= a {
      return;
    }
    let Ok((out_a, price_a, fee_a, base_a)) = quote(a) else {
      return;
    };
    let Ok((out_b, price_b, fee_b, base_b)) = quote(b) else {
      return;
    };
    if out_b <= out_a {
      return;
    }
    let chord = (out_b - out_a) as f64 / (b - a) as f64;
    let atol = OUT_QUANTUM / (b - a) as f64;
    let upper_ok = chord <= price_a * (1.0 + REL_TOL) + atol;
    let lower_ok = chord >= price_b * (1.0 - REL_TOL) - atol;
    if !(upper_ok && lower_ok) {
      let fee_rate_a = fee_a as f64 / base_a as f64;
      let fee_rate_b = fee_b as f64 / base_b as f64;
      println!(
        "VIOLATION [{a}, {b}]: chord={chord:.9} price_a={price_a:.9} \
         price_b={price_b:.9} upper_ok={upper_ok} lower_ok={lower_ok}\n  \
         fee_rate_a={fee_rate_a:.9} fee_rate_b={fee_rate_b:.9} out_a={out_a} \
         out_b={out_b}"
      );
      let d = ((b - a) / 100).max(1_000);
      [a, b].iter().for_each(|&x| {
        if let (Ok((lo, ..)), Ok((hi, ..))) = (quote(x - d), quote(x + d)) {
          let fd = (hi - lo) as f64 / (2 * d) as f64;
          println!("  central_diff({x}, d={d}) = {fd:.9}");
        }
      });
    }
  });
  Ok(())
}
