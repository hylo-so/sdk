use fix::prelude::*;

use crate::error::CoreError;
use crate::error::CoreError::{
  ExoCollateralToUsdc, ExoFromToken, ExoToToken, ExoUsdcToCollateral,
  LeverToStable, LstToToken, LstToUsdc, StableToLever, TokenToLst, UsdcToLst,
};
use crate::pyth::PriceRange;
#[cfg(any(test, feature = "offchain"))]
use crate::util::max_scaled_input;

/// Inverse of the `N9` to `N6` truncation under a cap.
#[cfg(any(test, feature = "offchain"))]
fn max_before_truncation(cap: UFix64<N6>) -> Option<UFix64<N9>> {
  cap
    .checked_add(&UFix64::new(1))
    .and_then(UFix64::checked_convert::<N9>)
    .and_then(|bound| bound.checked_sub(&UFix64::new(1)))
}

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
  ) -> Result<UFix64<N6>, CoreError> {
    self
      .lst_to_token_inner(amount_lst, token_nav)
      .ok_or(LstToToken)
  }

  fn lst_to_token_inner(
    &self,
    amount_lst: UFix64<N9>,
    token_nav: UFix64<N9>,
  ) -> Option<UFix64<N6>> {
    (token_nav != UFix64::zero())
      .then_some(amount_lst)
      .and_then(|amt| amt.mul_div_floor(self.lst_sol_price, UFix64::one()))
      .and_then(|sol| sol.mul_div_floor(self.usd_sol_price.lower, token_nav))
      .map(UFix64::convert)
  }

  /// Inverse of [`lst_to_token`](Self::lst_to_token) under a token cap.
  ///
  /// # Errors
  /// * Degenerate NAV or price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_lst_for_token(
    &self,
    cap: UFix64<N6>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    let unconverted = max_before_truncation(cap).ok_or(LstToToken)?;
    let sol =
      max_scaled_input(unconverted, self.usd_sol_price.lower, token_nav)
        .ok_or(LstToToken)?;
    max_scaled_input(sol, self.lst_sol_price, UFix64::one()).ok_or(LstToToken)
  }

  /// Overflow frontier of [`lst_to_token`](Self::lst_to_token).
  ///
  /// # Errors
  /// * Degenerate NAV or price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_representable_lst(
    &self,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    let sol = UFix64::new(u64::MAX)
      .mul_div_floor(token_nav, self.usd_sol_price.lower)
      .ok_or(LstToToken)?;
    max_scaled_input(sol, self.lst_sol_price, UFix64::one()).ok_or(LstToToken)
  }

  /// Finds the conversion amount between a protocol tokens and an LST.
  ///   `TOKEN * NAV / ((USD/SOL) * (SOL/LST))`
  pub fn token_to_lst(
    &self,
    amount_token: UFix64<N6>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    self
      .token_to_lst_inner(amount_token, token_nav)
      .ok_or(TokenToLst)
  }

  fn token_to_lst_inner(
    &self,
    amount_token: UFix64<N6>,
    token_nav: UFix64<N9>,
  ) -> Option<UFix64<N9>> {
    (self.usd_sol_price.upper != UFix64::zero()
      && self.lst_sol_price != UFix64::zero())
    .then_some(amount_token.convert::<N9>())
    .and_then(|amt| amt.mul_div_floor(token_nav, self.usd_sol_price.upper))
    .and_then(|sol| sol.mul_div_floor(UFix64::one(), self.lst_sol_price))
  }

  /// Inverse of [`token_to_lst`](Self::token_to_lst) under an LST cap.
  ///
  /// # Errors
  /// * Degenerate NAV
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_token_for_lst(
    &self,
    cap: UFix64<N9>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N6>, CoreError> {
    let sol = max_scaled_input(cap, UFix64::one(), self.lst_sol_price)
      .ok_or(TokenToLst)?;
    let unconverted =
      max_scaled_input(sol, token_nav, self.usd_sol_price.upper)
        .ok_or(TokenToLst)?;
    Ok(unconverted.convert::<N6>())
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
  ) -> Result<UFix64<N6>, CoreError> {
    SwapConversion::stable_to_lever_inner(
      amount_stable,
      self.stablecoin_nav,
      self.levercoin_nav.upper,
    )
    .ok_or(StableToLever)
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

  /// Inverse of [`stable_to_lever`](Self::stable_to_lever) under a
  /// levercoin cap.
  ///
  /// # Errors
  /// * Degenerate NAV
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_stable_for_lever(
    &self,
    cap: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let usd = max_scaled_input(cap, UFix64::one(), self.levercoin_nav.upper)
      .ok_or(StableToLever)?;
    max_scaled_input(usd, self.stablecoin_nav, UFix64::one())
      .ok_or(StableToLever)
  }

  pub fn lever_to_stable(
    &self,
    amount_lever: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    SwapConversion::lever_to_stable_inner(
      amount_lever,
      self.levercoin_nav.lower,
      self.stablecoin_nav,
    )
    .ok_or(LeverToStable)
  }

  /// Inverse of [`lever_to_stable`](Self::lever_to_stable) under a
  /// stablecoin cap.
  ///
  /// # Errors
  /// * Degenerate NAV
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_lever_for_stable(
    &self,
    cap: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let usd = max_scaled_input(cap, UFix64::one(), self.stablecoin_nav)
      .ok_or(LeverToStable)?;
    max_scaled_input(usd, self.levercoin_nav.lower, UFix64::one())
      .ok_or(LeverToStable)
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
  ) -> Result<UFix64<N6>, CoreError> {
    self.exo_to_token_inner(amount, token_nav).ok_or(ExoToToken)
  }

  fn exo_to_token_inner(
    &self,
    amount: UFix64<N9>,
    token_nav: UFix64<N9>,
  ) -> Option<UFix64<N6>> {
    (token_nav != UFix64::zero())
      .then_some(amount)
      .and_then(|amt| {
        amt.mul_div_floor(self.collateral_usd_price.lower, token_nav)
      })
      .and_then(UFix64::checked_convert::<N6>)
  }

  /// Inverse of [`exo_to_token`](Self::exo_to_token) under a token cap.
  ///
  /// # Errors
  /// * Degenerate price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_exo_for_token(
    &self,
    cap: UFix64<N6>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    let unconverted = max_before_truncation(cap).ok_or(ExoToToken)?;
    max_scaled_input(unconverted, self.collateral_usd_price.lower, token_nav)
      .ok_or(ExoToToken)
  }

  /// Inverse of [`token_to_exo`](Self::token_to_exo) under a
  /// collateral cap.
  ///
  /// # Errors
  /// * Degenerate NAV
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_token_for_exo(
    &self,
    cap: UFix64<N9>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N6>, CoreError> {
    let unconverted =
      max_scaled_input(cap, token_nav, self.collateral_usd_price.upper)
        .ok_or(ExoFromToken)?;
    Ok(unconverted.convert::<N6>())
  }

  /// Converts a protocol token amount to exogenous collateral.
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn token_to_exo(
    &self,
    amount: UFix64<N6>,
    token_nav: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    self
      .token_to_exo_inner(amount, token_nav)
      .ok_or(ExoFromToken)
  }

  fn token_to_exo_inner(
    &self,
    amount: UFix64<N6>,
    token_nav: UFix64<N9>,
  ) -> Option<UFix64<N9>> {
    (self.collateral_usd_price.upper != UFix64::zero())
      .then_some(amount)
      .and_then(UFix64::checked_convert::<N9>)
      .and_then(|a| a.mul_div_floor(token_nav, self.collateral_usd_price.upper))
  }
}

/// Directional conversion between USDC and stablecoin amounts.
pub struct UsdcStablecoinConversion {
  pub usdc_usd_price: PriceRange<N9>,
}

impl UsdcStablecoinConversion {
  #[must_use]
  pub fn new(usdc_usd_price: PriceRange<N9>) -> UsdcStablecoinConversion {
    UsdcStablecoinConversion { usdc_usd_price }
  }

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
  ) -> Result<UFix64<N6>, CoreError> {
    usdc_amount
      .mul_div_floor(self.usdc_usd_price.lower, UFix64::one())
      .and_then(UFix64::checked_convert)
      .ok_or(ExoToToken)
  }

  /// Stablecoin to USDC withdrawal amount using upper bound.
  /// Used on when user redeems stablecoin to USDC.
  ///
  /// # Errors
  /// * Arithmetic overflow or precision conversion
  pub fn stablecoin_to_withdrawal(
    &self,
    stablecoin_amount: UFix64<N6>,
  ) -> Result<UFix64<N9>, CoreError> {
    (self.usdc_usd_price.upper != UFix64::zero())
      .then_some(stablecoin_amount)
      .and_then(UFix64::checked_convert::<N9>)
      .and_then(|a| a.mul_div_floor(UFix64::one(), self.usdc_usd_price.upper))
      .ok_or(ExoFromToken)
  }

  /// Inverse of
  /// [`stablecoin_to_withdrawal`](Self::stablecoin_to_withdrawal) under
  /// a USDC cap.
  ///
  /// # Errors
  /// * Degenerate price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_stablecoin_for_withdrawal(
    &self,
    cap: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let withdrawal = max_before_truncation(cap).ok_or(ExoFromToken)?;
    let normalized =
      max_scaled_input(withdrawal, UFix64::one(), self.usdc_usd_price.upper)
        .ok_or(ExoFromToken)?;
    Ok(normalized.convert::<N6>())
  }

  /// Inverse of [`deposit_to_stablecoin`](Self::deposit_to_stablecoin)
  /// under a stablecoin cap.
  ///
  /// # Errors
  /// * Degenerate price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_deposit_for_stablecoin(
    &self,
    cap: UFix64<N6>,
  ) -> Result<UFix64<N9>, CoreError> {
    let unconverted = max_before_truncation(cap).ok_or(ExoToToken)?;
    max_scaled_input(unconverted, self.usdc_usd_price.lower, UFix64::one())
      .ok_or(ExoToToken)
  }

  /// `N9` normalization frontier for USDC deposits.
  #[cfg(any(test, feature = "offchain"))]
  #[must_use]
  pub fn max_representable_deposit() -> UFix64<N6> {
    UFix64::<N9>::new(u64::MAX).convert()
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
  ) -> Result<UFix64<N6>, CoreError> {
    usdc_amount
      .mul_div_floor(self.usdc_usd_price.upper, UFix64::one())
      .and_then(UFix64::checked_convert)
      .ok_or(ExoToToken)
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
  ) -> Result<UFix64<N9>, CoreError> {
    self
      .collateral_to_usdc_inner(collateral_amount)
      .ok_or(ExoCollateralToUsdc)
  }

  fn collateral_to_usdc_inner(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Option<UFix64<N9>> {
    (self.usdc_usd_price.upper != UFix64::zero())
      .then_some(collateral_amount)
      .and_then(|amt| {
        amt.mul_div_floor(self.collateral_usd_price, self.usdc_usd_price.upper)
      })
  }

  /// Inverse of [`collateral_to_usdc`](Self::collateral_to_usdc) under
  /// a USDC cap.
  ///
  /// # Errors
  /// * Degenerate price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_collateral_for_usdc(
    &self,
    cap: UFix64<N6>,
  ) -> Result<UFix64<N9>, CoreError> {
    let usdc = max_before_truncation(cap).ok_or(ExoCollateralToUsdc)?;
    max_scaled_input(usdc, self.collateral_usd_price, self.usdc_usd_price.upper)
      .ok_or(ExoCollateralToUsdc)
  }

  /// Converts USDC to exogenous collateral
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn usdc_to_collateral(
    &self,
    usdc_amount: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    self
      .usdc_to_collateral_inner(usdc_amount)
      .ok_or(ExoUsdcToCollateral)
  }

  fn usdc_to_collateral_inner(
    &self,
    usdc_amount: UFix64<N9>,
  ) -> Option<UFix64<N9>> {
    (self.collateral_usd_price != UFix64::zero())
      .then_some(usdc_amount)
      .and_then(|amt| {
        amt.mul_div_floor(self.usdc_usd_price.lower, self.collateral_usd_price)
      })
  }

  /// Inverse of [`usdc_to_collateral`](Self::usdc_to_collateral) under a
  /// collateral cap.
  ///
  /// # Errors
  /// * Degenerate price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_usdc_for_collateral(
    &self,
    cap: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    max_scaled_input(cap, self.usdc_usd_price.lower, self.collateral_usd_price)
      .ok_or(ExoUsdcToCollateral)
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
  pub fn lst_to_usdc(
    &self,
    lst_amount: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    (self.usdc_usd.upper != UFix64::zero())
      .then_some(lst_amount)
      .and_then(|amt| amt.mul_div_floor(self.lst_sol, UFix64::one()))
      .and_then(|sol| sol.mul_div_floor(self.sol_usd, self.usdc_usd.upper))
      .ok_or(LstToUsdc)
  }

  /// Inverse of [`lst_to_usdc`](Self::lst_to_usdc) under a USDC cap.
  ///
  /// # Errors
  /// * Degenerate price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_lst_for_usdc(
    &self,
    cap: UFix64<N6>,
  ) -> Result<UFix64<N9>, CoreError> {
    let usdc = max_before_truncation(cap).ok_or(LstToUsdc)?;
    let sol = max_scaled_input(usdc, self.sol_usd, self.usdc_usd.upper)
      .ok_or(LstToUsdc)?;
    max_scaled_input(sol, self.lst_sol, UFix64::one()).ok_or(LstToUsdc)
  }

  /// Converts USDC to LST for buy-side rebalancing.
  ///
  /// # Errors
  /// * Arithmetic failure
  pub fn usdc_to_lst(
    &self,
    usdc_amount: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    (self.sol_usd != UFix64::zero() && self.lst_sol != UFix64::zero())
      .then_some(usdc_amount)
      .and_then(|amt| amt.mul_div_floor(self.usdc_usd.lower, self.sol_usd))
      .and_then(|sol| sol.mul_div_floor(UFix64::one(), self.lst_sol))
      .ok_or(UsdcToLst)
  }

  /// Inverse of [`usdc_to_lst`](Self::usdc_to_lst) under an LST cap.
  ///
  /// # Errors
  /// * Degenerate price
  #[cfg(any(test, feature = "offchain"))]
  pub fn max_usdc_for_lst(
    &self,
    cap: UFix64<N9>,
  ) -> Result<UFix64<N9>, CoreError> {
    let sol =
      max_scaled_input(cap, UFix64::one(), self.lst_sol).ok_or(UsdcToLst)?;
    max_scaled_input(sol, self.usdc_usd.lower, self.sol_usd).ok_or(UsdcToLst)
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
  fn amount_to_mint_lever() -> Result<(), CoreError> {
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
  fn amount_to_mint_stable() -> Result<(), CoreError> {
    let usd_sol_price = PriceRange::one(UFix64::<N9>::new(171_030_000_000));
    let lst_sol = UFix64::<N9>::new(1_736_835_834);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let amount_in = UFix64::<N9>::new(568);
    let out = conversion.lst_to_token(amount_in, UFix64::one())?;
    assert_eq!(UFix64::new(168), out);
    Ok(())
  }

  #[test]
  fn amount_to_redeem_stable() -> Result<(), CoreError> {
    let usd_sol_price = PriceRange::one(UFix64::<N9>::new(171_030_000_000));
    let lst_sol = UFix64::<N9>::new(1_110_462_847);
    let conversion = Conversion::new(usd_sol_price, lst_sol);
    let amount = UFix64::<N6>::new(9_937_412_179);
    let lst_out: UFix64<N9> = conversion.token_to_lst(amount, UFix64::one())?;
    assert_eq!(UFix64::new(52_323_522_668), lst_out);
    Ok(())
  }

  #[test]
  fn amount_to_redeem_lever() -> Result<(), CoreError> {
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

    /// `usdc_to_lst(lst_to_usdc(x)) <= x`.
    #[test]
    fn lst_rebalance_roundtrip_favors_protocol(
      amount_lst in lst_amount(),
      lst_sol in lst_sol_price(),
      sol_usd in usd_sol_price(),
      usdc_usd in dollar_centered_price_range(),
    ) {
      let conv = LstRebalanceConversion::new(lst_sol, sol_usd, usdc_usd);
      let usdc = conv.lst_to_usdc(amount_lst)?;
      let back = conv.usdc_to_lst(usdc)?;
      prop_assert!(back <= amount_lst);
    }

    /// `lever_to_stable(stable_to_lever(x)) <= x`.
    #[test]
    fn swap_conversion_stable_roundtrip_favors_protocol(
      amount_stable in token_amount(),
      stablecoin_nav in stablecoin_nav(),
      levercoin_nav in dollar_centered_price_range(),
    ) {
      let conv = SwapConversion::new(stablecoin_nav, levercoin_nav);
      let lever = conv.stable_to_lever(amount_stable)?;
      let back = conv.lever_to_stable(lever)?;
      prop_assert!(back <= amount_stable);
    }
  }

  const UNDERPEGGED_USDC: PriceRange<N9> = PriceRange {
    lower: UFix64::constant(997_000_000),
    upper: UFix64::constant(999_000_000),
  };

  const COLLATERAL_PRICE: UFix64<N9> = UFix64::constant(148_370_000_000);

  #[test]
  fn exo_rebalance_collateral_to_usdc() -> Result<(), CoreError> {
    let conv = ExoRebalanceConversion::new(COLLATERAL_PRICE, UNDERPEGGED_USDC);
    let usdc = conv.collateral_to_usdc(UFix64::new(10_000_000_000))?;
    assert_eq!(usdc, UFix64::new(1_485_185_185_185));
    Ok(())
  }

  #[test]
  fn exo_rebalance_usdc_to_collateral() -> Result<(), CoreError> {
    let conv = ExoRebalanceConversion::new(COLLATERAL_PRICE, UNDERPEGGED_USDC);
    let coll = conv.usdc_to_collateral(UFix64::new(1_500_000_000_000))?;
    assert_eq!(coll, UFix64::new(10_079_530_902));
    Ok(())
  }

  const LST_SOL: UFix64<N9> = UFix64::constant(1_136_000_000);
  const SOL_USD: UFix64<N9> = UFix64::constant(171_030_000_000);

  #[test]
  fn lst_rebalance_lst_to_usdc() -> Result<(), CoreError> {
    let conv = LstRebalanceConversion::new(LST_SOL, SOL_USD, UNDERPEGGED_USDC);
    let usdc = conv.lst_to_usdc(UFix64::new(10_000_000_000))?;
    assert_eq!(usdc, UFix64::new(1_944_845_645_645));
    Ok(())
  }

  #[test]
  fn lst_rebalance_usdc_to_lst() -> Result<(), CoreError> {
    let conv = LstRebalanceConversion::new(LST_SOL, SOL_USD, UNDERPEGGED_USDC);
    let lst = conv.usdc_to_lst(UFix64::new(200_000_000_000))?;
    assert_eq!(lst, UFix64::new(1_026_300_467));
    Ok(())
  }
}

#[cfg(kani)]
mod proofs {
  use fix::prelude::*;

  use crate::conversion::{Conversion, ExoConversion, ExoRebalanceConversion};
  use crate::kani_generators::{
    dollar_centered_price_range, narrow_price_range, narrow_ufix64,
  };

  /// `token_to_lst(lst_to_token(x)) <= x`.
  #[kani::proof]
  fn lst_conversion_roundtrip_favors_protocol() {
    let amount_lst: UFix64<N9> = narrow_ufix64();
    let token_nav: UFix64<N9> = narrow_ufix64();
    let lst_sol_price: UFix64<N9> = narrow_ufix64();
    let back = narrow_price_range::<N9>().and_then(|usd_sol_price| {
      let conv = Conversion::new(usd_sol_price, lst_sol_price);
      conv
        .lst_to_token_inner(amount_lst, token_nav)
        .and_then(|t| conv.token_to_lst_inner(t, token_nav))
    });
    assert!(back.is_none_or(|b| b <= amount_lst));
  }

  /// `token_to_exo(exo_to_token(x)) <= x`.
  #[kani::proof]
  fn exo_conversion_roundtrip_favors_protocol() {
    let amount: UFix64<N9> = narrow_ufix64();
    let token_nav = UFix64::<N9>::one();
    let back = narrow_price_range::<N9>().and_then(|collateral_usd_price| {
      let conv = ExoConversion::new(collateral_usd_price);
      conv
        .exo_to_token_inner(amount, token_nav)
        .and_then(|t| conv.token_to_exo_inner(t, token_nav))
    });
    assert!(back.is_none_or(|b| b <= amount));
  }

  /// `usdc_to_collateral(collateral_to_usdc(x)) <= x`.
  #[kani::proof]
  fn exo_rebalance_roundtrip_favors_protocol() {
    let amount: UFix64<N9> = narrow_ufix64();
    let collateral_usd_price = UFix64::<N9>::one();
    let back = dollar_centered_price_range().and_then(|usdc_usd_price| {
      let conv =
        ExoRebalanceConversion::new(collateral_usd_price, usdc_usd_price);
      conv
        .collateral_to_usdc_inner(amount)
        .and_then(|u| conv.usdc_to_collateral_inner(u))
    });
    assert!(back.is_none_or(|b| b <= amount));
  }
}
