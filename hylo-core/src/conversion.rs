use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::{
  ExoCollateralToUsdc, ExoFromToken, ExoToToken, ExoUsdcToCollateral,
  LeverToStable, LstToToken, LstToUsdc, StableToLever, TokenToLst, UsdcToLst,
};
use crate::pyth::PriceRange;

/// Provides conversions between an LST and protocol tokens.
pub struct Conversion {
  pub usd_sol_price: PriceRange<N9>,
  pub lst_sol_price: UFix64<N9>,
}

impl Conversion {
  #[must_use]
  pub fn new(usd_sol_price: PriceRange<N9>, lst_sol_price: UFix64<N9>) -> Self {
    Conversion {
      usd_sol_price,
      lst_sol_price,
    }
  }

  #[must_use]
  pub fn spot(
    usd_sol_price: UFix64<N9>,
    lst_sol_price: UFix64<N9>,
  ) -> Conversion {
    Conversion::new(PriceRange::one(usd_sol_price), lst_sol_price)
  }

  /// Computes how much of a protocol token to emit for an input amount of SOL.
  ///   `LST * (SOL/LST) * (USD/SOL) / NAV`
  pub fn lst_to_token(
    &self,
    amount_lst: UFix64<N9>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N6>> {
    (token_nav != UFix64::zero())
      .then_some(amount_lst)
      .and_then(|amt| amt.mul_div_floor(self.lst_sol_price, UFix64::one()))
      .and_then(|sol| sol.mul_div_floor(self.usd_sol_price.lower, token_nav))
      .map(UFix64::convert)
      .ok_or(LstToToken.into())
  }

  /// Finds the conversion amount between a protocol tokens and an LST.
  ///   `TOKEN * NAV / ((USD/SOL) * (SOL/LST))`
  pub fn token_to_lst(
    &self,
    amount_token: UFix64<N6>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N9>> {
    (self.usd_sol_price.upper != UFix64::zero()
      && self.lst_sol_price != UFix64::zero())
    .then_some(amount_token.convert::<N9>())
    .and_then(|amt| amt.mul_div_floor(token_nav, self.usd_sol_price.upper))
    .and_then(|sol| sol.mul_div_floor(UFix64::one(), self.lst_sol_price))
    .ok_or(TokenToLst.into())
  }
}

/// Conversions between the protocol's tokens.
pub struct SwapConversion {
  pub stablecoin_nav: UFix64<N9>,
  pub levercoin_nav: PriceRange<N9>,
}

impl SwapConversion {
  #[must_use]
  pub fn new(
    stablecoin_nav: UFix64<N9>,
    levercoin_nav: PriceRange<N9>,
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
    stable_to_lever_inner(
      amount_stable,
      self.stablecoin_nav,
      self.levercoin_nav.upper,
    )
    .ok_or(StableToLever.into())
  }

  pub fn lever_to_stable(
    &self,
    amount_lever: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    lever_to_stable_inner(
      amount_lever,
      self.levercoin_nav.lower,
      self.stablecoin_nav,
    )
    .ok_or(LeverToStable.into())
  }
}

fn stable_to_lever_inner(
  amount_stable: UFix64<N6>,
  stablecoin_nav: UFix64<N9>,
  levercoin_nav_upper: UFix64<N9>,
) -> Option<UFix64<N6>> {
  (levercoin_nav_upper != UFix64::zero())
    .then_some(amount_stable)
    .and_then(|amt| amt.mul_div_floor(stablecoin_nav, UFix64::one()))
    .and_then(|usd| usd.mul_div_floor(UFix64::one(), levercoin_nav_upper))
}

fn lever_to_stable_inner(
  amount_lever: UFix64<N6>,
  levercoin_nav_lower: UFix64<N9>,
  stablecoin_nav: UFix64<N9>,
) -> Option<UFix64<N6>> {
  (stablecoin_nav != UFix64::zero())
    .then_some(amount_lever)
    .and_then(|amt| amt.mul_div_floor(levercoin_nav_lower, UFix64::one()))
    .and_then(|usd| usd.mul_div_floor(UFix64::one(), stablecoin_nav))
}

/// Conversions between an exogenous collateral and protocol tokens.
pub struct ExoConversion {
  collateral_usd_price: PriceRange<N9>,
}

impl ExoConversion {
  #[must_use]
  pub fn new(collateral_usd_price: PriceRange<N9>) -> ExoConversion {
    ExoConversion {
      collateral_usd_price,
    }
  }

  #[must_use]
  pub fn spot(collateral_usd_price: UFix64<N9>) -> ExoConversion {
    ExoConversion::new(PriceRange::one(collateral_usd_price))
  }

  /// Converts collateral amount to a protocol token amount.
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn exo_to_token(
    &self,
    amount: UFix64<N9>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N6>> {
    (token_nav != UFix64::zero())
      .then_some(amount)
      .and_then(|amt| {
        amt.mul_div_floor(self.collateral_usd_price.lower, token_nav)
      })
      .and_then(UFix64::checked_convert::<N6>)
      .ok_or(ExoToToken.into())
  }

  /// Converts a protocol token amount to exogenous collateral.
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn token_to_exo(
    &self,
    amount: UFix64<N6>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N9>> {
    (self.collateral_usd_price.upper != UFix64::zero())
      .then_some(amount)
      .and_then(UFix64::checked_convert::<N9>)
      .and_then(|a| a.mul_div_floor(token_nav, self.collateral_usd_price.upper))
      .ok_or(ExoFromToken.into())
  }
}

/// Directional conversion between USDC and stablecoin amounts.
pub struct UsdcStablecoinConversion {
  pub usdc_usd_price: PriceRange<N9>,
}

impl UsdcStablecoinConversion {
  /// USDC deposit to stablecoin amount using lower bound.
  ///
  /// Used for USDC to stablecoin swaps and sell-side collateral swaps to
  /// compute virtual stablecoins to mint when USDC enters the vault.
  ///
  /// # Errors
  /// * Arithmetic overflow or precision conversion
  pub fn deposit_to_stablecoin(
    &self,
    usdc_amount: UFix64<N9>,
  ) -> Result<UFix64<N6>> {
    usdc_amount
      .mul_div_floor(self.usdc_usd_price.lower, UFix64::one())
      .and_then(UFix64::checked_convert)
      .ok_or(ExoToToken.into())
  }

  /// Stablecoin to USDC withdrawal amount using upper bound.
  /// Used on when user redeems stablecoin to USDC.
  ///
  /// # Errors
  /// * Arithmetic overflow or precision conversion
  pub fn stablecoin_to_withdrawal(
    &self,
    stablecoin_amount: UFix64<N6>,
  ) -> Result<UFix64<N9>> {
    (self.usdc_usd_price.upper != UFix64::zero())
      .then_some(stablecoin_amount)
      .and_then(UFix64::checked_convert::<N9>)
      .and_then(|a| a.mul_div_floor(UFix64::one(), self.usdc_usd_price.upper))
      .ok_or(ExoFromToken.into())
  }

  /// USDC withdrawal to stablecoin equivalent using upper bound.
  ///
  /// Used on buy-side collateral swaps to compute virtual stablecoins to burn
  /// when USDC leaves the vault.
  ///
  /// # Errors
  /// * Arithmetic overflow or precision conversion
  pub fn withdrawal_to_stablecoin(
    &self,
    usdc_amount: UFix64<N9>,
  ) -> Result<UFix64<N6>> {
    usdc_amount
      .mul_div_floor(self.usdc_usd_price.upper, UFix64::one())
      .and_then(UFix64::checked_convert)
      .ok_or(ExoToToken.into())
  }
}

/// Conversions between exogenous collateral and USDC via oracle prices.
pub struct ExoRebalanceConversion {
  collateral_usd_price: UFix64<N9>,
  usdc_usd_price: PriceRange<N9>,
}

impl ExoRebalanceConversion {
  #[must_use]
  pub fn new(
    collateral_usd_price: UFix64<N9>,
    usdc_usd_price: PriceRange<N9>,
  ) -> ExoRebalanceConversion {
    ExoRebalanceConversion {
      collateral_usd_price,
      usdc_usd_price,
    }
  }

  /// Converts exogenous collateral to USDC
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn collateral_to_usdc(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Result<UFix64<N9>> {
    (self.usdc_usd_price.upper != UFix64::zero())
      .then_some(collateral_amount)
      .and_then(|amt| {
        amt.mul_div_floor(self.collateral_usd_price, self.usdc_usd_price.upper)
      })
      .ok_or(ExoCollateralToUsdc.into())
  }

  /// Converts USDC to exogenous collateral
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn usdc_to_collateral(
    &self,
    usdc_amount: UFix64<N9>,
  ) -> Result<UFix64<N9>> {
    (self.collateral_usd_price != UFix64::zero())
      .then_some(usdc_amount)
      .and_then(|amt| {
        amt.mul_div_floor(self.usdc_usd_price.lower, self.collateral_usd_price)
      })
      .ok_or(ExoUsdcToCollateral.into())
  }
}

/// Conversions between LST and USDC via SOL for rebalancing.
pub struct LstRebalanceConversion {
  lst_sol: UFix64<N9>,
  sol_usd: UFix64<N9>,
  usdc_usd: PriceRange<N9>,
}

impl LstRebalanceConversion {
  #[must_use]
  pub fn new(
    lst_sol: UFix64<N9>,
    sol_usd: UFix64<N9>,
    usdc_usd: PriceRange<N9>,
  ) -> LstRebalanceConversion {
    LstRebalanceConversion {
      lst_sol,
      sol_usd,
      usdc_usd,
    }
  }

  /// Converts LST to USDC for sell-side rebalancing.
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn lst_to_usdc(&self, lst_amount: UFix64<N9>) -> Result<UFix64<N9>> {
    (self.usdc_usd.upper != UFix64::zero())
      .then_some(lst_amount)
      .and_then(|amt| amt.mul_div_floor(self.lst_sol, UFix64::one()))
      .and_then(|sol| sol.mul_div_floor(self.sol_usd, self.usdc_usd.upper))
      .ok_or(LstToUsdc.into())
  }

  /// Converts USDC to LST for buy-side rebalancing.
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn usdc_to_lst(&self, usdc_amount: UFix64<N9>) -> Result<UFix64<N9>> {
    (self.sol_usd != UFix64::zero() && self.lst_sol != UFix64::zero())
      .then_some(usdc_amount)
      .and_then(|amt| amt.mul_div_floor(self.usdc_usd.lower, self.sol_usd))
      .and_then(|sol| sol.mul_div_floor(UFix64::one(), self.lst_sol))
      .ok_or(UsdcToLst.into())
  }
}

#[cfg(kani)]
mod proofs {
  use fix::prelude::*;

  use crate::conversion::{lever_to_stable_inner, stable_to_lever_inner};
  use crate::proofs::{any_ufix64, token_amount};

  #[kani::proof]
  fn stable_to_lever_none_for_zero_levercoin_nav() {
    let amount: UFix64<N6> = token_amount();
    let stablecoin_nav: UFix64<N9> = any_ufix64();
    let levercoin_nav_upper = UFix64::<N9>::zero();
    assert_eq!(
      stable_to_lever_inner(amount, stablecoin_nav, levercoin_nav_upper),
      None
    );
  }

  #[kani::proof]
  fn lever_to_stable_none_for_zero_stablecoin_nav() {
    let amount: UFix64<N6> = token_amount();
    let levercoin_nav_lower: UFix64<N9> = any_ufix64();
    let stablecoin_nav = UFix64::<N9>::zero();
    assert_eq!(
      lever_to_stable_inner(amount, levercoin_nav_lower, stablecoin_nav),
      None
    );
  }
}

#[cfg(test)]
mod tests {
  use proptest::prelude::*;

  use super::*;
  use crate::eq_tolerance;
  use crate::util::proptest::*;

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
    let usd_sol_price = PriceRange::one(UFix64::<N9>::new(171_030_000_000));
    let lst_sol = UFix64::<N9>::new(1_736_835_834);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let amount_in = UFix64::<N9>::new(50_123_303_006);
    let nav = UFix64::<N9>::new(100_232_580_000);
    let out = conversion.lst_to_token(amount_in, nav)?;
    assert_eq!(UFix64::new(148_546_300), out);
    Ok(())
  }

  #[test]
  fn amount_to_mint_stable() -> Result<()> {
    let usd_sol_price = PriceRange::one(UFix64::<N9>::new(171_030_000_000));
    let lst_sol = UFix64::<N9>::new(1_736_835_834);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let amount_in = UFix64::<N9>::new(568);
    let out = conversion.lst_to_token(amount_in, UFix64::one())?;
    assert_eq!(UFix64::new(168), out);
    Ok(())
  }

  #[test]
  fn amount_to_redeem_stable() -> Result<()> {
    let usd_sol_price = PriceRange::one(UFix64::<N9>::new(171_030_000_000));
    let lst_sol = UFix64::<N9>::new(1_110_462_847);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let amount = UFix64::<N6>::new(9_937_412_179);
    let lst_out: UFix64<N9> = conversion.token_to_lst(amount, UFix64::one())?;
    assert_eq!(UFix64::new(52_323_522_668), lst_out);
    Ok(())
  }

  #[test]
  fn amount_to_redeem_lever() -> Result<()> {
    let usd_sol_price = PriceRange::one(UFix64::<N9>::new(171_030_000_000));
    let lst_sol = UFix64::<N9>::new(1_110_462_847);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let nav = UFix64::<N9>::new(137_992_981_000);
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

  const UNDERPEGGED_USDC: PriceRange<N9> = PriceRange {
    lower: UFix64::constant(997_000_000),
    upper: UFix64::constant(999_000_000),
  };

  const COLLATERAL_PRICE: UFix64<N9> = UFix64::constant(148_370_000_000);

  #[test]
  fn exo_rebalance_collateral_to_usdc() -> Result<()> {
    let conv = ExoRebalanceConversion::new(COLLATERAL_PRICE, UNDERPEGGED_USDC);
    let usdc = conv.collateral_to_usdc(UFix64::new(10_000_000_000))?;
    assert_eq!(usdc, UFix64::new(1_485_185_185_185));
    Ok(())
  }

  #[test]
  fn exo_rebalance_usdc_to_collateral() -> Result<()> {
    let conv = ExoRebalanceConversion::new(COLLATERAL_PRICE, UNDERPEGGED_USDC);
    let coll = conv.usdc_to_collateral(UFix64::new(1_500_000_000_000))?;
    assert_eq!(coll, UFix64::new(10_079_530_902));
    Ok(())
  }

  const LST_SOL: UFix64<N9> = UFix64::constant(1_136_000_000);
  const SOL_USD: UFix64<N9> = UFix64::constant(171_030_000_000);

  #[test]
  fn lst_rebalance_lst_to_usdc() -> Result<()> {
    let conv = LstRebalanceConversion::new(LST_SOL, SOL_USD, UNDERPEGGED_USDC);
    let usdc = conv.lst_to_usdc(UFix64::new(10_000_000_000))?;
    assert_eq!(usdc, UFix64::new(1_944_845_645_645));
    Ok(())
  }

  #[test]
  fn lst_rebalance_usdc_to_lst() -> Result<()> {
    let conv = LstRebalanceConversion::new(LST_SOL, SOL_USD, UNDERPEGGED_USDC);
    let lst = conv.usdc_to_lst(UFix64::new(200_000_000_000))?;
    assert_eq!(lst, UFix64::new(1_026_300_467));
    Ok(())
  }
}
