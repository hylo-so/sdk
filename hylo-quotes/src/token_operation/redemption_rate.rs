//! State-derived sHYUSD redemption rate.
//!
//! A RATE for an oracle-feed consumer, never an executable quote. It
//! multiplies the sHYUSD -> hyUSD earn-pool exit rate (NAV net of the
//! withdrawal fee) by the best USD value among the hyUSD redemption
//! lanes, each valued at a reference amount. The LST and exo lanes are
//! valued at the lower oracle bound; the USDC lane is valued at the
//! USDC/USD spot capped at par (this SDK version has no USDC price
//! range).
//!
//! An LST/exo lane converts hyUSD to collateral at the UPPER oracle
//! bound while its valuation uses the LOWER bound, so the lane's hyUSD
//! rate is about `nav * lower / upper * (1 - fee)`: the oracle level
//! cancels out, and the rate cannot exceed par.
//!
//! The valuation is separate from the executable quote math in
//! [`TokenOperation`]. It shares the conversion and fee primitives, but
//! it skips route gates and prices the redeem fee at
//! `min(projected CR, curve x_max)`. A lane's `open` flag is the strict
//! quote at the reference amount.
//!
//! # Availability
//!
//! A route gate (pause, overdue harvest, oracle window) or a collateral
//! ratio above the redeem fee-curve domain never removes a lane by
//! itself: the reading survives both. A CR above the domain instead
//! prices the lane at the domain edge and reports
//! [`FeeBasis::RedeemMaxCr`].
//!
//! A lane is absent whenever it cannot absorb the reference amount, or
//! the lane's USD valuation cannot be formed. Known causes (not
//! exhaustive): the vault cannot cover the reference amount; the
//! pair's virtual stablecoin supply or burn floor cannot cover it; the
//! LST epoch price is missing; arithmetic overflow. The LST epoch price
//! is a regular, expected absence: after an epoch rollover it stays
//! missing until the LST price crank runs, so both LST lanes drop for
//! that window and `best` moves to another lane.

use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, ensure, Result};
use fix::prelude::*;
use hylo_core::collateral_ratio::CollateralRatio;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fees::controller::FeeExtract;
use hylo_core::fees::curve_controller::{
  InterpolatedFeeController, InterpolatedRedeemFees,
};
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::virtual_stablecoin::{validate_burn, SUPPLY_FLOOR};
use hylo_idl::tokens::{
  Exo, TokenMint, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, USDC,
};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{gate, TokenOperation};
use crate::{Local, LST};

/// Which collateral ratio priced a stablecoin redeem fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FeeBasis {
  /// Fee at the projected post-trade CR: what the strict math uses.
  /// Flat-fee lanes (USDC) have no CR to project, and always report
  /// this variant.
  CurrentCr,
  /// Projected CR is above the redeem fee-curve domain. Fee at the
  /// domain edge. The route cannot execute in this state.
  RedeemMaxCr,
}

/// Stablecoin redeem fee at `min(projected CR, curve x_max)`.
///
/// Compares in curve coordinates so the basis agrees with the strict
/// lookup, which truncates CR to `N5`.
///
/// # Errors
/// * Curve interpolation, fee conversion, or fee extraction
fn clamped_redeem_fee(
  fees: &InterpolatedRedeemFees,
  projected_cr: CollateralRatio,
  amount_out: UFix64<N9>,
) -> Result<(FeeExtract<N9>, FeeBasis), CoreError> {
  let x = projected_cr.fee_curve_x();
  let x_max = fees.curve().x_max();
  let basis = if x > x_max {
    FeeBasis::RedeemMaxCr
  } else {
    FeeBasis::CurrentCr
  };
  let fee_rate: UFix64<N5> = fees
    .fee_inner(x.min(x_max))?
    .narrow()
    .ok_or(CoreError::InterpFeeConversion)?;
  FeeExtract::new(fee_rate, amount_out).map(|extract| (extract, basis))
}

/// One hyUSD redemption lane priced at the reference amount.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RedemptionLane {
  /// Collateral mint the lane redeems into.
  pub mint: Pubkey,
  /// Route gates pass and the fee is unclamped: executable at
  /// `reference_hyusd` in the current state.
  pub open: bool,
  /// Which collateral ratio priced the redeem fee.
  pub fee_basis: FeeBasis,
  /// Net output in the lane token's own decimals.
  pub amount_out: UFixValue64,
  /// `amount_out` valued in USD: at the lower oracle bound for the
  /// LST and exo lanes, at the USDC/USD spot capped at par for USDC.
  pub usd_out: UFix64<N9>,
  /// USD per hyUSD through this lane.
  pub hyusd_usd_rate: UFix64<N9>,
  /// USD per sHYUSD through this lane.
  pub shyusd_usd_rate: UFix64<N9>,
}

/// The sHYUSD redemption rate and the lanes behind it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RedemptionRate {
  /// hyUSD per sHYUSD: earn-pool NAV net of the withdrawal fee.
  pub shyusd_hyusd_rate: UFix64<N9>,
  /// hyUSD amount every lane was priced at.
  pub reference_hyusd: UFix64<N6>,
  /// Lanes that priced the reference.
  pub lanes: Vec<RedemptionLane>,
  /// Lane with the highest `usd_out`. May be a CLOSED lane: check
  /// `best.open`. On an exact tie the later lane in the fixed order
  /// JITOSOL, HYLOSOL, CBBTC, HYPE, USDC wins, since that is what
  /// `Iterator::max_by_key` returns.
  pub best: RedemptionLane,
}

impl<C: SolanaClock> ProtocolState<C> {
  /// hyUSD per sHYUSD: earn-pool NAV, floored at true `N9`, net of the
  /// withdrawal fee. Ignores the withdrawal limiter, which gates
  /// execution and not value.
  ///
  /// # Degenerate states
  /// Zero sHYUSD supply gives NAV `1.0` (the same convention as
  /// `lp_token_nav`), so this returns `1.0` net of the withdrawal fee,
  /// not an error. A nonzero supply against an empty earn pool gives
  /// NAV `0`, and so a rate of `0`, also not an error.
  ///
  /// # Errors
  /// * NAV overflows `N9`
  /// * Withdrawal fee conversion or fee extraction
  fn shyusd_exit_rate(&self) -> Result<UFix64<N9>> {
    let pool = UFix64::<N6>::new(self.hyusd_pool.amount);
    let supply = UFix64::<N6>::new(self.shyusd_mint.supply);
    let nav = if supply == UFix64::zero() {
      UFix64::<N9>::one()
    } else {
      UFix64::<N9>::one()
        .mul_div_floor(pool, supply)
        .ok_or_else(|| anyhow!("earn pool NAV overflows N9"))?
    };
    let withdrawal_fee: UFix64<N4> =
      self.pool_config.withdrawal_fee.try_into()?;
    Ok(FeeExtract::new(withdrawal_fee, nav)?.amount_remaining)
  }

  /// LST/USD at the lower SOL/USD bound.
  fn lst_usd_lower<L: LST + Local>(&self) -> Option<UFix64<N9>> {
    let price: LstSolPrice = self.lst_header::<L>().ok()?.price_sol.into();
    price
      .get_epoch_price(self.exchange_context.clock.epoch())
      .ok()?
      .mul_div_floor(
        self.exchange_context.collateral_usd_price().lower,
        UFix64::one(),
      )
  }

  /// LST out for `reference` hyUSD, net of the clamped redeem fee.
  ///
  /// # Errors
  /// * Conversion, vault or burn capacity, projection, or fee
  fn lst_lane_value<L: LST + Local>(
    &self,
    reference: UFix64<N6>,
  ) -> Result<(UFix64<N9>, FeeBasis), CoreError> {
    let context = &self.exchange_context;
    let lst_price: LstSolPrice = self.lst_header::<L>()?.price_sol.into();
    let lst_out = context
      .token_conversion(&lst_price)?
      .token_to_lst(reference, context.stablecoin_nav()?)?;
    gate(
      lst_out <= self.lst_vault_balance::<L>()?,
      CoreError::InsufficientLiquidity,
    )?;
    validate_burn(
      context.virtual_stablecoin_supply()?,
      reference,
      SUPPLY_FLOOR,
    )?;
    let projected = context.projected_redeem_state(&lst_price, lst_out)?;
    clamped_redeem_fee(
      &context.stablecoin_redeem_fees,
      projected.collateral_ratio,
      lst_out,
    )
    .map(|(extract, basis)| (extract.amount_remaining, basis))
  }

  /// Exo collateral out for `reference` hyUSD, net of the clamped
  /// redeem fee.
  ///
  /// # Errors
  /// * Pair lookup, conversion, collateral or burn capacity, projection, fee,
  ///   or precision
  fn exo_lane_value<E: Exo>(
    &self,
    reference: UFix64<N6>,
  ) -> Result<(UFix64<E::Exp>, FeeBasis), CoreError>
  where
    UFix64<E::Exp>: FixExt,
  {
    let pair = self.exo_pair::<E>()?;
    let context = &pair.context;
    let collateral_out = context
      .exo_conversion()
      .token_to_exo(reference, context.stablecoin_nav()?)?;
    gate(
      collateral_out <= context.total_collateral,
      CoreError::InsufficientLiquidity,
    )?;
    validate_burn(
      context.virtual_stablecoin_supply()?,
      reference,
      pair.supply_floor,
    )?;
    let projected = context.projected_redeem_state(collateral_out)?;
    let (extract, basis) = clamped_redeem_fee(
      &context.stablecoin_redeem_fees,
      projected.collateral_ratio,
      collateral_out,
    )?;
    extract
      .amount_remaining
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)
      .map(|amount_out| (amount_out, basis))
  }

  /// Exo/USD at the lower oracle bound.
  fn exo_usd_lower<E: Exo>(&self) -> Option<UFix64<N9>> {
    self
      .exo_pair::<E>()
      .ok()
      .map(|pair| pair.context.collateral_usd_price().lower)
  }

  /// Lowers one valued lane. `None` drops the lane.
  fn redemption_lane<OUT>(
    &self,
    reference: UFix64<N6>,
    shyusd_hyusd_rate: UFix64<N9>,
    valued: Result<(UFix64<OUT::Exp>, FeeBasis), CoreError>,
    usd_price_lower: Option<UFix64<N9>>,
  ) -> Option<RedemptionLane>
  where
    OUT: TokenMint,
    ProtocolState<C>: TokenOperation<HYUSD, OUT>,
    UFix64<OUT::Exp>: FixExt,
  {
    let (out_amount, fee_basis) = valued.ok()?;
    let usd_out = out_amount
      .checked_convert::<N9>()?
      .mul_div_floor(usd_price_lower?, UFix64::<N9>::one())?;
    let hyusd_usd_rate =
      usd_out.mul_div_floor(UFix64::<N6>::one(), reference)?;
    let shyusd_usd_rate =
      shyusd_hyusd_rate.mul_div_floor(hyusd_usd_rate, UFix64::<N9>::one())?;
    let open =
      TokenOperation::<HYUSD, OUT>::compute_output(self, reference).is_ok();
    Some(RedemptionLane {
      mint: OUT::MINT,
      open,
      fee_basis,
      amount_out: out_amount.into(),
      usd_out,
      hyusd_usd_rate,
      shyusd_usd_rate,
    })
  }

  /// USD value of one sHYUSD through the best hyUSD redemption lane.
  ///
  /// A rate, never an executable quote: see the module docs.
  ///
  /// # Degenerate states
  /// Zero sHYUSD supply gives NAV `1.0`, so the rate is `1.0` net of
  /// the withdrawal fee, not an error. A nonzero supply against an
  /// empty earn pool gives NAV `0`, and so a rate of `0`, also not an
  /// error. A consumer must treat both as "pool not live".
  ///
  /// # Errors
  /// * Zero `reference`
  /// * Earn-pool NAV or withdrawal fee
  /// * No lane can price `reference`
  pub fn redemption_rate(
    &self,
    reference: UFix64<N6>,
  ) -> Result<RedemptionRate> {
    ensure!(
      reference > UFix64::zero(),
      "redemption reference amount must be nonzero"
    );
    let exit = self.shyusd_exit_rate()?;
    let lanes: Vec<RedemptionLane> = [
      self.redemption_lane::<JITOSOL>(
        reference,
        exit,
        self.lst_lane_value::<JITOSOL>(reference),
        self.lst_usd_lower::<JITOSOL>(),
      ),
      self.redemption_lane::<HYLOSOL>(
        reference,
        exit,
        self.lst_lane_value::<HYLOSOL>(reference),
        self.lst_usd_lower::<HYLOSOL>(),
      ),
      self.redemption_lane::<CBBTC>(
        reference,
        exit,
        self.exo_lane_value::<CBBTC>(reference),
        self.exo_usd_lower::<CBBTC>(),
      ),
      self.redemption_lane::<HYPE>(
        reference,
        exit,
        self.exo_lane_value::<HYPE>(reference),
        self.exo_usd_lower::<HYPE>(),
      ),
      self.redemption_lane::<USDC>(
        reference,
        exit,
        // Flat fee and no CR domain: the ungated quote is the value.
        TokenOperation::<HYUSD, USDC>::compute_output_ungated(self, reference)
          .map(|op| (op.out_amount, FeeBasis::CurrentCr)),
        // The rate skips route gates, and the USDC par-tolerance check
        // is one of those gates: an above-par USDC spot would
        // otherwise lift this lane above 1 USD and make it `best`.
        // This SDK version has no USDC price range, so cap at par.
        Some(
          self
            .usdc_exchange_state
            .usdc_usd_spot
            .min(UFix64::<N9>::one()),
        ),
      ),
    ]
    .into_iter()
    .flatten()
    .collect();
    let best = lanes
      .iter()
      .max_by_key(|lane| lane.usd_out.bits)
      .cloned()
      .ok_or_else(|| {
        anyhow!("no redemption lane can price the reference amount")
      })?;
    Ok(RedemptionRate {
      shyusd_hyusd_rate: exit,
      reference_hyusd: reference,
      lanes,
      best,
    })
  }
}

#[cfg(test)]
mod tests {
  use anyhow::Result;
  use fix::prelude::*;
  use hylo_core::collateral_ratio::CR;
  use hylo_core::fees::curve_controller::{
    InterpolatedFeeController, InterpolatedRedeemFees,
  };
  use hylo_core::fees::curves::redeem_fee_curve;

  use super::{clamped_redeem_fee, FeeBasis};

  const AMOUNT: UFix64<N9> = UFix64::constant(5_000_000_000);

  fn fees() -> Result<InterpolatedRedeemFees> {
    Ok(InterpolatedRedeemFees::new(redeem_fee_curve()?))
  }

  #[test]
  fn clamped_fee_equals_strict_inside_domain() -> Result<()> {
    let fees = fees()?;
    let cr = CR::Finite(UFix64::new(1_300_000_000));
    let strict = fees.apply_fee(cr, AMOUNT)?;
    let (clamped, basis) = clamped_redeem_fee(&fees, cr, AMOUNT)?;
    assert_eq!(basis, FeeBasis::CurrentCr);
    assert_eq!(clamped.fees_extracted, strict.fees_extracted);
    assert_eq!(clamped.amount_remaining, strict.amount_remaining);
    Ok(())
  }

  #[test]
  fn clamped_fee_at_domain_edge_is_current_cr() -> Result<()> {
    let fees = fees()?;
    let cr = CR::Finite(UFix64::new(1_500_000_000));
    let (_, basis) = clamped_redeem_fee(&fees, cr, AMOUNT)?;
    assert_eq!(basis, FeeBasis::CurrentCr);
    Ok(())
  }

  #[test]
  fn clamped_fee_truncates_like_strict() -> Result<()> {
    // 1.500005 truncates to 1.50000 in N5: strict accepts, so CurrentCr.
    let fees = fees()?;
    let cr = CR::Finite(UFix64::new(1_500_005_000));
    assert!(fees.apply_fee(cr, AMOUNT).is_ok());
    let (_, basis) = clamped_redeem_fee(&fees, cr, AMOUNT)?;
    assert_eq!(basis, FeeBasis::CurrentCr);
    Ok(())
  }

  #[test]
  fn clamped_fee_above_domain_uses_edge_fee() -> Result<()> {
    let fees = fees()?;
    let edge =
      fees.apply_fee(CR::Finite(UFix64::new(1_500_000_000)), AMOUNT)?;
    let above = CR::Finite(UFix64::new(3_000_000_000));
    assert!(fees.apply_fee(above, AMOUNT).is_err());
    let (clamped, basis) = clamped_redeem_fee(&fees, above, AMOUNT)?;
    assert_eq!(basis, FeeBasis::RedeemMaxCr);
    assert_eq!(clamped.fees_extracted, edge.fees_extracted);
    assert_eq!(clamped.amount_remaining, edge.amount_remaining);
    Ok(())
  }

  #[test]
  fn clamped_fee_handles_infinite_cr() -> Result<()> {
    let fees = fees()?;
    let edge =
      fees.apply_fee(CR::Finite(UFix64::new(1_500_000_000)), AMOUNT)?;
    let (clamped, basis) = clamped_redeem_fee(&fees, CR::Infinite, AMOUNT)?;
    assert_eq!(basis, FeeBasis::RedeemMaxCr);
    assert_eq!(clamped.amount_remaining, edge.amount_remaining);
    Ok(())
  }

  #[test]
  fn clamped_fee_conserves_amount() -> Result<()> {
    let fees = fees()?;
    let (clamped, _) = clamped_redeem_fee(&fees, CR::Infinite, AMOUNT)?;
    assert_eq!(
      clamped
        .fees_extracted
        .checked_add(&clamped.amount_remaining),
      Some(AMOUNT)
    );
    Ok(())
  }
}
