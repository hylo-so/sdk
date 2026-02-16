use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fix::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use super::{validate_stability_thresholds, ExchangeContext};
use crate::conversion::Conversion;
use crate::error::CoreError::{
  DestinationFeeSol, DestinationFeeStablecoin, LevercoinNav,
  NoNextStabilityThreshold,
};
use crate::exchange_math::{collateral_ratio, max_swappable_stablecoin};
use crate::fee_controller::{FeeController, FeeExtract, LevercoinFees};
use crate::fee_curves::{mint_fee_curve, redeem_fee_curve};
use crate::interpolated_fees::{
  InterpolatedFeeController, InterpolatedMintFees, InterpolatedRedeemFees,
};
use crate::lst_sol_price::LstSolPrice;
use crate::pyth::{query_pyth_price, OracleConfig, PriceRange};
use crate::solana_clock::SolanaClock;
use crate::stability_mode::{StabilityController, StabilityMode};
use crate::total_sol_cache::TotalSolCache;
use crate::virtual_stablecoin::VirtualStablecoin;

/// Exchange context for SOL/LST collateral pairs.
#[derive(Clone)]
pub struct LstExchangeContext<C> {
  pub clock: C,
  pub total_sol: UFix64<N9>,
  pub sol_usd_price: PriceRange<N8>,
  virtual_stablecoin: VirtualStablecoin,
  levercoin_supply: Option<UFix64<N6>>,
  collateral_ratio: UFix64<N9>,
  pub stability_controller: StabilityController,
  stability_mode: StabilityMode,
  stablecoin_mint_fees: InterpolatedMintFees,
  stablecoin_redeem_fees: InterpolatedRedeemFees,
  levercoin_fees: LevercoinFees,
}

impl<C: SolanaClock> ExchangeContext for LstExchangeContext<C> {
  fn total_collateral(&self) -> UFix64<N9> {
    self.total_sol
  }

  fn collateral_usd_price(&self) -> PriceRange<N8> {
    self.sol_usd_price
  }

  fn virtual_stablecoin_supply(&self) -> Result<UFix64<N6>> {
    self.virtual_stablecoin.supply()
  }

  fn levercoin_supply(&self) -> Result<UFix64<N6>> {
    self.levercoin_supply.ok_or(LevercoinNav.into())
  }

  fn stability_controller(&self) -> &StabilityController {
    &self.stability_controller
  }

  fn stability_mode(&self) -> StabilityMode {
    self.stability_mode
  }

  fn collateral_ratio(&self) -> UFix64<N9> {
    self.collateral_ratio
  }

  fn levercoin_fees(&self) -> &LevercoinFees {
    &self.levercoin_fees
  }
}

impl<C: SolanaClock> LstExchangeContext<C> {
  /// Creates context for LST exchange operations from account data.
  ///
  /// # Errors
  /// * Oracle, cache, curve, or stability controller validation
  #[allow(clippy::too_many_arguments)]
  pub fn load(
    clock: C,
    total_sol_cache: &TotalSolCache,
    stability_threshold_1: UFix64<N2>,
    oracle_config: OracleConfig<N8>,
    levercoin_fees: LevercoinFees,
    sol_usd_pyth_feed: &PriceUpdateV2,
    virtual_stablecoin: VirtualStablecoin,
    levercoin_mint: Option<&Mint>,
  ) -> Result<LstExchangeContext<C>> {
    let total_sol = total_sol_cache.get_validated(clock.epoch())?;
    let sol_usd_price =
      query_pyth_price(&clock, sol_usd_pyth_feed, oracle_config)?;
    let stablecoin_mint_fees = InterpolatedMintFees::new(mint_fee_curve()?);
    let stablecoin_redeem_fees =
      InterpolatedRedeemFees::new(redeem_fee_curve()?);
    let stability_threshold_2 = stablecoin_redeem_fees.cr_floor()?;
    validate_stability_thresholds(
      stability_threshold_1,
      stability_threshold_2,
    )?;
    let stability_controller =
      StabilityController::new(stability_threshold_1, stability_threshold_2)?;
    let stablecoin_supply = virtual_stablecoin.supply()?;
    let levercoin_supply = levercoin_mint.map(|m| UFix64::new(m.supply));
    let collateral_ratio =
      collateral_ratio(total_sol, sol_usd_price.lower, stablecoin_supply)?;
    let stability_mode =
      stability_controller.stability_mode(collateral_ratio)?;
    Ok(LstExchangeContext {
      clock,
      total_sol,
      sol_usd_price,
      virtual_stablecoin,
      levercoin_supply,
      collateral_ratio,
      stability_controller,
      stability_mode,
      stablecoin_mint_fees,
      stablecoin_redeem_fees,
      levercoin_fees,
    })
  }

  /// Stablecoin mint fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection overflow, interpolation, or fee extraction
  pub fn stablecoin_mint_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let new_sol = lst_sol_price.convert_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationFeeSol)?;
    let new_total_stablecoin = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst, self.stablecoin_nav()?)?
      .checked_add(&self.virtual_stablecoin_supply()?)
      .ok_or(DestinationFeeStablecoin)?;
    let projected_cr = collateral_ratio(
      new_total_sol,
      self.sol_usd_price.lower,
      new_total_stablecoin,
    )?;
    self
      .stablecoin_mint_fees
      .apply_fee(projected_cr, amount_lst)
  }

  /// Stablecoin redeem fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection underflow, interpolation, or fee extraction
  pub fn stablecoin_redeem_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let sol_rm = lst_sol_price.convert_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationFeeSol)?;
    let stablecoin_redeemed = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst, self.stablecoin_nav()?)?;
    let new_total_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_redeemed)
      .ok_or(DestinationFeeStablecoin)?;
    let projected_cr = collateral_ratio(
      new_total_sol,
      self.sol_usd_price.lower,
      new_total_stablecoin,
    )?;
    self
      .stablecoin_redeem_fees
      .apply_fee(projected_cr, amount_lst)
  }

  /// Levercoin mint fee based on projected stability mode.
  ///
  /// # Errors
  /// * Projection overflow or fee lookup
  pub fn levercoin_mint_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let new_sol = lst_sol_price.convert_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationFeeSol)?;

    let stability_mode_for_fees = {
      let projected = self.projected_stability_mode(
        new_total_sol,
        self.virtual_stablecoin_supply()?,
      )?;
      self.select_stability_mode_for_fees(projected)
    };

    self
      .levercoin_fees
      .mint_fee(stability_mode_for_fees)
      .and_then(|fee| FeeExtract::new(fee, amount_lst))
  }

  /// Levercoin redeem fee based on projected stability mode.
  ///
  /// # Errors
  /// * Projection underflow or fee lookup
  pub fn levercoin_redeem_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let sol_rm = lst_sol_price.convert_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationFeeSol)?;

    let stability_mode_for_fees = {
      let projected = self.projected_stability_mode(
        new_total_sol,
        self.virtual_stablecoin_supply()?,
      )?;
      self.select_stability_mode_for_fees(projected)
    };

    self
      .levercoin_fees
      .redeem_fee(stability_mode_for_fees)
      .and_then(|fee| FeeExtract::new(fee, amount_lst))
  }

  /// LST/SOL token conversion helper.
  ///
  /// # Errors
  /// * Epoch price lookup failure
  pub fn token_conversion(
    &self,
    lst_sol_price: &LstSolPrice,
  ) -> Result<Conversion> {
    let lst_sol = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    Ok(Conversion::new(self.sol_usd_price, lst_sol))
  }

  /// Special case conversion from raw SOL to stablecoin.
  /// Reuses LST/SOL converter with a 1:1 base conversion.
  ///
  /// # Errors
  /// * NAV or arithmetic failure
  pub fn sol_to_stablecoin(
    &self,
    amount_sol: UFix64<N9>,
  ) -> Result<UFix64<N6>> {
    let nav = self.stablecoin_nav()?;
    let conversion = Conversion::new(self.sol_usd_price, UFix64::one());
    conversion.lst_to_token(amount_sol, nav)
  }

  /// Special case conversion from raw SOL to levercoin.
  ///
  /// # Errors
  /// * NAV or arithmetic failure
  pub fn sol_to_levercoin(&self, amount_sol: UFix64<N9>) -> Result<UFix64<N6>> {
    let nav = self.levercoin_mint_nav()?;
    let conversion = Conversion::new(self.sol_usd_price, UFix64::one());
    conversion.lst_to_token(amount_sol, nav)
  }

  /// Maximum stablecoin swappable from levercoin using the next
  /// lowest CR threshold as the limit.
  ///
  /// # Errors
  /// * No next stability threshold or arithmetic failure
  pub fn max_swappable_stablecoin_to_next_threshold(
    &self,
  ) -> Result<UFix64<N6>> {
    let total_value_locked = self.total_value_locked()?;
    let next_stability_threshold = self
      .stability_controller
      .next_stability_threshold(self.stability_mode)
      .ok_or(NoNextStabilityThreshold)?;
    max_swappable_stablecoin(
      next_stability_threshold,
      total_value_locked,
      self.virtual_stablecoin_supply()?,
    )
  }
}
