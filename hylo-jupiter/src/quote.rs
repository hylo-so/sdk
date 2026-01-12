//! Quote generation for Jupiter AMM interface.

use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Result};
use fix::prelude::*;
use hylo_clients::protocol_state::ProtocolState;
use hylo_clients::syntax_helpers::TokenOperationExt;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::idl::exchange::accounts::LstHeader;
use hylo_core::idl::stability_pool::accounts::PoolConfig;
use hylo_core::idl::tokens::{HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_core::lst_sol_price::LstSolPrice;
use hylo_core::stability_pool_math::{
  amount_token_to_withdraw, stablecoin_withdrawal_fee,
};
use hylo_jupiter_amm_interface::{ClockRef, Quote};

use crate::util::{fee_pct_decimal, operation_to_quote};

/// Generates mint quote for HYUSD from `JitoSOL`.
///
/// # Errors
/// `TokenOperation` errors.
pub fn hyusd_mint(
  state: &ProtocolState<ClockRef>,
  in_amount: u64,
) -> Result<Quote> {
  let op = state.compute_quote::<JITOSOL, HYUSD>(in_amount)?;
  Ok(operation_to_quote(op))
}

/// Generates redeem quote for HYUSD to `JitoSOL`.
///
/// # Errors
/// `TokenOperation` errors.
pub fn hyusd_redeem(
  state: &ProtocolState<ClockRef>,
  in_amount: u64,
) -> Result<Quote> {
  let op = state.compute_quote::<HYUSD, JITOSOL>(in_amount)?;
  Ok(operation_to_quote(op))
}

/// Generates mint quote for XSOL from `JitoSOL`.
///
/// # Errors
/// `TokenOperation` errors.
pub fn xsol_mint(
  state: &ProtocolState<ClockRef>,
  in_amount: u64,
) -> Result<Quote> {
  let op = state.compute_quote::<JITOSOL, XSOL>(in_amount)?;
  Ok(operation_to_quote(op))
}

/// Generates redeem quote for XSOL to `JitoSOL`.
///
/// # Errors
/// `TokenOperation` errors.
pub fn xsol_redeem(
  state: &ProtocolState<ClockRef>,
  in_amount: u64,
) -> Result<Quote> {
  let op = state.compute_quote::<XSOL, JITOSOL>(in_amount)?;
  Ok(operation_to_quote(op))
}

/// Generates swap quote for HYUSD to XSOL.
///
/// # Errors
/// `TokenOperation` errors.
pub fn hyusd_xsol_swap(
  state: &ProtocolState<ClockRef>,
  in_amount: u64,
) -> Result<Quote> {
  let op = state.compute_quote::<HYUSD, XSOL>(in_amount)?;
  Ok(operation_to_quote(op))
}

/// Generates swap quote for XSOL to HYUSD.
///
/// # Errors
/// `TokenOperation` errors.
pub fn xsol_hyusd_swap(
  state: &ProtocolState<ClockRef>,
  in_amount: u64,
) -> Result<Quote> {
  let op = state.compute_quote::<XSOL, HYUSD>(in_amount)?;
  Ok(operation_to_quote(op))
}

/// Generates deposit quote for HYUSD to SHYUSD.
///
/// # Errors
/// * `TokenOperation` errors
pub fn shyusd_mint(
  state: &ProtocolState<ClockRef>,
  in_amount: u64,
) -> Result<Quote> {
  let op = state.compute_quote::<HYUSD, SHYUSD>(in_amount)?;
  Ok(operation_to_quote(op))
}

/// Generates withdrawal quote for SHYUSD to HYUSD.
///
/// # Errors
/// * `TokenOperation` errors
pub fn shyusd_redeem(
  state: &ProtocolState<ClockRef>,
  in_amount: u64,
) -> Result<Quote> {
  let op = state.compute_quote::<SHYUSD, HYUSD>(in_amount)?;
  Ok(operation_to_quote(op))
}

/// Liquidation redeem quote for sHYUSD to an LST via hyUSD and xSOL.
///
/// # Errors
/// * Pro-rata withdrawal calculation
/// * Fee extraction across multiple tokens
/// * Token conversions
/// * Arithmetic overflow
pub fn shyusd_redeem_lst(
  ctx: &ExchangeContext<ClockRef>,
  shyusd_mint: &Mint,
  hyusd_pool: &TokenAccount,
  xsol_pool: &TokenAccount,
  pool_config: &PoolConfig,
  lst_header: &LstHeader,
  shyusd_in: UFix64<N6>,
) -> Result<Quote> {
  // Get pro rata share of hyUSD and xSOL
  let shyusd_supply = UFix64::new(shyusd_mint.supply);
  let hyusd_in_pool = UFix64::new(hyusd_pool.amount);
  let hyusd_to_withdraw =
    amount_token_to_withdraw(shyusd_in, shyusd_supply, hyusd_in_pool)?;
  let xsol_in_pool = UFix64::new(xsol_pool.amount);
  let xsol_to_withdraw =
    amount_token_to_withdraw(shyusd_in, shyusd_supply, xsol_in_pool)?;

  // Withdrawal fees as LST
  let withdrawal_fee = UFix64::new(pool_config.withdrawal_fee.bits);
  let hyusd_nav = ctx.stablecoin_nav()?;
  let xsol_mint_nav = ctx.levercoin_mint_nav()?;
  let FeeExtract {
    fees_extracted: withdrawal_fee_hyusd,
    amount_remaining: hyusd_remaining,
  } = stablecoin_withdrawal_fee(
    hyusd_in_pool,
    hyusd_to_withdraw,
    hyusd_nav,
    xsol_to_withdraw,
    xsol_mint_nav,
    withdrawal_fee,
  )?;
  let lst_sol_price: LstSolPrice = lst_header.price_sol.into();
  let conversion = ctx.token_conversion(&lst_sol_price)?;
  let withdrawal_fee_lst =
    conversion.token_to_lst(withdrawal_fee_hyusd, hyusd_nav)?;

  // Convert remaining hyUSD to LST, take fees in LST
  let hyusd_redeem_lst = conversion.token_to_lst(hyusd_remaining, hyusd_nav)?;
  let FeeExtract {
    fees_extracted: hyusd_redeem_fee_lst,
    amount_remaining: hyusd_remaining_lst,
  } = ctx.stablecoin_redeem_fee(&lst_sol_price, hyusd_redeem_lst)?;

  // Convert xSOL to given LST, take fees in LST
  let xsol_redeem_nav = ctx.levercoin_redeem_nav()?;
  let xsol_redeem_lst =
    conversion.token_to_lst(xsol_to_withdraw, xsol_redeem_nav)?;
  let FeeExtract {
    fees_extracted: xsol_redeem_fee_lst,
    amount_remaining: xsol_remaining_lst,
  } = ctx.levercoin_redeem_fee(&lst_sol_price, xsol_redeem_lst)?;

  // Compute totals
  let total_fees_lst = withdrawal_fee_lst
    .checked_add(&hyusd_redeem_fee_lst)
    .and_then(|sub| sub.checked_add(&xsol_redeem_fee_lst))
    .ok_or(anyhow!("Fee overflow: withdrawal + hyUSD + xSOL"))?;
  let total_out_lst = hyusd_remaining_lst
    .checked_add(&xsol_remaining_lst)
    .ok_or(anyhow!("Output overflow: hyUSD + xSOL"))?;

  Ok(Quote {
    in_amount: shyusd_in.bits,
    out_amount: total_out_lst.bits,
    fee_amount: total_fees_lst.bits,
    fee_mint: lst_header.mint,
    fee_pct: fee_pct_decimal(total_fees_lst, total_out_lst)?,
  })
}
