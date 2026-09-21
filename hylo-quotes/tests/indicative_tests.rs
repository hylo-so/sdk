//! The indicative tier prices a route as if it could execute.

mod common;

use anyhow::Result;
use common::{
  load_state, with_exo_cr, with_lst_cr, CR_ABOVE_DOMAIN, CR_IN_DOMAIN,
};
use fix::prelude::*;
use hylo_core::error::CoreError;
use hylo_idl::tokens::{CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_quotes::prelude::{TokenOperation, TokenOperationExt};

#[test]
fn indicative_defaults_to_ungated() -> Result<()> {
  let state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  let lst_in = UFix64::<N9>::new(1_000_000_000);
  let hyusd_in = UFix64::<N6>::new(1_000_000);
  assert_eq!(
    state.indicative_output::<JITOSOL, XSOL>(lst_in)?,
    TokenOperation::<JITOSOL, XSOL>::compute_output_ungated(&state, lst_in)?
  );
  assert_eq!(
    state.indicative_output::<JITOSOL, HYLOSOL>(lst_in)?,
    TokenOperation::<JITOSOL, HYLOSOL>::compute_output_ungated(&state, lst_in)?
  );
  assert_eq!(
    state.indicative_output::<HYUSD, SHYUSD>(hyusd_in)?,
    TokenOperation::<HYUSD, SHYUSD>::compute_output_ungated(&state, hyusd_in)?
  );
  Ok(())
}

const REFERENCE: UFix64<N6> = UFix64::constant(1_000_000_000);

#[test]
fn lst_redeem_indicative_equals_ungated_in_domain() -> Result<()> {
  let state = with_lst_cr(load_state()?, CR_IN_DOMAIN)?;
  assert_eq!(
    state.indicative_output::<HYUSD, JITOSOL>(REFERENCE)?,
    TokenOperation::<HYUSD, JITOSOL>::compute_output_ungated(
      &state, REFERENCE
    )?
  );
  Ok(())
}

#[test]
fn lst_redeem_strict_still_fails_above_domain() -> Result<()> {
  let state = with_lst_cr(load_state()?, CR_ABOVE_DOMAIN)?;
  assert_eq!(
    TokenOperation::<HYUSD, JITOSOL>::compute_output_ungated(&state, REFERENCE),
    Err(CoreError::NoValidStablecoinRedeemFee)
  );
  Ok(())
}

#[test]
fn lst_redeem_indicative_prices_above_domain() -> Result<()> {
  let state = with_lst_cr(load_state()?, CR_ABOVE_DOMAIN)?;
  let jito = state.indicative_output::<HYUSD, JITOSOL>(REFERENCE)?;
  let hylo = state.indicative_output::<HYUSD, HYLOSOL>(REFERENCE)?;
  assert!(jito.out_amount > UFix64::zero());
  assert!(hylo.out_amount > UFix64::zero());
  assert_eq!(
    jito.fee_amount.checked_add(&jito.out_amount),
    Some(jito.fee_base)
  );
  assert!(jito.marginal_rate > 0.0);
  Ok(())
}

#[test]
fn lst_redeem_indicative_fee_is_flat_above_domain() -> Result<()> {
  // Same edge fee at any CR above the domain. Collateral-out does not
  // depend on total SOL, so the outputs are equal.
  let at_3 = with_lst_cr(load_state()?, CR_ABOVE_DOMAIN)?;
  let at_6 = with_lst_cr(load_state()?, UFix64::new(6_000_000_000))?;
  assert_eq!(
    at_3
      .indicative_output::<HYUSD, JITOSOL>(REFERENCE)?
      .out_amount,
    at_6
      .indicative_output::<HYUSD, JITOSOL>(REFERENCE)?
      .out_amount
  );
  Ok(())
}

#[test]
fn exo_redeem_indicative_equals_ungated_in_domain() -> Result<()> {
  let mut state = load_state()?;
  with_exo_cr(&mut state.cbbtc_pair, CR_IN_DOMAIN)?;
  assert_eq!(
    state.indicative_output::<HYUSD, CBBTC>(REFERENCE)?,
    TokenOperation::<HYUSD, CBBTC>::compute_output_ungated(&state, REFERENCE)?
  );
  Ok(())
}

#[test]
fn exo_redeem_indicative_prices_above_domain() -> Result<()> {
  let mut state = load_state()?;
  with_exo_cr(&mut state.cbbtc_pair, CR_ABOVE_DOMAIN)?;
  with_exo_cr(&mut state.hype_pair, CR_ABOVE_DOMAIN)?;
  assert_eq!(
    TokenOperation::<HYUSD, CBBTC>::compute_output_ungated(&state, REFERENCE),
    Err(CoreError::NoValidStablecoinRedeemFee)
  );
  let cbbtc = state.indicative_output::<HYUSD, CBBTC>(REFERENCE)?;
  let hype = state.indicative_output::<HYUSD, HYPE>(REFERENCE)?;
  assert!(cbbtc.out_amount > UFix64::zero());
  assert!(hype.out_amount > UFix64::zero());
  assert!(cbbtc.marginal_rate > 0.0);
  Ok(())
}

#[test]
fn exo_pairs_clamp_independently() -> Result<()> {
  let mut state = load_state()?;
  with_exo_cr(&mut state.cbbtc_pair, CR_IN_DOMAIN)?;
  with_exo_cr(&mut state.hype_pair, CR_ABOVE_DOMAIN)?;
  assert!(TokenOperation::<HYUSD, CBBTC>::compute_output_ungated(
    &state, REFERENCE
  )
  .is_ok());
  assert!(TokenOperation::<HYUSD, HYPE>::compute_output_ungated(
    &state, REFERENCE
  )
  .is_err());
  assert!(state.indicative_output::<HYUSD, HYPE>(REFERENCE).is_ok());
  Ok(())
}
