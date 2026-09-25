//! State-derived sHYUSD redemption rate for oracle feeds. Not a quote.
//!
//! `rate = sHYUSD exit rate * best hyUSD lane value`. Lanes skip route
//! gates and price the redeem fee at `min(projected CR, curve x_max)`.
//! A lane drops when it cannot absorb the reference amount or its
//! collateral oracle is outside the stablecoin oracle window.

use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, ensure, Result};
use fix::prelude::*;
use hylo_core::collateral_ratio::CollateralRatio;
use hylo_core::error::CoreError;
use hylo_core::fees::controller::FeeExtract;
use hylo_core::fees::curve_controller::{
  InterpolatedFeeController, InterpolatedRedeemFees,
};
use hylo_core::solana_clock::SolanaClock;
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
  /// Projected CR, as in the strict math. Always used by USDC.
  CurrentCr,
  /// Projected CR above the fee-curve domain; fee at the domain edge.
  RedeemMaxCr,
}

/// Stablecoin redeem fee at `min(projected CR, curve x_max)`.
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
  /// Strict quote at `reference_hyusd` succeeds.
  pub open: bool,
  /// Which collateral ratio priced the redeem fee.
  pub fee_basis: FeeBasis,
  /// Net output in the lane token's own decimals.
  pub amount_out: UFixValue64,
  /// `amount_out` at the lower oracle bound, in N6 precision
  /// (USDC: spot capped at 1).
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
  /// hyUSD per sHYUSD: net withdraw of the reference, per sHYUSD.
  pub shyusd_hyusd_rate: UFix64<N9>,
  /// hyUSD amount every lane was priced at.
  pub reference_hyusd: UFix64<N6>,
  /// Lanes that priced the reference.
  pub lanes: Vec<RedemptionLane>,
  /// Lane with the highest `usd_out`. May be closed.
  pub best: RedemptionLane,
}

/// Net lane output, its fee basis, and its USD value at the lower bound.
struct LaneValue<Exp> {
  amount_out: UFix64<Exp>,
  fee_basis: FeeBasis,
  usd_out: UFix64<N6>,
}

impl<C: SolanaClock> ProtocolState<C> {
  /// hyUSD per sHYUSD: net withdraw of `reference` sHYUSD (capped at
  /// supply) per sHYUSD. Ignores the withdrawal limiter.
  ///
  /// # Errors
  /// * Zero sHYUSD supply
  /// * Withdraw arithmetic or withdrawal fee
  fn shyusd_exit_rate(&self, reference: UFix64<N6>) -> Result<UFix64<N9>> {
    let shares = reference.min(UFix64::new(self.shyusd_mint.supply));
    let (_, extract) = self.withdraw_shyusd_gross(shares)?;
    UFix64::<N9>::one()
      .mul_div_floor(extract.amount_remaining, shares)
      .ok_or_else(|| anyhow!("earn pool has no sHYUSD supply"))
  }

  /// LST out for `reference` hyUSD, net of the clamped redeem fee.
  ///
  /// # Errors
  /// * Stale SOL/USD feed
  /// * Conversion, vault or burn capacity, projection, or fee
  fn lst_lane_value<L: LST + Local>(
    &self,
    reference: UFix64<N6>,
  ) -> Result<LaneValue<N9>, CoreError> {
    gate(
      self.sol_usd_in_stablecoin_oracle_window(),
      CoreError::PythOracleOutdated,
    )?;
    let (lst_price, lst_out) =
      self.redeem_stablecoin_lst_gross::<L>(reference)?;
    let context = &self.exchange_context;
    let projected = context.projected_redeem_state(&lst_price, lst_out)?;
    let (extract, fee_basis) = clamped_redeem_fee(
      &context.stablecoin_redeem_fees,
      projected.collateral_ratio,
      lst_out,
    )?;
    let usd_out = context
      .token_conversion(&lst_price)?
      .lst_to_token(extract.amount_remaining, UFix64::one())?;
    Ok(LaneValue {
      amount_out: extract.amount_remaining,
      fee_basis,
      usd_out,
    })
  }

  /// Exo out for `reference` hyUSD, net of the clamped redeem fee.
  ///
  /// # Errors
  /// * Stale collateral feed
  /// * Conversion, collateral or burn capacity, projection, or fee
  fn exo_lane_value<E: Exo>(
    &self,
    reference: UFix64<N6>,
  ) -> Result<LaneValue<E::Exp>, CoreError>
  where
    UFix64<E::Exp>: FixExt,
  {
    let pair = self.exo_pair::<E>()?;
    gate(
      pair.collateral_usd_in_stablecoin_oracle_window(),
      CoreError::PythOracleOutdated,
    )?;
    let collateral_out = self.redeem_stablecoin_exo_gross::<E>(reference)?;
    let context = &pair.context;
    let projected = context.projected_redeem_state(collateral_out)?;
    let (extract, fee_basis) = clamped_redeem_fee(
      &context.stablecoin_redeem_fees,
      projected.collateral_ratio,
      collateral_out,
    )?;
    let amount_out: UFix64<E::Exp> = extract
      .amount_remaining
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    let usd_out = context.exo_conversion().exo_to_token(
      amount_out
        .checked_convert::<N9>()
        .ok_or(CoreError::TokenAmountPrecision)?,
      UFix64::<N9>::one(),
    )?;
    Ok(LaneValue {
      amount_out,
      fee_basis,
      usd_out,
    })
  }

  /// USDC out for `reference` hyUSD. Spot is capped at par because the
  /// par-tolerance gate is skipped.
  fn usdc_lane_value(&self, reference: UFix64<N6>) -> Option<LaneValue<N6>> {
    let amount_out =
      TokenOperation::<HYUSD, USDC>::compute_output_ungated(self, reference)
        .ok()?
        .out_amount;
    let spot = self
      .usdc_exchange_state
      .usdc_usd_spot
      .min(UFix64::<N9>::one());
    Some(LaneValue {
      amount_out,
      fee_basis: FeeBasis::CurrentCr,
      usd_out: amount_out.mul_div_floor(spot, UFix64::<N9>::one())?,
    })
  }

  /// Lowers one valued lane. `None` drops the lane.
  fn redemption_lane<OUT>(
    &self,
    reference: UFix64<N6>,
    shyusd_hyusd_rate: UFix64<N9>,
    valued: Option<LaneValue<OUT::Exp>>,
  ) -> Option<RedemptionLane>
  where
    OUT: TokenMint,
    ProtocolState<C>: TokenOperation<HYUSD, OUT>,
    UFix64<OUT::Exp>: FixExt,
  {
    let LaneValue {
      amount_out,
      fee_basis,
      usd_out,
    } = valued?;
    let usd_out = usd_out.checked_convert::<N9>()?;
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
      amount_out: amount_out.into(),
      usd_out,
      hyusd_usd_rate,
      shyusd_usd_rate,
    })
  }

  /// USD value of one sHYUSD through the best hyUSD redemption lane.
  ///
  /// # Errors
  /// * Zero `reference`
  /// * Earn-pool withdraw, withdrawal fee, or zero sHYUSD supply
  /// * No lane can price `reference`
  pub fn redemption_rate(
    &self,
    reference: UFix64<N6>,
  ) -> Result<RedemptionRate> {
    ensure!(
      reference > UFix64::zero(),
      "redemption reference amount must be nonzero"
    );
    let exit = self.shyusd_exit_rate(reference)?;
    let lanes: Vec<RedemptionLane> = [
      self.redemption_lane::<JITOSOL>(
        reference,
        exit,
        self.lst_lane_value::<JITOSOL>(reference).ok(),
      ),
      self.redemption_lane::<HYLOSOL>(
        reference,
        exit,
        self.lst_lane_value::<HYLOSOL>(reference).ok(),
      ),
      self.redemption_lane::<CBBTC>(
        reference,
        exit,
        self.exo_lane_value::<CBBTC>(reference).ok(),
      ),
      self.redemption_lane::<HYPE>(
        reference,
        exit,
        self.exo_lane_value::<HYPE>(reference).ok(),
      ),
      self.redemption_lane::<USDC>(
        reference,
        exit,
        self.usdc_lane_value(reference),
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
