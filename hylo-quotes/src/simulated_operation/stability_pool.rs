//! `SimulatedOperation` implementations for stability pool pairs.

use anyhow::{bail, Context, Result};
use fix::prelude::*;
use hylo_clients::prelude::StabilityPoolClient;
use hylo_idl::stability_pool::events::{UserDepositEvent, UserWithdrawEventV1};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::simulated_operation::SimulatedOperation;
use crate::token_operation::SwapOperationOutput;

/// Deposit stablecoin.
impl SimulatedOperation<HYUSD, SHYUSD> for StabilityPoolClient {
  type FeeExp = N6;
  type Event = UserDepositEvent;

  fn quote_from_event(event: &Self::Event) -> Result<SwapOperationOutput> {
    let in_amount: UFix64<N6> = UFix64::new(event.stablecoin_deposited.bits);
    let out_amount: UFix64<N6> = UFix64::new(event.lp_token_minted.bits);
    Ok(SwapOperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
    })
  }
}

/// Withdraw stablecoin.
impl SimulatedOperation<SHYUSD, HYUSD> for StabilityPoolClient {
  type FeeExp = N6;
  type Event = UserWithdrawEventV1;

  fn quote_from_event(event: &Self::Event) -> Result<SwapOperationOutput> {
    if event.levercoin_withdrawn.bits > 0 {
      bail!("SHYUSD → HYUSD not possible: levercoin present in pool");
    }
    let in_amount: UFix64<N6> = UFix64::new(event.lp_token_burned.bits);
    let out_amount: UFix64<N6> = UFix64::new(event.stablecoin_withdrawn.bits);
    let fee_amount: UFix64<N6> = UFix64::new(event.stablecoin_fees.bits);
    let fee_base: UFix64<N6> = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(SwapOperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: HYUSD::MINT,
      fee_base,
    })
  }
}
