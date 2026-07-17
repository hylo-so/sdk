//! `TokenOperation` implementations for earn pool pairs.

use fix::prelude::*;
use hylo_core::earn_pool_math::{
  amount_token_to_withdraw, lp_token_nav, lp_token_out,
};
use hylo_core::error::CoreError;
use hylo_core::fees::controller::FeeExtract;
use hylo_core::limiter::deposit::DepositLimiter;
use hylo_core::limiter::withdraw::WithdrawalLimiter;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{
  gate, linear_rate, OperationOutput, SwapOperationOutput, TokenOperation,
};

impl<C: SolanaClock> TokenOperation<HYUSD, SHYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<SwapOperationOutput, CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.pool_config.paused, CoreError::PairPaused)?;
    gate(
      self.hyusd_pool.amount > 0 || self.shyusd_mint.supply == 0,
      CoreError::OperationDisabled,
    )?;
    let shyusd_nav = lp_token_nav(
      UFix64::new(self.hyusd_pool.amount),
      UFix64::new(self.shyusd_mint.supply),
    )?;
    let deposit_limiter: DepositLimiter =
      self.pool_config.deposit_limiter.into();
    deposit_limiter
      .validate_deposit(UFix64::new(self.hyusd_pool.amount), in_amount)?;
    let shyusd_out = lp_token_out(in_amount, shyusd_nav)?;
    gate(shyusd_out > UFix64::zero(), CoreError::ZeroAmount)?;
    Ok(OperationOutput {
      in_amount,
      out_amount: shyusd_out,
      fee_amount: UFix64::<N6>::zero(),
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
      marginal_rate: linear_rate(in_amount, shyusd_out)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<SHYUSD, HYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<SwapOperationOutput, CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.pool_config.paused, CoreError::PairPaused)?;
    let shyusd_supply = UFix64::new(self.shyusd_mint.supply);
    let hyusd_in_pool = UFix64::new(self.hyusd_pool.amount);
    let hyusd_to_withdraw =
      amount_token_to_withdraw(in_amount, shyusd_supply, hyusd_in_pool)?;
    let withdrawal_limiter: WithdrawalLimiter =
      self.pool_config.withdrawal_limiter.into();
    withdrawal_limiter.validate_withdrawal(
      hyusd_to_withdraw,
      self.exchange_context.clock.epoch(),
    )?;
    let withdrawal_fee: UFix64<N4> =
      self.pool_config.withdrawal_fee.try_into()?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = FeeExtract::new(withdrawal_fee, hyusd_to_withdraw)?;
    gate(amount_remaining > UFix64::zero(), CoreError::ZeroAmount)?;
    Ok(OperationOutput {
      in_amount,
      out_amount: amount_remaining,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: hyusd_to_withdraw,
      marginal_rate: linear_rate(in_amount, amount_remaining)?,
    })
  }
}
