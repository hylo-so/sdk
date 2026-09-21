//! State-derived sHYUSD redemption rate.

mod common;

use anyhow::Result;
use common::{
  load_state, with_exo_cr, with_lst_cr, CR_ABOVE_DOMAIN, CR_IN_DOMAIN,
};
use fix::prelude::*;
use hylo_idl::tokens::{TokenMint, CBBTC, HYLOSOL, HYPE, JITOSOL};
use hylo_quotes::prelude::{FeeBasis, RedemptionLane, RedemptionRate};

const REFERENCE: UFix64<N6> = UFix64::constant(1_000_000_000);

fn lane(
  rate: &RedemptionRate,
  mint: anchor_lang::prelude::Pubkey,
) -> Option<&RedemptionLane> {
  rate.lanes.iter().find(|lane| lane.mint == mint)
}

#[test]
fn raw_mainnet_snapshot_has_a_rate() -> Result<()> {
  // Every collateral pair is above 150% CR in this snapshot, and the
  // strict math refuses all five lanes. The rate must still exist.
  let rate = load_state()?.redemption_rate(REFERENCE)?;
  assert!(rate.lanes.len() >= 4);
  assert!(rate
    .lanes
    .iter()
    .all(|lane| lane.fee_basis == FeeBasis::RedeemMaxCr && !lane.open));
  assert_eq!(rate.reference_hyusd, REFERENCE);
  Ok(())
}

#[test]
fn rate_is_near_par_and_nav() -> Result<()> {
  let rate = load_state()?.redemption_rate(REFERENCE)?;
  // hyUSD redeems near 1 USD: above 0.95, never above 1.
  assert!(rate.best.hyusd_usd_rate > UFix64::new(950_000_000));
  assert!(rate.best.hyusd_usd_rate <= UFix64::one());
  // Snapshot NAV is 18851015400385 / 12665223434433 = 1.4884,
  // withdrawal fee 0.10% -> 1.4869.
  assert!(rate.shyusd_hyusd_rate > UFix64::new(1_486_000_000));
  assert!(rate.shyusd_hyusd_rate < UFix64::new(1_488_000_000));
  Ok(())
}

#[test]
fn best_lane_has_max_usd_out() -> Result<()> {
  let rate = load_state()?.redemption_rate(REFERENCE)?;
  let max = rate.lanes.iter().map(|lane| lane.usd_out.bits).max();
  assert_eq!(Some(rate.best.usd_out.bits), max);
  assert!(rate.lanes.contains(&rate.best));
  Ok(())
}

#[test]
fn lane_rates_compose() -> Result<()> {
  let rate = load_state()?.redemption_rate(REFERENCE)?;
  rate.lanes.iter().try_for_each(|lane| {
    let expected = rate
      .shyusd_hyusd_rate
      .mul_div_floor(lane.hyusd_usd_rate, UFix64::<N9>::one());
    assert_eq!(Some(lane.shyusd_usd_rate), expected);
    Ok(())
  })
}

#[test]
fn in_domain_lane_reports_current_cr() -> Result<()> {
  let mut state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  with_exo_cr(&mut state.cbbtc_pair, CR_ABOVE_DOMAIN)?;
  let rate = state.redemption_rate(REFERENCE)?;
  let jito = lane(&rate, JITOSOL::MINT).map(|lane| lane.fee_basis);
  let hylo = lane(&rate, HYLOSOL::MINT).map(|lane| lane.fee_basis);
  let cbbtc = lane(&rate, CBBTC::MINT).map(|lane| lane.fee_basis);
  assert_eq!(jito, Some(FeeBasis::CurrentCr));
  assert_eq!(hylo, Some(FeeBasis::CurrentCr));
  assert_eq!(cbbtc, Some(FeeBasis::RedeemMaxCr));
  Ok(())
}

#[test]
fn clamped_lane_is_never_open() -> Result<()> {
  let rate = load_state()?.redemption_rate(REFERENCE)?;
  assert!(rate
    .lanes
    .iter()
    .filter(|lane| lane.fee_basis == FeeBasis::RedeemMaxCr)
    .all(|lane| !lane.open));
  Ok(())
}

#[test]
fn empty_vault_drops_the_lane() -> Result<()> {
  let mut state = load_state()?;
  state.jitosol_vault_balance = UFix64::zero();
  state.hype_pair.context.total_collateral = UFix64::zero();
  let rate = state.redemption_rate(REFERENCE)?;
  assert!(lane(&rate, JITOSOL::MINT).is_none());
  assert!(lane(&rate, HYPE::MINT).is_none());
  assert!(lane(&rate, HYLOSOL::MINT).is_some());
  Ok(())
}

#[test]
fn paused_protocol_keeps_the_rate() -> Result<()> {
  let open = load_state()?.redemption_rate(REFERENCE)?;
  let mut state = load_state()?;
  state.protocol_paused = true;
  let paused = state.redemption_rate(REFERENCE)?;
  assert_eq!(paused.best.shyusd_usd_rate, open.best.shyusd_usd_rate);
  assert!(paused.lanes.iter().all(|lane| !lane.open));
  Ok(())
}

#[test]
fn zero_reference_fails() -> Result<()> {
  assert!(load_state()?.redemption_rate(UFix64::zero()).is_err());
  Ok(())
}

#[test]
fn no_priced_lane_fails() -> Result<()> {
  let mut state = load_state()?;
  state.jitosol_vault_balance = UFix64::zero();
  state.hylosol_vault_balance = UFix64::zero();
  state.cbbtc_pair.context.total_collateral = UFix64::zero();
  state.hype_pair.context.total_collateral = UFix64::zero();
  state.usdc_exchange_state.vault_balance = UFix64::zero();
  assert!(state.redemption_rate(REFERENCE).is_err());
  Ok(())
}
