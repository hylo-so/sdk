//! `TokenOperation` implementations for earn pool pairs.

use fix::prelude::*;
use hylo_core::calculus::positive_rate;
use hylo_core::earn_pool_math::{
  amount_token_to_withdraw, lp_token_nav, lp_token_out,
  max_lp_token_for_withdrawal, max_token_for_lp_deposit,
};
use hylo_core::error::CoreError;
use hylo_core::fees::controller::FeeExtract;
use hylo_core::limiter::deposit::DepositLimiter;
use hylo_core::limiter::withdraw::WithdrawalLimiter;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{
  gate, past_zero, OperationOutput, SwapOperationOutput, TokenOperation,
};

impl<C: SolanaClock> TokenOperation<HYUSD, SHYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn preconditions(&self) -> Result<(), CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.pool_config.paused, CoreError::PairPaused)?;
    gate(
      self.hyusd_pool.amount > 0 || self.shyusd_mint.supply == 0,
      CoreError::OperationDisabled,
    )
  }

  fn compute_output_ungated(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<SwapOperationOutput, CoreError> {
    let shyusd_nav = lp_token_nav(
      UFix64::new(self.hyusd_pool.amount),
      UFix64::new(self.shyusd_mint.supply),
    )?;
    let deposit_limiter: DepositLimiter =
      self.pool_config.deposit_limiter.into();
    deposit_limiter
      .validate_deposit(UFix64::new(self.hyusd_pool.amount), in_amount)?;
    let shyusd_out = lp_token_out(in_amount, shyusd_nav)?;

    // shyusd_out(x) = x / shyusd_nav
    let marginal_rate = positive_rate(1.0 / shyusd_nav.to_f64())?;
    Ok(OperationOutput {
      in_amount,
      out_amount: shyusd_out,
      fee_amount: UFix64::<N6>::zero(),
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
      marginal_rate,
    })
  }

  fn max_input_ungated(&self) -> Result<UFix64<N6>, CoreError> {
    let deposit_limiter: DepositLimiter =
      self.pool_config.deposit_limiter.into();
    deposit_limiter.max_deposit(UFix64::new(self.hyusd_pool.amount))
  }

  fn min_input_ungated(&self) -> Result<UFix64<N6>, CoreError> {
    let shyusd_nav = lp_token_nav(
      UFix64::new(self.hyusd_pool.amount),
      UFix64::new(self.shyusd_mint.supply),
    )?;
    past_zero(max_token_for_lp_deposit(UFix64::zero(), shyusd_nav)?)
  }
}

impl<C: SolanaClock> TokenOperation<SHYUSD, HYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn preconditions(&self) -> Result<(), CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.pool_config.paused, CoreError::PairPaused)?;
    gate(
      self.hyusd_pool.amount > 0,
      CoreError::InsufficientEarnPoolLiquidity,
    )
  }

  fn compute_output_ungated(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<SwapOperationOutput, CoreError> {
    let shyusd_supply = UFix64::new(self.shyusd_mint.supply);
    let hyusd_in_pool = UFix64::new(self.hyusd_pool.amount);
    gate(
      in_amount <= shyusd_supply,
      CoreError::InsufficientEarnPoolLiquidity,
    )?;
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

    // hyusd_out(x) = x * hyusd_in_pool / shyusd_supply * (1 - fee)
    let marginal_rate = positive_rate(
      hyusd_in_pool.to_f64() / shyusd_supply.to_f64()
        * (1.0 - withdrawal_fee.to_f64()),
    )?;
    Ok(OperationOutput {
      in_amount,
      out_amount: amount_remaining,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: hyusd_to_withdraw,
      marginal_rate,
    })
  }

  fn max_input_ungated(&self) -> Result<UFix64<N6>, CoreError> {
    let shyusd_supply = UFix64::new(self.shyusd_mint.supply);
    let hyusd_in_pool = UFix64::new(self.hyusd_pool.amount);
    let withdrawal_limiter: WithdrawalLimiter =
      self.pool_config.withdrawal_limiter.into();
    let headroom =
      withdrawal_limiter.max_withdrawal(self.exchange_context.clock.epoch())?;
    let limiter_cap =
      max_lp_token_for_withdrawal(headroom, shyusd_supply, hyusd_in_pool)?;
    Ok(limiter_cap.min(shyusd_supply))
  }

  fn min_input_ungated(&self) -> Result<UFix64<N6>, CoreError> {
    let withdrawal_fee: UFix64<N4> =
      self.pool_config.withdrawal_fee.try_into()?;
    let max_zero_withdrawal =
      FeeExtract::max_input(withdrawal_fee, UFix64::zero())?;
    let max_zero_lp = max_lp_token_for_withdrawal(
      max_zero_withdrawal,
      UFix64::new(self.shyusd_mint.supply),
      UFix64::new(self.hyusd_pool.amount),
    )?;
    past_zero(max_zero_lp)
  }
}
