//! `SimulatedOperation` implementations for earn pool pairs.

use anyhow::{Context, Result};
use fix::prelude::*;
use hylo_clients::router_client::RouterClient;
use hylo_idl::earn_pool::events::{UserDepositEvent, UserWithdrawEvent};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::simulated_operation::SimulatedOperation;
use crate::token_operation::{linear_rate, SwapOperationOutput};

/// Deposit stablecoin.
impl SimulatedOperation<HYUSD, SHYUSD> for RouterClient {
  type FeeExp = N6;
  type Event = UserDepositEvent;

  fn extract_output(event: &UserDepositEvent) -> Result<SwapOperationOutput> {
    let in_amount: UFix64<N6> = event.stablecoin_deposited.try_into()?;
    let out_amount: UFix64<N6> = event.lp_token_minted.try_into()?;
    Ok(SwapOperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

/// Withdraw stablecoin.
impl SimulatedOperation<SHYUSD, HYUSD> for RouterClient {
  type FeeExp = N6;
  type Event = UserWithdrawEvent;

  fn extract_output(event: &UserWithdrawEvent) -> Result<SwapOperationOutput> {
    let in_amount: UFix64<N6> = event.lp_token_burned.try_into()?;
    let out_amount: UFix64<N6> = event.stablecoin_withdrawn.try_into()?;
    let fee_amount: UFix64<N6> = event.stablecoin_fees.try_into()?;
    let fee_base = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(SwapOperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: HYUSD::MINT,
      fee_base,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}
