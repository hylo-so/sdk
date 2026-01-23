//! `TokenOperation` implementations for stability pool pairs.

use anyhow::{ensure, Context, Result};
use fix::prelude::*;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_pool_math::{
  amount_token_to_withdraw, lp_token_nav, lp_token_out,
  stablecoin_withdrawal_fee,
};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{
  OperationOutput, RedeemOperationOutput, SwapOperationOutput, TokenOperation,
  TokenOperationExt,
};
use crate::{Local, LST};

/// Deposit stablecoin (HYUSD) into stability pool for LP token (SHYUSD).
impl<C: SolanaClock> TokenOperation<HYUSD, SHYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_quote(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<SwapOperationOutput> {
    let shyusd_nav = lp_token_nav(
      self.exchange_context.stablecoin_nav()?,
      UFix64::new(self.hyusd_pool.amount),
      self.exchange_context.levercoin_mint_nav()?,
      UFix64::new(self.xsol_pool.amount),
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

  fn compute_quote(
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
    let withdrawal_fee = self.pool_config.withdrawal_fee.try_into()?;
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

/// Withdraw LP token from stability pool and redeem for LST.
impl<L: LST + Local, C: SolanaClock> TokenOperation<SHYUSD, L>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_quote(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<RedeemOperationOutput> {
    let lp_token_supply = UFix64::new(self.shyusd_mint.supply);
    let stablecoin_in_pool = UFix64::new(self.hyusd_pool.amount);

    // Compute pro-rata withdrawal amounts
    let stablecoin_to_withdraw =
      amount_token_to_withdraw(in_amount, lp_token_supply, stablecoin_in_pool)?;
    let levercoin_to_withdraw = amount_token_to_withdraw(
      in_amount,
      lp_token_supply,
      UFix64::new(self.xsol_pool.amount),
    )?;

    // Compute withdrawal fee from total allocation cap
    let withdrawal_fee = self.pool_config.withdrawal_fee.try_into()?;
    let stablecoin_nav = self.exchange_context.stablecoin_nav()?;
    let levercoin_nav = self.exchange_context.levercoin_mint_nav()?;
    let FeeExtract {
      amount_remaining: stablecoin_amount_remaining,
      ..
    } = stablecoin_withdrawal_fee(
      stablecoin_in_pool,
      stablecoin_to_withdraw,
      stablecoin_nav,
      levercoin_to_withdraw,
      levercoin_nav,
      withdrawal_fee,
    )?;

    // Redeem stablecoin for LST
    let (lst_from_stablecoin, fee_from_stablecoin) =
      if stablecoin_amount_remaining > UFix64::zero() {
        let op = self.quote::<HYUSD, L>(stablecoin_amount_remaining)?;
        (op.out_amount, op.fee_amount)
      } else {
        (UFix64::zero(), UFix64::zero())
      };

    // Redeem levercoin for LST
    let (lst_from_levercoin, fee_from_levercoin) =
      if levercoin_to_withdraw > UFix64::zero() {
        let op = self.quote::<XSOL, L>(levercoin_to_withdraw)?;
        (op.out_amount, op.fee_amount)
      } else {
        (UFix64::zero(), UFix64::zero())
      };

    // Sum LST outputs and redemption fees
    let out_amount = lst_from_stablecoin
      .checked_add(&lst_from_levercoin)
      .context("out_amount overflow")?;
    let fee_amount = fee_from_stablecoin
      .checked_add(&fee_from_levercoin)
      .context("fee_amount overflow")?;

    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: L::MINT,
      fee_base: out_amount
        .checked_add(&fee_amount)
        .context("fee_base overflow")?,
    })
  }
}
