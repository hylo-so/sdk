//! `SimulatedOperation` implementations for stability pool pairs.

use anyhow::{bail, Context, Result};
use fix::prelude::*;
use hylo_clients::prelude::StabilityPoolClient;
use hylo_idl::stability_pool::events::{UserDepositEvent, UserWithdrawEventV1};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::simulated_operation::SimulatedOperation;
use crate::token_operation::OperationOutput;

/// Deposit HYUSD to mint sHYUSD.
impl SimulatedOperation<HYUSD, SHYUSD> for StabilityPoolClient {
  type FeeExp = N6;
  type Event = UserDepositEvent;

  fn from_event(event: &Self::Event) -> Result<OperationOutput<N6, N6, N6>> {
    let in_amount = UFix64::new(event.stablecoin_deposited.bits);
    let out_amount = UFix64::new(event.lp_token_minted.bits);
    // No fees on deposit, fee_base = in_amount
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
    })
  }
}

/// Withdraw sHYUSD to receive HYUSD.
impl SimulatedOperation<SHYUSD, HYUSD> for StabilityPoolClient {
  type FeeExp = N6;
  type Event = UserWithdrawEventV1;

  fn from_event(event: &Self::Event) -> Result<OperationOutput<N6, N6, N6>> {
    if event.levercoin_withdrawn.bits > 0 {
      bail!("SHYUSD → HYUSD not possible: levercoin present in pool");
    }
    let in_amount = UFix64::new(event.lp_token_burned.bits);
    let out_amount = UFix64::new(event.stablecoin_withdrawn.bits);
    let fee_amount = UFix64::new(event.stablecoin_fees.bits);
    // fee_base = pre-fee withdrawal = stablecoin_withdrawn + stablecoin_fees
    let fee_base = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: HYUSD::MINT,
      fee_base,
    })
  }
}
