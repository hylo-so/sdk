use anchor_spl::token::{Mint, TokenAccount};
use anyhow::Result;
use fix::prelude::*;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::idl::exchange::accounts::LstHeader;
use hylo_core::idl::pda;
use hylo_core::stability_pool_math::lp_token_out;
use hylo_core::{
  exchange_context::ExchangeContext, stability_pool_math::lp_token_nav,
};
use jupiter_amm_interface::{ClockRef, Quote};
use rust_decimal::Decimal;

use crate::util::fee_pct_decimal;

/// Generates mint quote for HYUSD from LST.
///
/// # Errors
/// - Fee extraction
/// - Stablecoin NAV calculation
/// - Token conversion
/// - Stablecoin amount validation
/// - Fee percentage calculation
pub fn hyusd_mint(
  ctx: &ExchangeContext<ClockRef>,
  lst_header: &LstHeader,
  in_amount: UFix64<N9>,
) -> Result<Quote> {
  let lst_price = lst_header.price_sol.into();
  let FeeExtract {
    fees_extracted,
    amount_remaining,
  } = ctx.stablecoin_mint_fee(&lst_price, in_amount)?;
  let stablecoin_nav = ctx.stablecoin_nav()?;
  let hyusd_out = {
    let converted = ctx
      .token_conversion(&lst_price)?
      .lst_to_token(amount_remaining, stablecoin_nav)?;
    ctx.validate_stablecoin_amount(converted)
  }?;
  Ok(Quote {
    in_amount: in_amount.bits,
    out_amount: hyusd_out.bits,
    fee_amount: fees_extracted.bits,
    fee_mint: lst_header.mint,
    fee_pct: fee_pct_decimal(fees_extracted, in_amount)?,
  })
}

/// Generates redeem quote for HYUSD to LST.
///
/// # Errors
/// - Stablecoin NAV calculation
/// - Token conversion
/// - Fee extraction
/// - Fee percentage calculation
pub fn hyusd_redeem(
  ctx: &ExchangeContext<ClockRef>,
  lst_header: &LstHeader,
  in_amount: UFix64<N6>,
) -> Result<Quote> {
  let lst_price = lst_header.price_sol.into();
  let stablecoin_nav = ctx.stablecoin_nav()?;
  let lst_out = ctx
    .token_conversion(&lst_price)?
    .token_to_lst(in_amount, stablecoin_nav)?;
  let FeeExtract {
    fees_extracted,
    amount_remaining,
  } = ctx.stablecoin_redeem_fee(&lst_price, lst_out)?;
  Ok(Quote {
    in_amount: in_amount.bits,
    out_amount: amount_remaining.bits,
    fee_amount: fees_extracted.bits,
    fee_mint: lst_header.mint,
    fee_pct: fee_pct_decimal(fees_extracted, lst_out)?,
  })
}

/// Generates mint quote for XSOL from LST.
///
/// # Errors
/// - Fee extraction
/// - Levercoin mint NAV calculation
/// - Token conversion
/// - Fee percentage calculation
pub fn xsol_mint(
  ctx: &ExchangeContext<ClockRef>,
  lst_header: &LstHeader,
  in_amount: UFix64<N9>,
) -> Result<Quote> {
  let lst_price = lst_header.price_sol.into();
  let FeeExtract {
    fees_extracted,
    amount_remaining,
  } = ctx.levercoin_mint_fee(&lst_price, in_amount)?;
  let levercoin_mint_nav = ctx.levercoin_mint_nav()?;
  let xsol_out = ctx
    .token_conversion(&lst_price)?
    .lst_to_token(amount_remaining, levercoin_mint_nav)?;
  Ok(Quote {
    in_amount: in_amount.bits,
    out_amount: xsol_out.bits,
    fee_amount: fees_extracted.bits,
    fee_mint: lst_header.mint,
    fee_pct: fee_pct_decimal(fees_extracted, in_amount)?,
  })
}

/// Generates redeem quote for XSOL to LST.
///
/// # Errors
/// - Levercoin redeem NAV calculation
/// - Token conversion
/// - Fee extraction
/// - Fee percentage calculation
pub fn xsol_redeem(
  ctx: &ExchangeContext<ClockRef>,
  lst_header: &LstHeader,
  in_amount: UFix64<N6>,
) -> Result<Quote> {
  let lst_price = lst_header.price_sol.into();
  let xsol_nav = ctx.levercoin_redeem_nav()?;
  let lst_out = ctx
    .token_conversion(&lst_price)?
    .token_to_lst(in_amount, xsol_nav)?;
  let FeeExtract {
    fees_extracted,
    amount_remaining,
  } = ctx.levercoin_redeem_fee(&lst_price, lst_out)?;
  Ok(Quote {
    in_amount: in_amount.bits,
    out_amount: amount_remaining.bits,
    fee_amount: fees_extracted.bits,
    fee_mint: lst_header.mint,
    fee_pct: fee_pct_decimal(fees_extracted, lst_out)?,
  })
}

/// Generates swap quote for HYUSD/XSOL.
///
/// # Errors
/// - Fee extraction
/// - Swap conversion
/// - Fee percentage calculation
pub fn hyusd_xsol_swap(
  ctx: &ExchangeContext<ClockRef>,
  in_amount: UFix64<N6>,
) -> Result<Quote> {
  let FeeExtract {
    fees_extracted,
    amount_remaining,
  } = ctx.stablecoin_to_levercoin_fee(in_amount)?;
  let xsol_out = ctx.swap_conversion()?.stable_to_lever(amount_remaining)?;
  Ok(Quote {
    in_amount: in_amount.bits,
    out_amount: xsol_out.bits,
    fee_amount: fees_extracted.bits,
    fee_mint: *pda::HYUSD,
    fee_pct: fee_pct_decimal(fees_extracted, in_amount)?,
  })
}

/// Generates swap quote for XSOL/HYUSD.
///
/// # Errors
/// - Swap conversion
/// - Stablecoin swap amount validation
/// - Fee extraction
/// - Fee percentage calculation
pub fn xsol_hyusd_swap(
  ctx: &ExchangeContext<ClockRef>,
  in_amount: UFix64<N6>,
) -> Result<Quote> {
  let hyusd_total = {
    let converted = ctx.swap_conversion()?.lever_to_stable(in_amount)?;
    ctx.validate_stablecoin_swap_amount(converted)
  }?;
  let FeeExtract {
    fees_extracted,
    amount_remaining,
  } = ctx.levercoin_to_stablecoin_fee(hyusd_total)?;
  Ok(Quote {
    in_amount: in_amount.bits,
    out_amount: amount_remaining.bits,
    fee_amount: fees_extracted.bits,
    fee_mint: *pda::HYUSD,
    fee_pct: fee_pct_decimal(fees_extracted, hyusd_total)?,
  })
}

pub fn shyusd_mint(
  ctx: &ExchangeContext<ClockRef>,
  shyusd_mint: &Mint,
  hyusd_pool: &TokenAccount,
  xsol_pool: &TokenAccount,
  hyusd_in: UFix64<N6>,
) -> Result<Quote> {
  let shyusd_nav = lp_token_nav(
    ctx.stablecoin_nav()?,
    UFix64::new(hyusd_pool.amount),
    ctx.levercoin_mint_nav()?,
    UFix64::new(xsol_pool.amount),
    UFix64::new(shyusd_mint.supply),
  )?;
  let shyusd_out = lp_token_out(hyusd_in, shyusd_nav)?;
  Ok(Quote {
    in_amount: hyusd_in.bits,
    out_amount: shyusd_out.bits,
    fee_amount: u64::MIN,
    fee_mint: *pda::HYUSD,
    fee_pct: Decimal::ZERO,
  })
}

pub fn shyusd_redeem(
  ctx: &ExchangeContext<ClockRef>,
  shyusd_mint: &Mint,
  hyusd_pool: &TokenAccount,
  xsol_pool: &TokenAccount,
  shyusd_in: UFix64<N6>,
) -> Result<Quote> {
  let shyusd_nav = lp_token_nav(
    ctx.stablecoin_nav()?,
    UFix64::new(hyusd_pool.amount),
    ctx.levercoin_mint_nav()?,
    UFix64::new(xsol_pool.amount),
    UFix64::new(shyusd_mint.supply),
  )?;
  todo!("")
}
