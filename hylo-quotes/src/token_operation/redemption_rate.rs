//! State-derived sHYUSD redemption rate.
//!
//! A RATE for an oracle-feed consumer, never an executable quote. It
//! multiplies the sHYUSD -> hyUSD earn-pool exit rate (NAV net of the
//! withdrawal fee) by the best USD value among the hyUSD redemption
//! lanes, each priced with the indicative math at a reference amount.
//! The LST and exo lanes are valued at the lower oracle bound; the USDC
//! lane is valued at the USDC/USD spot capped at par (this SDK version
//! has no USDC price range).
//!
//! An LST/exo lane's redeem leg converts at the UPPER oracle bound
//! while its valuation uses the LOWER bound, so the lane's hyUSD rate
//! is about `nav * lower / upper * (1 - fee)`: the oracle level
//! cancels out, and the rate cannot exceed par.
//!
//! # Availability
//!
//! A route gate (pause, overdue harvest, oracle window) or a collateral
//! ratio above the redeem fee-curve domain never removes a lane by
//! itself: the reading survives both. A CR above the domain instead
//! prices the lane at the domain edge and reports
//! [`FeeBasis::RedeemMaxCr`].
//!
//! A lane is absent whenever its indicative math refuses the reference
//! amount, or the lane's USD valuation cannot be formed. Known causes
//! (not exhaustive): the vault cannot cover the reference amount; the
//! pair's virtual stablecoin supply or burn limit cannot cover it; the
//! LST epoch price is missing; arithmetic overflow. The LST epoch price
//! is a regular, expected absence: after an epoch rollover it stays
//! missing until the LST price crank runs, so both LST lanes drop for
//! that window and `best` moves to another lane.

use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, ensure, Result};
use fix::prelude::*;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fees::controller::FeeExtract;
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, USDC};

use crate::protocol_state::ProtocolState;
use crate::token_operation::exchange::RedeemFeeMode;
use crate::token_operation::{FeeBasis, TokenOperation};
use crate::{Local, LST};

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
        self
          .redeem_stablecoin_lst_quote::<JITOSOL>(
            reference,
            RedeemFeeMode::Clamped,
          )
          .map(|(op, basis)| (op.out_amount, basis)),
        self.lst_usd_lower::<JITOSOL>(),
      ),
      self.redemption_lane::<HYLOSOL>(
        reference,
        exit,
        self
          .redeem_stablecoin_lst_quote::<HYLOSOL>(
            reference,
            RedeemFeeMode::Clamped,
          )
          .map(|(op, basis)| (op.out_amount, basis)),
        self.lst_usd_lower::<HYLOSOL>(),
      ),
      self.redemption_lane::<CBBTC>(
        reference,
        exit,
        self
          .redeem_stablecoin_exo_quote::<CBBTC>(
            reference,
            RedeemFeeMode::Clamped,
          )
          .map(|(op, basis)| (op.out_amount, basis)),
        Some(self.cbbtc_pair.context.collateral_usd_price().lower),
      ),
      self.redemption_lane::<HYPE>(
        reference,
        exit,
        self
          .redeem_stablecoin_exo_quote::<HYPE>(
            reference,
            RedeemFeeMode::Clamped,
          )
          .map(|(op, basis)| (op.out_amount, basis)),
        Some(self.hype_pair.context.collateral_usd_price().lower),
      ),
      self.redemption_lane::<USDC>(
        reference,
        exit,
        TokenOperation::<HYUSD, USDC>::compute_output_indicative(
          self, reference,
        )
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
