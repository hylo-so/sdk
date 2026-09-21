//! State-derived sHYUSD redemption rate.
//!
//! A RATE for an oracle-feed consumer, never an executable quote. It
//! multiplies the sHYUSD -> hyUSD earn-pool exit rate (NAV net of the
//! withdrawal fee) by the best USD value among the hyUSD redemption
//! lanes, each priced with the indicative math at a reference amount and
//! valued at the lower oracle bound.
//!
//! # Availability
//!
//! The reading survives every route gate (pause, overdue harvest, oracle
//! window), and a collateral ratio above the redeem fee-curve domain:
//! such a lane is priced with the fee at the domain edge and reports
//! [`FeeBasis::RedeemMaxCr`]. A lane is absent only when it cannot price
//! the reference at all (vault cannot cover it, LST epoch price missing,
//! arithmetic overflow).

use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, ensure, Result};
use fix::prelude::*;
use hylo_core::earn_pool_math::lp_token_nav;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fees::controller::FeeExtract;
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, USDC};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{FeeBasis, TokenOperation};
use crate::{Local, LST};

/// One hyUSD redemption lane priced at the reference amount.
#[derive(Debug, Clone, PartialEq)]
pub struct RedemptionLane {
  /// Collateral mint the lane redeems into.
  pub mint: Pubkey,
  /// Route gates pass and the fee is unclamped: executable now.
  pub open: bool,
  /// Which collateral ratio priced the redeem fee.
  pub fee_basis: FeeBasis,
  /// Net output in the lane token's own decimals.
  pub amount_out: UFixValue64,
  /// `amount_out` valued at the lower oracle bound.
  pub usd_out: UFix64<N9>,
  /// USD per hyUSD through this lane.
  pub hyusd_usd_rate: UFix64<N9>,
  /// USD per sHYUSD through this lane.
  pub shyusd_usd_rate: UFix64<N9>,
}

/// The sHYUSD redemption rate and the lanes behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct RedemptionRate {
  /// hyUSD per sHYUSD: earn-pool NAV net of the withdrawal fee.
  pub shyusd_hyusd_rate: UFix64<N9>,
  /// hyUSD amount every lane was priced at.
  pub reference_hyusd: UFix64<N6>,
  /// Lanes that priced the reference.
  pub lanes: Vec<RedemptionLane>,
  /// Lane with the highest `usd_out`.
  pub best: RedemptionLane,
}

impl<C: SolanaClock> ProtocolState<C> {
  /// hyUSD per sHYUSD at `N9`: NAV net of the withdrawal fee. Ignores
  /// the withdrawal limiter, which gates execution and not value.
  fn shyusd_exit_rate(&self) -> Result<UFix64<N9>> {
    let nav: UFix64<N6> = lp_token_nav(
      UFix64::new(self.hyusd_pool.amount),
      UFix64::new(self.shyusd_mint.supply),
    )?;
    let withdrawal_fee: UFix64<N4> =
      self.pool_config.withdrawal_fee.try_into()?;
    Ok(FeeExtract::new(withdrawal_fee, nav.convert::<N9>())?.amount_remaining)
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

  /// Lowers one priced lane. `None` drops the lane.
  fn redemption_lane<OUT>(
    &self,
    reference: UFix64<N6>,
    shyusd_hyusd_rate: UFix64<N9>,
    priced: Result<(UFix64<OUT::Exp>, FeeBasis), CoreError>,
    usd_price_lower: Option<UFix64<N9>>,
  ) -> Option<RedemptionLane>
  where
    OUT: TokenMint,
    ProtocolState<C>: TokenOperation<HYUSD, OUT>,
    UFix64<OUT::Exp>: FixExt,
  {
    let (out_amount, fee_basis) = priced.ok()?;
    let usd_out = out_amount
      .checked_convert::<N9>()?
      .mul_div_floor(usd_price_lower?, UFix64::<N9>::one())?;
    let hyusd_usd_rate =
      usd_out.mul_div_floor(UFix64::<N6>::one(), reference)?;
    let shyusd_usd_rate =
      shyusd_hyusd_rate.mul_div_floor(hyusd_usd_rate, UFix64::<N9>::one())?;
    let open = TokenOperation::<HYUSD, OUT>::preconditions(self).is_ok()
      && fee_basis == FeeBasis::CurrentCr;
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
        self
          .redeem_stablecoin_lst_indicative::<JITOSOL>(reference)
          .map(|(op, basis)| (op.out_amount, basis)),
        self.lst_usd_lower::<JITOSOL>(),
      ),
      self.redemption_lane::<HYLOSOL>(
        reference,
        exit,
        self
          .redeem_stablecoin_lst_indicative::<HYLOSOL>(reference)
          .map(|(op, basis)| (op.out_amount, basis)),
        self.lst_usd_lower::<HYLOSOL>(),
      ),
      self.redemption_lane::<CBBTC>(
        reference,
        exit,
        self
          .redeem_stablecoin_exo_indicative::<CBBTC>(reference)
          .map(|(op, basis)| (op.out_amount, basis)),
        Some(self.cbbtc_pair.context.collateral_usd_price().lower),
      ),
      self.redemption_lane::<HYPE>(
        reference,
        exit,
        self
          .redeem_stablecoin_exo_indicative::<HYPE>(reference)
          .map(|(op, basis)| (op.out_amount, basis)),
        Some(self.hype_pair.context.collateral_usd_price().lower),
      ),
      self.redemption_lane::<USDC>(
        reference,
        exit,
        TokenOperation::<HYUSD, USDC>::compute_output_ungated(self, reference)
          .map(|op| (op.out_amount, FeeBasis::CurrentCr)),
        Some(self.usdc_exchange_state.usdc_usd_spot),
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
