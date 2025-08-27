use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Result};
use fix::num_traits::Zero;
use fix::prelude::*;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::idl::exchange::accounts::LstHeader;
use hylo_core::idl::pda::HYUSD;
use hylo_core::idl::stability_pool::accounts::PoolConfig;
use hylo_core::lst_sol_price::LstSolPrice;
use hylo_core::stability_pool_math::{
  amount_token_to_withdraw, lp_token_out, stablecoin_withdrawal_fee,
};
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
    fee_mint: HYUSD,
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
    fee_mint: HYUSD,
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
    fee_mint: HYUSD,
    fee_pct: Decimal::ZERO,
  })
}

/// Generates redeem quote for sHYUSD to hyUSD.
///
/// # Errors
/// - Blocked if xSOL present in pool
/// - Pro-rata withdrawal calculation
/// - Fee extraction
/// - Fee percentage calculation
pub fn shyusd_redeem(
  shyusd_mint: &Mint,
  hyusd_pool: &TokenAccount,
  xsol_pool: &TokenAccount,
  pool_config: &PoolConfig,
  shyusd_in: UFix64<N6>,
) -> Result<Quote> {
  if xsol_pool.amount.is_zero() {
    let stablecoin_in_pool = UFix64::new(hyusd_pool.amount);
    let stablecoin_to_withdraw = amount_token_to_withdraw(
      shyusd_in,
      UFix64::new(shyusd_mint.supply),
      stablecoin_in_pool,
    )?;
    let withdrawal_fee = UFix64::new(pool_config.withdrawal_fee.bits);
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = FeeExtract::new(withdrawal_fee, stablecoin_to_withdraw)?;
    Ok(Quote {
      in_amount: shyusd_in.bits,
      out_amount: amount_remaining.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: HYUSD,
      fee_pct: fee_pct_decimal(fees_extracted, stablecoin_to_withdraw)?,
    })
  } else {
    Err(anyhow!(
      "sHYUSD/hyUSD not possible due to xSOL in stability pool."
    ))
  }
}

/// Generates liquidation redeem quote for sHYUSD to an LST via hyUSD and xSOL.
///
/// # Errors
/// - Pro-rata withdrawal calculation
/// - Fee extraction across multiple tokens
/// - Token conversions
/// - Arithmetic overflow
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
