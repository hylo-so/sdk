use crate::error::CoreError::{
  LeverToStable, LstToToken, StableToLever, TokenToLst,
};
use crate::pyth::PriceRange;

use anchor_lang::prelude::*;
use fix::prelude::*;

/// Provides conversions between an LST and protocol tokens.
pub struct Conversion {
  pub usd_sol_price: PriceRange<N8>,
  pub lst_sol_price: UFix64<N9>,
}

impl Conversion {
  #[must_use]
  pub fn new(usd_sol_price: PriceRange<N8>, lst_sol_price: UFix64<N9>) -> Self {
    Conversion {
      usd_sol_price,
      lst_sol_price,
    }
  }

  /// Computes how much of a protocol token to emit for an input amount of SOL.
  ///   `LST * (SOL/LST) * (USD/SOL) / NAV`
  pub fn lst_to_token(
    &self,
    amount_lst: UFix64<N9>,
    token_nav: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    amount_lst
      .mul_div_floor(self.lst_sol_price, UFix64::one())
      .and_then(|sol| {
        sol.mul_div_floor(self.usd_sol_price.lower, token_nav.convert::<N8>())
      })
      .map(UFix64::convert)
      .ok_or(LstToToken.into())
  }

  /// Finds the conversion amount between a protocol tokens and an LST.
  ///   `TOKEN * NAV / ((USD/SOL) * (SOL/LST))`
  pub fn token_to_lst(
    &self,
    amount_token: UFix64<N6>,
    token_nav: UFix64<N6>,
  ) -> Result<UFix64<N9>> {
    amount_token
      .convert::<N9>()
      .mul_div_floor(token_nav.convert::<N8>(), self.usd_sol_price.upper)
      .and_then(|sol| sol.mul_div_floor(UFix64::one(), self.lst_sol_price))
      .map(UFix64::convert)
      .ok_or(TokenToLst.into())
  }
}

/// Conversions between the protocol's tokens.
pub struct SwapConversion {
  pub stablecoin_nav: UFix64<N6>,
  pub levercoin_nav: PriceRange<N6>,
}

impl SwapConversion {
  #[must_use]
  pub fn new(
    stablecoin_nav: UFix64<N6>,
    levercoin_nav: PriceRange<N6>,
  ) -> Self {
    SwapConversion {
      stablecoin_nav,
      levercoin_nav,
    }
  }

  pub fn stable_to_lever(
    &self,
    amount_stable: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    amount_stable
      .mul_div_floor(self.stablecoin_nav, UFix64::one())
      .and_then(|usd| {
        usd.mul_div_floor(UFix64::one(), self.levercoin_nav.upper)
      })
      .ok_or(StableToLever.into())
  }

  pub fn lever_to_stable(
    &self,
    amount_lever: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    amount_lever
      .mul_div_floor(self.levercoin_nav.lower, UFix64::one())
      .and_then(|usd| usd.mul_div_floor(UFix64::one(), self.stablecoin_nav))
      .ok_or(LeverToStable.into())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::eq_tolerance;
  use crate::util::proptest::*;
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn lst_to_stablecoin_roundtrip(
      state in protocol_state(()),
      lst_sol_price in lst_sol_price(),
      lst_amount in lst_amount(),
    ) {
      let usd_sol_price = PriceRange::one(state.usd_sol_price);
      let conversion = Conversion::new(usd_sol_price, lst_sol_price);
      let amount_token = conversion.lst_to_token(lst_amount, state.stablecoin_nav)?;
      let back_amount_lst = conversion.token_to_lst(amount_token, state.stablecoin_nav)?;
      // Checks converted values are within tolerance of 0.000001 LST
      prop_assert!(
        eq_tolerance!(lst_amount, back_amount_lst, N9, UFix64::new(1000))
      );
    }

    #[test]
    fn lst_to_levercoin_roundtrip(
      state in protocol_state(()),
      lst_sol_price in lst_sol_price(),
      lst_amount in lst_amount(),
    ) {
      let usd_sol_price = PriceRange::one(state.usd_sol_price);
      let conversion = Conversion::new(usd_sol_price, lst_sol_price);
      let amount_token = conversion.lst_to_token(lst_amount, state.levercoin_nav)?;
      let back_amount_lst = conversion.token_to_lst(amount_token, state.levercoin_nav)?;
      // Checks converted values are within tolerance of 0.0001 LST
      // Inherently lossier considering small levercoin NAVs
      prop_assert!(
        eq_tolerance!(lst_amount, back_amount_lst, N9, UFix64::new(100_000))
      );
    }

    #[test]
    fn stablecoin_to_lst_roundtrip(
      state in protocol_state(()),
      lst_sol_price in lst_sol_price(),
    ) {
      let usd_sol_price = PriceRange::one(state.usd_sol_price);
      let conversion = Conversion::new(usd_sol_price, lst_sol_price);
      let amount_lst = conversion.token_to_lst(state.stablecoin_amount, state.stablecoin_nav)?;
      let back_amount_token = conversion.lst_to_token(amount_lst, state.stablecoin_nav)?;
      // Checks converted values are within tolerance of $0.001
      prop_assert!(
        eq_tolerance!(state.stablecoin_amount, back_amount_token, N6, UFix64::new(1000))
      );
    }

    #[test]
    fn levercoin_to_lst_roundtrip(
      state in protocol_state(()),
      lst_sol_price in lst_sol_price(),
    ) {
      let usd_sol_price = PriceRange::one(state.usd_sol_price);
      let conversion = Conversion::new(usd_sol_price, lst_sol_price);
      let amount_lst = conversion.token_to_lst(state.levercoin_amount, state.levercoin_nav)?;
      let back_amount_levercoin = conversion.lst_to_token(amount_lst, state.levercoin_nav)?;
      // Checks converted values are within tolerance of $0.001
      prop_assert!(
        eq_tolerance!(state.levercoin_amount, back_amount_levercoin, N6, UFix64::new(1000))
      );
    }
  }

  #[test]
  fn amount_to_mint_lever() -> Result<()> {
    let usd_sol_price = PriceRange::one(UFix64::<N8>::new(17_103_000_000));
    let lst_sol = UFix64::<N9>::new(1_736_835_834);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let amount_in = UFix64::<N9>::new(50_123_303_006);
    let nav = UFix64::<N6>::new(100_232_580);
    let out = conversion.lst_to_token(amount_in, nav)?;
    assert_eq!(UFix64::new(148_546_300), out);
    Ok(())
  }

  #[test]
  fn amount_to_mint_stable() -> Result<()> {
    let usd_sol_price = PriceRange::one(UFix64::<N8>::new(17_103_000_000));
    let lst_sol = UFix64::<N9>::new(1_736_835_834);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let amount_in = UFix64::<N9>::new(568);
    let out = conversion.lst_to_token(amount_in, UFix64::one())?;
    assert_eq!(UFix64::new(168), out);
    Ok(())
  }

  #[test]
  fn amount_to_redeem_stable() -> Result<()> {
    let usd_sol_price = PriceRange::one(UFix64::<N8>::new(17_103_000_000));
    let lst_sol = UFix64::<N9>::new(1_110_462_847);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let amount = UFix64::<N6>::new(9_937_412_179);
    let lst_out: UFix64<N9> = conversion.token_to_lst(amount, UFix64::one())?;
    assert_eq!(UFix64::new(52_323_522_668), lst_out);
    Ok(())
  }

  #[test]
  fn amount_to_redeem_lever() -> Result<()> {
    let usd_sol_price = PriceRange::one(UFix64::<N8>::new(17_103_000_000));
    let lst_sol = UFix64::<N9>::new(1_110_462_847);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let nav = UFix64::<N6>::new(137_992_981);
    let amount = UFix64::<N6>::new(543_150_099);
    let lst_out: UFix64<N9> = conversion.token_to_lst(amount, nav)?;
    assert_eq!(UFix64::new(394_639_480_798), lst_out);
    Ok(())
  }

  proptest! {
    #[test]
    fn stable_lever_roundtrip(
      stablecoin_nav in stablecoin_nav(),
      levercoin_nav in levercoin_nav(),
      amount_stable in token_amount(),
    ) {
      let conversion = SwapConversion::new(stablecoin_nav, PriceRange::one(levercoin_nav));
      let amount_lever = conversion.stable_to_lever(amount_stable)?;
      let amount_stable_out = conversion.lever_to_stable(amount_lever)?;

      // Checks converted values are within tolerance of 0.01 USD
      prop_assert!(
        eq_tolerance!(amount_stable, amount_stable_out, N6, UFix64::new(10000))
      );
    }

    #[test]
    fn lever_stable_roundtrip(
      stablecoin_nav in stablecoin_nav(),
      levercoin_nav in levercoin_nav(),
      amount_lever in token_amount(),
    ) {
      let conversion = SwapConversion::new(stablecoin_nav, PriceRange::one(levercoin_nav));
      let amount_stable = conversion.lever_to_stable(amount_lever)?;
      let amount_lever_out = conversion.stable_to_lever(amount_stable)?;

      // Checks converted values are within tolerance of 0.01 USD
      prop_assert!(
        eq_tolerance!(amount_lever, amount_lever_out, N6, UFix64::new(10000))
      );
    }
  }
}
