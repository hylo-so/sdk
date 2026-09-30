//! State derived sHYUSD redemption rate for oracle feeds.
//!
//! Values one sHYUSD as the hyUSD its withdrawal returns, priced through
//! the hyUSD redemption lane with the highest USD output:
//!
//! ```txt
//!                          usd_out
//! hyusd_usd_rate   =  -----------------
//!                      reference_hyusd
//!
//! shyusd_usd_rate  =  shyusd_hyusd_rate * hyusd_usd_rate
//! ```
//!
//! A lane prices when its collateral covers the reference amount and its
//! oracle price is inside the stablecoin window. Route gates and the
//! withdrawal limiter close execution without moving the rate. The redeem
//! fee saturates at `y_max`.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use anchor_lang::prelude::Pubkey;
use fix::prelude::*;
use hylo_core::earn_pool_math::stablecoin_withdrawal_fee;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::virtual_stablecoin::{validate_burn, SUPPLY_FLOOR};
use hylo_idl::tokens::{
  Exo, TokenMint, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, SHYUSD, USDC,
};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{gate, FeeBase, TokenOperation};
use crate::{Local, LST};

/// One hyUSD redemption lane priced at the reference amount.
#[derive(Debug, Clone)]
pub struct RedemptionLane {
  /// Collateral mint the lane redeems into.
  pub mint: Pubkey,
  /// Outcome of the strict quote at the reference amount.
  pub execution: Result<(), CoreError>,
  /// Collateral out, in the lane token's own decimals.
  pub amount_out: UFixValue64,
  /// USD value of `amount_out` at the lower oracle bound.
  pub usd_out: UFix64<N9>,
  /// USD per hyUSD through this lane.
  pub hyusd_usd_rate: UFix64<N9>,
  /// USD per sHYUSD through this lane.
  pub shyusd_usd_rate: UFix64<N9>,
}

impl RedemptionLane {
  fn new<OUT: TokenMint>(
    execution: Result<(), CoreError>,
    amount_out: UFix64<OUT::Exp>,
    usd_out: UFix64<N9>,
    reference_hyusd: UFix64<N6>,
    shyusd_hyusd_rate: UFix64<N6>,
  ) -> Result<RedemptionLane, CoreError> {
    let hyusd_usd_rate = reference_hyusd
      .checked_convert::<N9>()
      .and_then(|reference| usd_out.div_floor(reference))
      .ok_or(CoreError::RedemptionRateOverflow)?;
    let shyusd_usd_rate = shyusd_hyusd_rate
      .checked_convert::<N9>()
      .and_then(|exit| exit.mul_floor(hyusd_usd_rate))
      .ok_or(CoreError::RedemptionRateOverflow)?;
    Ok(RedemptionLane {
      mint: OUT::MINT,
      execution,
      amount_out: amount_out.into(),
      usd_out,
      hyusd_usd_rate,
      shyusd_usd_rate,
    })
  }
}

/// Lanes rank by USD output, then by mint.
impl Ord for RedemptionLane {
  fn cmp(&self, other: &RedemptionLane) -> Ordering {
    self
      .usd_out
      .cmp(&other.usd_out)
      .then_with(|| self.mint.cmp(&other.mint))
  }
}

impl PartialOrd for RedemptionLane {
  fn partial_cmp(&self, other: &RedemptionLane) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for RedemptionLane {
  fn eq(&self, other: &RedemptionLane) -> bool {
    self.cmp(other) == Ordering::Equal
  }
}

impl Eq for RedemptionLane {}

/// The sHYUSD redemption rate and the lanes behind it.
#[derive(Debug, Clone)]
pub struct RedemptionRate {
  /// hyUSD withdrawn for one sHYUSD, net of the withdrawal fee.
  pub shyusd_hyusd_rate: UFix64<N6>,
  /// hyUSD amount every lane was priced at.
  pub reference_hyusd: UFix64<N6>,
  /// Lane with the highest USD output.
  pub best: RedemptionLane,
  /// Remaining lanes, highest USD output first.
  pub other_lanes: Vec<RedemptionLane>,
}

impl RedemptionRate {
  /// Derives the rate from protocol state at a reference hyUSD amount.
  ///
  /// # Errors
  /// * `ZeroAmount` on a zero reference
  /// * `InsufficientEarnPoolLiquidity` on zero sHYUSD supply
  /// * `NoRedemptionLane` when no lane prices the reference
  /// * Withdrawal fee conversion or extraction
  pub fn new<C: SolanaClock>(
    state: &ProtocolState<C>,
    reference_hyusd: UFix64<N6>,
  ) -> Result<RedemptionRate, CoreError> {
    gate(reference_hyusd > UFix64::zero(), CoreError::ZeroAmount)?;
    let shyusd_hyusd_rate = shyusd_exit_rate(state)?;
    let lanes: BTreeSet<RedemptionLane> = [
      lst_lane::<JITOSOL, C>(state, reference_hyusd, shyusd_hyusd_rate),
      lst_lane::<HYLOSOL, C>(state, reference_hyusd, shyusd_hyusd_rate),
      exo_lane::<CBBTC, C>(state, reference_hyusd, shyusd_hyusd_rate),
      exo_lane::<HYPE, C>(state, reference_hyusd, shyusd_hyusd_rate),
      usdc_lane(state, reference_hyusd, shyusd_hyusd_rate),
    ]
    .into_iter()
    .filter_map(Result::ok)
    .collect();
    let best = lanes.last().cloned().ok_or(CoreError::NoRedemptionLane)?;
    let other_lanes = lanes.into_iter().rev().skip(1).collect();
    Ok(RedemptionRate {
      shyusd_hyusd_rate,
      reference_hyusd,
      best,
      other_lanes,
    })
  }
}

/// hyUSD withdrawn for one sHYUSD, net of the withdrawal fee. Skips the
/// withdrawal limiter.
fn shyusd_exit_rate<C: SolanaClock>(
  state: &ProtocolState<C>,
) -> Result<UFix64<N6>, CoreError> {
  let hyusd_out = FeeBase::<SHYUSD, HYUSD>::fee_base(state, UFix64::one())?;
  let withdrawal_fee: UFix64<N4> =
    state.pool_config.withdrawal_fee.try_into()?;
  Ok(stablecoin_withdrawal_fee(hyusd_out, withdrawal_fee)?.amount_remaining)
}

/// Prices `reference_hyusd` through an LST lane at the lower SOL/USD bound.
fn lst_lane<L, C>(
  state: &ProtocolState<C>,
  reference_hyusd: UFix64<N6>,
  shyusd_hyusd_rate: UFix64<N6>,
) -> Result<RedemptionLane, CoreError>
where
  L: LST + Local,
  C: SolanaClock,
  ProtocolState<C>: FeeBase<HYUSD, L> + TokenOperation<HYUSD, L, FeeExp = N9>,
{
  gate(
    state.sol_usd_in_stablecoin_oracle_window(),
    CoreError::PythOracleOutdated,
  )?;
  let context = &state.exchange_context;
  let lst_out = FeeBase::<HYUSD, L>::fee_base(state, reference_hyusd)?;
  validate_burn(
    context.virtual_stablecoin_supply()?,
    reference_hyusd,
    SUPPLY_FLOOR,
  )?;
  let lst_sol_price: LstSolPrice = state.lst_header::<L>()?.price_sol.into();
  let amount_out = context
    .saturating_stablecoin_redeem_fee(&lst_sol_price, lst_out)?
    .amount_remaining;
  let usd_out = context
    .token_conversion(&lst_sol_price)?
    .lst_to_usd(amount_out)?;
  RedemptionLane::new::<L>(
    TokenOperation::<HYUSD, L>::compute_output(state, reference_hyusd)
      .map(|_| ()),
    amount_out,
    usd_out,
    reference_hyusd,
    shyusd_hyusd_rate,
  )
}

/// Prices `reference_hyusd` through an exo lane at the lower oracle bound.
fn exo_lane<E, C>(
  state: &ProtocolState<C>,
  reference_hyusd: UFix64<N6>,
  shyusd_hyusd_rate: UFix64<N6>,
) -> Result<RedemptionLane, CoreError>
where
  E: Exo,
  C: SolanaClock,
  UFix64<E::Exp>: FixExt,
  ProtocolState<C>: FeeBase<HYUSD, E> + TokenOperation<HYUSD, E, FeeExp = N9>,
{
  let pair = state.exo_pair::<E>()?;
  gate(
    pair.collateral_usd_in_stablecoin_oracle_window(),
    CoreError::PythOracleOutdated,
  )?;
  let collateral_out = FeeBase::<HYUSD, E>::fee_base(state, reference_hyusd)?;
  validate_burn(
    pair.context.virtual_stablecoin_supply()?,
    reference_hyusd,
    pair.supply_floor,
  )?;
  let amount_out: UFix64<E::Exp> = pair
    .context
    .saturating_stablecoin_redeem_fee(collateral_out)?
    .amount_remaining
    .checked_convert()
    .ok_or(CoreError::TokenAmountPrecision)?;
  let usd_out = pair.context.exo_conversion().exo_to_usd(
    amount_out
      .checked_convert::<N9>()
      .ok_or(CoreError::TokenAmountPrecision)?,
  )?;
  RedemptionLane::new::<E>(
    TokenOperation::<HYUSD, E>::compute_output(state, reference_hyusd)
      .map(|_| ()),
    amount_out,
    usd_out,
    reference_hyusd,
    shyusd_hyusd_rate,
  )
}

/// Prices `reference_hyusd` through the USDC lane at spot, capped at par.
fn usdc_lane<C: SolanaClock>(
  state: &ProtocolState<C>,
  reference_hyusd: UFix64<N6>,
  shyusd_hyusd_rate: UFix64<N6>,
) -> Result<RedemptionLane, CoreError> {
  let amount_out = TokenOperation::<HYUSD, USDC>::compute_output_ungated(
    state,
    reference_hyusd,
  )?
  .out_amount;
  let spot = state.usdc_exchange_state.usdc_usd_spot.min(UFix64::one());
  let usd_out = amount_out
    .checked_convert::<N9>()
    .and_then(|usdc| usdc.mul_floor(spot))
    .ok_or(CoreError::RedemptionRateOverflow)?;
  RedemptionLane::new::<USDC>(
    TokenOperation::<HYUSD, USDC>::compute_output(state, reference_hyusd)
      .map(|_| ()),
    amount_out,
    usd_out,
    reference_hyusd,
    shyusd_hyusd_rate,
  )
}
