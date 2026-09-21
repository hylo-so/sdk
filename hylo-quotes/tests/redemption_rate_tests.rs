//! State-derived sHYUSD redemption rate.

mod common;

use anyhow::{anyhow, Result};
use common::{
  load_state, with_exo_cr, with_lst_cr, CR_ABOVE_DOMAIN, CR_IN_DOMAIN,
};
use fix::prelude::*;
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::solana_clock::SolanaClock;
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
fn every_lane_hyusd_rate_is_near_par() -> Result<()> {
  // On the raw snapshot every lane's hyUSD leg redeems in a narrow band
  // just under par (0.99664..0.99771). A wrong oracle bound or a wrong
  // price on any single lane, not only the best one, would fall outside
  // this band.
  let rate = load_state()?.redemption_rate(REFERENCE)?;
  assert!(rate.lanes.iter().all(|lane| {
    lane.hyusd_usd_rate > UFix64::<N9>::new(990_000_000)
      && lane.hyusd_usd_rate <= UFix64::one()
  }));
  Ok(())
}

#[test]
fn lane_usd_out_matches_amount_times_lower_price() -> Result<()> {
  // Recomputes `usd_out` independently of `redemption_rate`'s own
  // formula, straight from state, to pin `usd_out = amount_out x
  // lower_price` rather than trusting the production code to check
  // itself.
  let state = load_state()?;
  let rate = state.redemption_rate(REFERENCE)?;

  let jitosol =
    lane(&rate, JITOSOL::MINT).ok_or_else(|| anyhow!("no JITOSOL lane"))?;
  let jitosol_price: LstSolPrice = state.jitosol_header.price_sol.into();
  let jitosol_lower = jitosol_price
    .get_epoch_price(state.exchange_context.clock.epoch())?
    .mul_div_floor(
      state.exchange_context.sol_usd_price.lower,
      UFix64::<N9>::one(),
    )
    .ok_or_else(|| anyhow!("jitosol lower price overflow"))?;
  let jitosol_amount = UFix64::<N9>::try_from(jitosol.amount_out)?;
  let jitosol_expected = jitosol_amount
    .mul_div_floor(jitosol_lower, UFix64::<N9>::one())
    .ok_or_else(|| anyhow!("jitosol usd_out overflow"))?;
  assert_eq!(
    jitosol.usd_out, jitosol_expected,
    "jitosol usd_out mismatch: lane={:?} independent={:?}",
    jitosol.usd_out, jitosol_expected
  );

  let cbbtc =
    lane(&rate, CBBTC::MINT).ok_or_else(|| anyhow!("no CBBTC lane"))?;
  let cbbtc_lower = state.cbbtc_pair.context.collateral_usd_price.lower;
  let cbbtc_amount = UFix64::<N8>::try_from(cbbtc.amount_out)?
    .checked_convert::<N9>()
    .ok_or_else(|| anyhow!("cbbtc amount convert overflow"))?;
  let cbbtc_expected = cbbtc_amount
    .mul_div_floor(cbbtc_lower, UFix64::<N9>::one())
    .ok_or_else(|| anyhow!("cbbtc usd_out overflow"))?;
  assert_eq!(
    cbbtc.usd_out, cbbtc_expected,
    "cbbtc usd_out mismatch: lane={:?} independent={:?}",
    cbbtc.usd_out, cbbtc_expected
  );
  Ok(())
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
fn in_domain_lane_opens_and_closes_on_pause() -> Result<()> {
  // `paused_protocol_keeps_the_rate` is vacuous on the raw snapshot:
  // every lane is already closed by `RedeemMaxCr`. Build a state where
  // the LST lanes start open, and check pausing actually closes them.
  let state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  let open_rate = state.redemption_rate(REFERENCE)?;
  let open_jito = lane(&open_rate, JITOSOL::MINT)
    .ok_or_else(|| anyhow!("no JITOSOL lane"))?;
  let open_hylo = lane(&open_rate, HYLOSOL::MINT)
    .ok_or_else(|| anyhow!("no HYLOSOL lane"))?;
  assert_eq!(open_jito.fee_basis, FeeBasis::CurrentCr);
  assert_eq!(open_hylo.fee_basis, FeeBasis::CurrentCr);
  assert!(open_jito.open, "expected the JITOSOL lane to start open");
  assert!(open_hylo.open, "expected the HYLOSOL lane to start open");

  let mut paused_state = state;
  paused_state.protocol_paused = true;
  let paused_rate = paused_state.redemption_rate(REFERENCE)?;
  let paused_jito = lane(&paused_rate, JITOSOL::MINT)
    .ok_or_else(|| anyhow!("no JITOSOL lane"))?;
  let paused_hylo = lane(&paused_rate, HYLOSOL::MINT)
    .ok_or_else(|| anyhow!("no HYLOSOL lane"))?;
  assert!(!paused_jito.open);
  assert!(!paused_hylo.open);
  assert_eq!(paused_jito.shyusd_usd_rate, open_jito.shyusd_usd_rate);
  assert_eq!(paused_hylo.shyusd_usd_rate, open_hylo.shyusd_usd_rate);
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
