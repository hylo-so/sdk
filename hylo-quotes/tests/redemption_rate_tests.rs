//! State derived sHYUSD redemption rate.

mod common;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::clock::Clock;
use anyhow::{anyhow, Result};
use common::load_state;
use fix::prelude::*;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{
  TokenMint, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, SHYUSD, USDC,
};
use hylo_quotes::prelude::{
  FeeBase, ProtocolState, RedemptionLane, RedemptionRate, TokenOperation,
};
use hylo_quotes::protocol_state::ExoPairState;

const REFERENCE: UFix64<N6> = UFix64::constant(1_000_000_000);

/// CR inside the redeem fee curve domain, which ends at 1.50.
const CR_IN_DOMAIN: UFix64<N9> = UFix64::constant(1_400_000_000);

/// CR above the redeem fee curve domain.
const CR_ABOVE_DOMAIN: UFix64<N9> = UFix64::constant(1_600_000_000);

/// Collateral that puts a pair at `cr`.
///
/// ```txt
///                 cr * supply
/// collateral  =  -------------
///                  usd_lower
/// ```
fn collateral_for_cr(
  supply: UFix64<N6>,
  usd_lower: UFix64<N9>,
  cr: UFix64<N9>,
) -> Result<UFix64<N9>> {
  supply
    .checked_convert::<N9>()
    .and_then(|supply| supply.mul_div_floor(cr, usd_lower))
    .ok_or_else(|| anyhow!("collateral for target CR overflows"))
}

/// Rewrites total SOL so the LST pair projects to `target_cr`.
fn with_lst_cr(
  mut state: ProtocolState<Clock>,
  target_cr: UFix64<N9>,
) -> Result<ProtocolState<Clock>> {
  let supply = state.exchange_context.virtual_stablecoin_supply()?;
  let lower = state.exchange_context.sol_usd_price.lower;
  state.exchange_context.total_sol =
    collateral_for_cr(supply, lower, target_cr)?;
  Ok(state)
}

/// Rewrites an exo pair's collateral so it projects to `target_cr`.
fn with_exo_cr(
  pair: &mut ExoPairState<Clock>,
  target_cr: UFix64<N9>,
) -> Result<()> {
  let supply = pair.context.virtual_stablecoin_supply()?;
  let lower = pair.context.collateral_usd_price.lower;
  pair.context.total_collateral = collateral_for_cr(supply, lower, target_cr)?;
  Ok(())
}

fn lanes(rate: &RedemptionRate) -> impl Iterator<Item = &RedemptionLane> {
  std::iter::once(&rate.best).chain(&rate.other_lanes)
}

fn lane(rate: &RedemptionRate, mint: Pubkey) -> Result<&RedemptionLane> {
  lanes(rate)
    .find(|lane| lane.mint == mint)
    .ok_or_else(|| anyhow!("no lane for {mint}"))
}

fn has_lane(rate: &RedemptionRate, mint: Pubkey) -> bool {
  lanes(rate).any(|lane| lane.mint == mint)
}

#[test]
fn raw_mainnet_snapshot_has_a_rate() -> Result<()> {
  let rate = RedemptionRate::new(&load_state()?, REFERENCE)?;
  assert_eq!(rate.reference_hyusd, REFERENCE);
  assert!(rate.best.usd_out > UFix64::zero());
  Ok(())
}

#[test]
fn exit_rate_equals_strict_withdraw_of_one_shyusd() -> Result<()> {
  let state = load_state()?;
  let rate = RedemptionRate::new(&state, REFERENCE)?;
  let quote = TokenOperation::<SHYUSD, HYUSD>::compute_output(
    &state,
    UFix64::<N6>::one(),
  )?;
  assert_eq!(rate.shyusd_hyusd_rate, quote.out_amount);
  Ok(())
}

#[test]
fn rate_is_near_par() -> Result<()> {
  let rate = RedemptionRate::new(&load_state()?, REFERENCE)?;
  assert!(rate.best.hyusd_usd_rate > UFix64::new(950_000_000));
  assert!(rate.best.hyusd_usd_rate <= UFix64::one());
  assert!(lanes(&rate).all(|lane| {
    lane.hyusd_usd_rate > UFix64::<N9>::new(990_000_000)
      && lane.hyusd_usd_rate <= UFix64::one()
  }));
  Ok(())
}

#[test]
fn best_lane_leads_and_other_lanes_descend() -> Result<()> {
  let rate = RedemptionRate::new(&load_state()?, REFERENCE)?;
  assert!(rate.other_lanes.iter().all(|lane| lane < &rate.best));
  assert!(rate.other_lanes.windows(2).all(|pair| pair[0] > pair[1]));
  Ok(())
}

#[test]
fn lanes_rank_by_usd_out_then_mint() -> Result<()> {
  let rate = RedemptionRate::new(&load_state()?, REFERENCE)?;
  let tied = RedemptionLane {
    mint: Pubkey::new_from_array([u8::MAX; 32]),
    ..rate.best.clone()
  };
  assert!(tied > rate.best);
  assert_ne!(tied, rate.best);
  Ok(())
}

#[test]
fn lane_rates_compose() -> Result<()> {
  let rate = RedemptionRate::new(&load_state()?, REFERENCE)?;
  let exit = rate
    .shyusd_hyusd_rate
    .checked_convert::<N9>()
    .ok_or_else(|| anyhow!("exit rate overflows N9"))?;
  assert!(lanes(&rate).all(|lane| {
    Some(lane.shyusd_usd_rate) == exit.mul_floor(lane.hyusd_usd_rate)
  }));
  Ok(())
}

#[test]
fn lane_usd_out_is_amount_times_lower_price() -> Result<()> {
  let state = load_state()?;
  let rate = RedemptionRate::new(&state, REFERENCE)?;

  let jitosol = lane(&rate, JITOSOL::MINT)?;
  let jitosol_price: LstSolPrice = state.jitosol_header.price_sol.into();
  let jitosol_usd = UFix64::<N9>::try_from(jitosol.amount_out)?
    .mul_floor(
      jitosol_price.get_epoch_price(state.exchange_context.clock.epoch())?,
    )
    .and_then(|sol| sol.mul_floor(state.exchange_context.sol_usd_price.lower))
    .ok_or_else(|| anyhow!("jitosol usd_out overflows"))?;
  assert_eq!(jitosol.usd_out, jitosol_usd);

  let cbbtc = lane(&rate, CBBTC::MINT)?;
  let cbbtc_usd = UFix64::<N8>::try_from(cbbtc.amount_out)?
    .checked_convert::<N9>()
    .and_then(|amount| {
      amount.mul_floor(state.cbbtc_pair.context.collateral_usd_price.lower)
    })
    .ok_or_else(|| anyhow!("cbbtc usd_out overflows"))?;
  assert_eq!(cbbtc.usd_out, cbbtc_usd);
  Ok(())
}

#[test]
fn in_domain_lane_equals_strict_quote() -> Result<()> {
  let mut state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  with_exo_cr(&mut state.cbbtc_pair, CR_IN_DOMAIN)?;
  let rate = RedemptionRate::new(&state, REFERENCE)?;
  let jitosol =
    TokenOperation::<HYUSD, JITOSOL>::compute_output(&state, REFERENCE)?;
  let cbbtc =
    TokenOperation::<HYUSD, CBBTC>::compute_output(&state, REFERENCE)?;
  assert_eq!(
    lane(&rate, JITOSOL::MINT)?.amount_out,
    jitosol.out_amount.into()
  );
  assert_eq!(
    lane(&rate, CBBTC::MINT)?.amount_out,
    cbbtc.out_amount.into()
  );
  assert_eq!(lane(&rate, JITOSOL::MINT)?.execution, Ok(()));
  assert_eq!(lane(&rate, CBBTC::MINT)?.execution, Ok(()));
  Ok(())
}

#[test]
fn above_domain_lane_prices_at_edge_fee_and_reports_it() -> Result<()> {
  let mut state = with_lst_cr(load_state()?, CR_ABOVE_DOMAIN)?;
  with_exo_cr(&mut state.cbbtc_pair, CR_ABOVE_DOMAIN)?;
  with_exo_cr(&mut state.hype_pair, CR_ABOVE_DOMAIN)?;
  let rate = RedemptionRate::new(&state, REFERENCE)?;
  [JITOSOL::MINT, HYLOSOL::MINT, CBBTC::MINT, HYPE::MINT]
    .into_iter()
    .try_for_each(|mint| {
      let collateral_lane = lane(&rate, mint)?;
      assert_eq!(
        collateral_lane.execution,
        Err(CoreError::NoValidStablecoinRedeemFee)
      );
      assert!(collateral_lane.usd_out > UFix64::zero());
      Ok(())
    })
}

#[test]
fn fee_base_plus_strict_fee_equals_quote() -> Result<()> {
  let mut state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  with_exo_cr(&mut state.cbbtc_pair, CR_IN_DOMAIN)?;

  let lst_price: LstSolPrice = state.jitosol_header.price_sol.into();
  let lst_out = FeeBase::<HYUSD, JITOSOL>::fee_base(&state, REFERENCE)?;
  let lst_net = state
    .exchange_context
    .stablecoin_redeem_fee(&lst_price, lst_out)?
    .amount_remaining;
  let lst_quote =
    TokenOperation::<HYUSD, JITOSOL>::compute_output(&state, REFERENCE)?;
  assert_eq!(lst_out, lst_quote.fee_base);
  assert_eq!(lst_net, lst_quote.out_amount);

  let exo_out = FeeBase::<HYUSD, CBBTC>::fee_base(&state, REFERENCE)?;
  let exo_quote =
    TokenOperation::<HYUSD, CBBTC>::compute_output(&state, REFERENCE)?;
  assert_eq!(exo_out, exo_quote.fee_base);

  let shares = UFix64::<N6>::new(1_000_000_000);
  let hyusd_out = FeeBase::<SHYUSD, HYUSD>::fee_base(&state, shares)?;
  let withdraw_quote =
    TokenOperation::<SHYUSD, HYUSD>::compute_output(&state, shares)?;
  assert_eq!(hyusd_out, withdraw_quote.fee_base);
  Ok(())
}

#[test]
fn strict_quote_reports_fee_before_burn_floor() -> Result<()> {
  // Supply just above the reference leaves the burn below the floor and
  // pushes the CR above the curve; the protocol reports the fee first.
  let mut state = load_state()?;
  let supply = REFERENCE
    .checked_add(&state.cbbtc_pair.supply_floor)
    .and_then(|floor| floor.checked_sub(&UFix64::new(1)))
    .ok_or_else(|| anyhow!("supply arithmetic"))?;
  state.cbbtc_pair.context.virtual_stablecoin.supply = supply.into();
  let quote =
    TokenOperation::<HYUSD, CBBTC>::compute_output_ungated(&state, REFERENCE);
  assert_eq!(quote.err(), Some(CoreError::NoValidStablecoinRedeemFee));
  Ok(())
}

#[test]
fn empty_vault_drops_the_lane() -> Result<()> {
  let mut state = load_state()?;
  state.jitosol_vault_balance = UFix64::zero();
  state.hype_pair.context.total_collateral = UFix64::zero();
  let rate = RedemptionRate::new(&state, REFERENCE)?;
  assert!(!has_lane(&rate, JITOSOL::MINT));
  assert!(!has_lane(&rate, HYPE::MINT));
  assert!(has_lane(&rate, HYLOSOL::MINT));
  Ok(())
}

#[test]
fn paused_protocol_keeps_the_rate() -> Result<()> {
  let open_state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  let open_rate = RedemptionRate::new(&open_state, REFERENCE)?;
  assert_eq!(lane(&open_rate, JITOSOL::MINT)?.execution, Ok(()));

  let mut paused_state = open_state;
  paused_state.protocol_paused = true;
  let paused_rate = RedemptionRate::new(&paused_state, REFERENCE)?;
  assert_eq!(
    paused_rate.best.shyusd_usd_rate,
    open_rate.best.shyusd_usd_rate
  );
  assert!(lanes(&paused_rate)
    .all(|lane| lane.execution == Err(CoreError::ProtocolPaused)));
  Ok(())
}

#[test]
fn exhausted_withdrawal_limiter_keeps_the_rate() -> Result<()> {
  let untouched = RedemptionRate::new(&load_state()?, REFERENCE)?;
  let mut state = load_state()?;
  state.pool_config.withdrawal_limiter.limit = UFixValue64::new(0, -6).into();
  assert!(TokenOperation::<SHYUSD, HYUSD>::compute_output_ungated(
    &state,
    UFix64::<N6>::one()
  )
  .is_err());
  let rate = RedemptionRate::new(&state, REFERENCE)?;
  assert_eq!(rate.shyusd_hyusd_rate, untouched.shyusd_hyusd_rate);
  Ok(())
}

#[test]
fn overdue_harvest_closes_the_lane_but_keeps_the_rate() -> Result<()> {
  let state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  let open_rate = RedemptionRate::new(&state, REFERENCE)?;
  let open_jitosol = lane(&open_rate, JITOSOL::MINT)?;
  assert_eq!(open_jitosol.execution, Ok(()));

  let mut overdue_state = state;
  overdue_state.yield_harvest_epoch = overdue_state
    .yield_harvest_epoch
    .checked_sub(1)
    .ok_or_else(|| anyhow!("yield_harvest_epoch underflow"))?;
  let overdue_rate = RedemptionRate::new(&overdue_state, REFERENCE)?;
  let overdue_jitosol = lane(&overdue_rate, JITOSOL::MINT)?;
  assert_eq!(
    overdue_jitosol.execution,
    Err(CoreError::YieldHarvestNotRun)
  );
  assert_eq!(
    overdue_jitosol.shyusd_usd_rate,
    open_jitosol.shyusd_usd_rate
  );
  Ok(())
}

#[test]
fn stale_sol_oracle_drops_the_lst_lanes() -> Result<()> {
  let mut state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  let fresh_rate = RedemptionRate::new(&state, REFERENCE)?;
  assert!(has_lane(&fresh_rate, JITOSOL::MINT));
  assert!(has_lane(&fresh_rate, HYLOSOL::MINT));

  state.sol_usd_publish_time = 0;
  let stale_rate = RedemptionRate::new(&state, REFERENCE)?;
  assert!(!has_lane(&stale_rate, JITOSOL::MINT));
  assert!(!has_lane(&stale_rate, HYLOSOL::MINT));
  Ok(())
}

#[test]
fn stale_exo_oracle_drops_only_that_lane() -> Result<()> {
  let mut state = load_state()?;
  assert!(has_lane(
    &RedemptionRate::new(&state, REFERENCE)?,
    CBBTC::MINT
  ));

  state.cbbtc_pair.oracle_publish_time = 0;
  let rate = RedemptionRate::new(&state, REFERENCE)?;
  assert!(!has_lane(&rate, CBBTC::MINT));
  assert!(has_lane(&rate, HYPE::MINT));
  Ok(())
}

#[test]
fn no_priced_lane_fails() -> Result<()> {
  let mut state = load_state()?;
  state.sol_usd_publish_time = 0;
  state.cbbtc_pair.oracle_publish_time = 0;
  state.hype_pair.oracle_publish_time = 0;
  state.usdc_exchange_state.vault_balance = UFix64::zero();
  assert_eq!(
    RedemptionRate::new(&state, REFERENCE).err(),
    Some(CoreError::NoRedemptionLane)
  );
  Ok(())
}

#[test]
fn usdc_lane_prices_at_spot_capped_at_par() -> Result<()> {
  let mut state = load_state()?;
  // Raw snapshot USDC capacity cannot absorb the reference.
  state.usdc_exchange_state.virtual_stablecoin.supply =
    UFix64::<N6>::new(1_000_000_000_000).into();
  state.usdc_exchange_state.vault_balance =
    UFix64::<N6>::new(1_000_000_000_000);

  state.usdc_exchange_state.usdc_usd_spot = UFix64::new(1_050_000_000);
  let above_par = RedemptionRate::new(&state, REFERENCE)?;
  let above_lane = lane(&above_par, USDC::MINT)?;
  assert!(above_lane.hyusd_usd_rate <= UFix64::one());
  assert_eq!(above_lane.execution, Err(CoreError::ParToleranceExceeded));

  state.usdc_exchange_state.usdc_usd_spot = UFix64::new(970_000_000);
  let below_par = RedemptionRate::new(&state, REFERENCE)?;
  let below_lane = lane(&below_par, USDC::MINT)?;
  let expected = UFix64::<N6>::try_from(below_lane.amount_out)?
    .checked_convert::<N9>()
    .and_then(|usdc| usdc.mul_floor(UFix64::new(970_000_000)))
    .ok_or_else(|| anyhow!("usdc usd_out overflows"))?;
  assert_eq!(below_lane.usd_out, expected);
  assert_eq!(below_lane.execution, Err(CoreError::ParToleranceExceeded));
  Ok(())
}

#[test]
fn zero_reference_fails() -> Result<()> {
  assert_eq!(
    RedemptionRate::new(&load_state()?, UFix64::zero()).err(),
    Some(CoreError::ZeroAmount)
  );
  Ok(())
}
