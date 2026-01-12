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
use crate::token_operation::{OperationOutput, TokenOperation};

/// Deposit stablecoin (HYUSD) into stability pool for LP token (SHYUSD).
impl<C: SolanaClock> TokenOperation<HYUSD, SHYUSD> for ProtocolState<C> {
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput> {
    let amount = UFix64::<N6>::new(amount_in);
    let shyusd_nav = lp_token_nav(
      self.exchange_context.stablecoin_nav()?,
      UFix64::new(self.hyusd_pool.amount),
      self.exchange_context.levercoin_mint_nav()?,
      UFix64::new(self.xsol_pool.amount),
      UFix64::new(self.shyusd_mint.supply),
    )?;
    let shyusd_out = lp_token_out(amount, shyusd_nav)?;
    Ok(OperationOutput {
      in_amount: amount_in,
      out_amount: shyusd_out.bits,
      fee_amount: 0,
      fee_mint: HYUSD::MINT,
      fee_base: 0,
    })
  }
}

/// Withdraw LP token (SHYUSD) from stability pool for stablecoin (HYUSD).
impl<C: SolanaClock> TokenOperation<SHYUSD, HYUSD> for ProtocolState<C> {
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput> {
    ensure!(
      self.xsol_pool.amount == 0,
      "SHYUSD -> HYUSD not possible: levercoin present in pool"
    );
    let amount = UFix64::<N6>::new(amount_in);
    let shyusd_supply = UFix64::new(self.shyusd_mint.supply);
    let hyusd_in_pool = UFix64::new(self.hyusd_pool.amount);
    let hyusd_to_withdraw =
      amount_token_to_withdraw(amount, shyusd_supply, hyusd_in_pool)?;
    let withdrawal_fee = UFix64::new(self.pool_config.withdrawal_fee.bits);
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = FeeExtract::new(withdrawal_fee, hyusd_to_withdraw)?;
    Ok(OperationOutput {
      in_amount: amount_in,
      out_amount: amount_remaining.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: HYUSD::MINT,
      fee_base: hyusd_to_withdraw.bits,
    })
  }
}
