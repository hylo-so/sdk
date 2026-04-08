//! `TokenOperation` implementations for stability pool pairs.

use anyhow::{ensure, Result};
use fix::prelude::*;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_pool_math::{
  amount_token_to_withdraw, lp_token_nav, lp_token_out,
};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{
  OperationOutput, SwapOperationOutput, TokenOperation,
};

/// Deposit stablecoin (HYUSD) into stability pool for LP token (SHYUSD).
impl<C: SolanaClock> TokenOperation<HYUSD, SHYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<SwapOperationOutput> {
    let shyusd_nav = lp_token_nav(
      UFix64::new(self.hyusd_pool.amount),
      UFix64::new(self.shyusd_mint.supply),
    )?;
    let shyusd_out = lp_token_out(in_amount, shyusd_nav)?;
    Ok(OperationOutput {
      in_amount,
      out_amount: shyusd_out,
      fee_amount: UFix64::<N6>::zero(),
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
    })
  }
}

/// Withdraw LP token (SHYUSD) from stability pool for stablecoin (HYUSD).
impl<C: SolanaClock> TokenOperation<SHYUSD, HYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<SwapOperationOutput> {
    ensure!(
      self.xsol_pool.amount == 0,
      "SHYUSD -> HYUSD not possible: levercoin present in pool"
    );
    let shyusd_supply = UFix64::new(self.shyusd_mint.supply);
    let hyusd_in_pool = UFix64::new(self.hyusd_pool.amount);
    let hyusd_to_withdraw =
      amount_token_to_withdraw(in_amount, shyusd_supply, hyusd_in_pool)?;
    let withdrawal_fee: UFix64<N4> =
      self.pool_config.withdrawal_fee.try_into()?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = FeeExtract::new(withdrawal_fee, hyusd_to_withdraw)?;
    Ok(OperationOutput {
      in_amount,
      out_amount: amount_remaining,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: hyusd_to_withdraw,
    })
  }
}
