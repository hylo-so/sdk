//! The indicative tier prices a route as if it could execute.

mod common;

use anyhow::Result;
use common::{load_state, with_lst_cr, CR_IN_DOMAIN};
use fix::prelude::*;
use hylo_idl::tokens::{HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};
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
