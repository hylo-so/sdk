use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fix::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use super::ExchangeContext;
use crate::conversion::{Conversion, LstRebalanceConversion};
use crate::error::CoreError::{
  DestinationCollateral, DestinationStablecoin, LevercoinNav,
  RebalanceAmountExceeded,
};
use crate::exchange_math::collateral_ratio;
use crate::fees::controller::{FeeController, FeeExtract, LevercoinFees};
use crate::fees::curve_controller::{
  InterpolatedFeeController, InterpolatedMintFees, InterpolatedRedeemFees,
};
use crate::fees::curves::{mint_fee_curve, redeem_fee_curve};
use crate::lst::sol_price::LstSolPrice;
use crate::lst::total_sol_cache::TotalSolCache;
use crate::pyth::{query_pyth_oracle, OracleConfig, OraclePrice, PriceRange};
use crate::rebalance::mode::RebalanceMode;
use crate::rebalance::pricing::{
  RebalanceCurveConfig, RebalancePriceController,
};
use crate::solana_clock::SolanaClock;
use crate::virtual_stablecoin::VirtualStablecoin;

/// Exchange context for SOL/LST collateral pairs.
#[derive(Clone)]
pub struct LstExchangeContext<C> {
  pub clock: C,
  pub total_sol: UFix64<N9>,
  pub sol_usd_oracle: OraclePrice,
  pub sol_usd_price: PriceRange<N9>,
  virtual_stablecoin: VirtualStablecoin,
  levercoin_supply: Option<UFix64<N6>>,
  collateral_ratio: UFix64<N9>,
  stablecoin_mint_threshold: UFix64<N9>,
  rebalance_mode: RebalanceMode,
  stablecoin_mint_fees: InterpolatedMintFees,
  stablecoin_redeem_fees: InterpolatedRedeemFees,
  levercoin_fees: LevercoinFees,
  rebalance_deviation_tolerance: UFix64<N9>,
  sell_curve_config: RebalanceCurveConfig,
  buy_curve_config: RebalanceCurveConfig,
}

impl<C: SolanaClock> ExchangeContext for LstExchangeContext<C> {
  fn total_collateral(&self) -> UFix64<N9> {
    self.total_sol
  }

  fn collateral_usd_price(&self) -> PriceRange<N9> {
    self.sol_usd_price
  }

  fn collateral_oracle_price(&self) -> OraclePrice {
    self.sol_usd_oracle
  }

  fn stablecoin_mint_threshold(&self) -> UFix64<N9> {
    self.stablecoin_mint_threshold
  }

  fn sell_curve_config(&self) -> &RebalanceCurveConfig {
    &self.sell_curve_config
  }

  fn buy_curve_config(&self) -> &RebalanceCurveConfig {
    &self.buy_curve_config
  }

  fn virtual_stablecoin_supply(&self) -> Result<UFix64<N6>> {
    self.virtual_stablecoin.supply()
  }

  fn levercoin_supply(&self) -> Result<UFix64<N6>> {
    self.levercoin_supply.ok_or(LevercoinNav.into())
  }

  fn rebalance_mode(&self) -> RebalanceMode {
    self.rebalance_mode
  }

  fn collateral_ratio(&self) -> UFix64<N9> {
    self.collateral_ratio
  }

  fn levercoin_fees(&self) -> &LevercoinFees {
    &self.levercoin_fees
  }

  fn rebalance_deviation_tolerance(&self) -> UFix64<N9> {
    self.rebalance_deviation_tolerance
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
    stablecoin_mint_threshold: UFix64<N9>,
    oracle_config: OracleConfig,
    levercoin_fees: LevercoinFees,
    sol_usd_pyth_feed: &PriceUpdateV2,
    virtual_stablecoin: VirtualStablecoin,
    levercoin_mint: Option<&Mint>,
    rebalance_deviation_tolerance: UFix64<N9>,
    sell_curve_config: RebalanceCurveConfig,
    buy_curve_config: RebalanceCurveConfig,
  ) -> Result<LstExchangeContext<C>> {
    let total_sol = total_sol_cache.get_validated(clock.epoch())?;
    let sol_usd_oracle =
      query_pyth_oracle(&clock, sol_usd_pyth_feed, oracle_config)?;
    let sol_usd_price = sol_usd_oracle.price_range()?;
    let stablecoin_mint_fees = InterpolatedMintFees::new(mint_fee_curve()?);
    let stablecoin_redeem_fees =
      InterpolatedRedeemFees::new(redeem_fee_curve()?);
    let stablecoin_supply = virtual_stablecoin.supply()?;
    let levercoin_supply = levercoin_mint.map(|m| UFix64::new(m.supply));
    let collateral_ratio =
      collateral_ratio(total_sol, sol_usd_price.lower, stablecoin_supply)?;
    let rebalance_mode = RebalanceMode::from_cr(collateral_ratio);
    Ok(LstExchangeContext {
      clock,
      total_sol,
      sol_usd_oracle,
      sol_usd_price,
      virtual_stablecoin,
      levercoin_supply,
      collateral_ratio,
      stablecoin_mint_threshold,
      rebalance_mode,
      stablecoin_mint_fees,
      stablecoin_redeem_fees,
      levercoin_fees,
      rebalance_deviation_tolerance,
      sell_curve_config,
      buy_curve_config,
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
    let new_sol =
      lst_sol_price.convert_lst_to_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationCollateral)?;
    let new_total_stablecoin = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst, self.stablecoin_nav()?)?
      .checked_add(&self.virtual_stablecoin_supply()?)
      .ok_or(DestinationStablecoin)?;
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
    let sol_rm =
      lst_sol_price.convert_lst_to_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationCollateral)?;
    let stablecoin_redeemed = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst, self.stablecoin_nav()?)?;
    let new_total_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_redeemed)
      .ok_or(DestinationStablecoin)?;
    let projected_cr = collateral_ratio(
      new_total_sol,
      self.sol_usd_price.lower,
      new_total_stablecoin,
    )?;
    self
      .stablecoin_redeem_fees
      .apply_fee(projected_cr, amount_lst)
  }

  /// Levercoin mint fee based on projected rebalance mode.
  ///
  /// # Errors
  /// * Projection overflow or fee lookup
  pub fn levercoin_mint_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let new_sol =
      lst_sol_price.convert_lst_to_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationCollateral)?;

    let rebalance_mode_for_fees = {
      let projected = self.projected_rebalance_mode(
        new_total_sol,
        self.virtual_stablecoin_supply()?,
      )?;
      self.select_rebalance_mode_for_fees(projected)
    };

    self
      .levercoin_fees
      .mint_fee(rebalance_mode_for_fees)
      .and_then(|fee| FeeExtract::new(fee, amount_lst))
  }

  /// Levercoin redeem fee based on projected rebalance mode.
  ///
  /// # Errors
  /// * Projection underflow or fee lookup
  pub fn levercoin_redeem_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let sol_rm =
      lst_sol_price.convert_lst_to_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationCollateral)?;

    let rebalance_mode_for_fees = {
      let projected = self.projected_rebalance_mode(
        new_total_sol,
        self.virtual_stablecoin_supply()?,
      )?;
      self.select_rebalance_mode_for_fees(projected)
    };

    self
      .levercoin_fees
      .redeem_fee(rebalance_mode_for_fees)
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

  /// Builds conversion for sell-side LST rebalancing.
  ///
  /// # Errors
  /// * Curve setup, pricing, projection overflow, or epoch validation
  pub fn rebalance_sell_conversion(
    &self,
    lst_sol_price: &LstSolPrice,
    usdc_usd_price: PriceRange<N9>,
    usdc_amount: UFix64<N9>,
  ) -> Result<LstRebalanceConversion> {
    let sol_spot_price = self.collateral_oracle_price().spot;
    let lst_sol_price = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    let lst_delta = LstRebalanceConversion::new(
      lst_sol_price,
      sol_spot_price,
      usdc_usd_price,
    )
    .usdc_to_lst(usdc_amount)?;
    let sol_delta = lst_delta
      .mul_div_floor(lst_sol_price, UFix64::one())
      .ok_or(RebalanceAmountExceeded)?;
    let new_total_sol = self
      .total_sol
      .checked_sub(&sol_delta)
      .ok_or(RebalanceAmountExceeded)?;
    let stablecoin_delta = Conversion::spot(sol_spot_price, lst_sol_price)
      .lst_to_token(lst_delta, self.stablecoin_nav()?)?;
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_delta)
      .ok_or(DestinationStablecoin)?;
    let projected_cr =
      collateral_ratio(new_total_sol, sol_spot_price, new_stablecoin)?;
    let curve = self.rebalance_sell_curve()?;
    let sol_usd_price = curve.price(projected_cr)?;
    Ok(LstRebalanceConversion::new(
      lst_sol_price,
      sol_usd_price,
      usdc_usd_price,
    ))
  }

  /// Builds conversion for buy-side LST rebalancing.
  ///
  /// # Errors
  /// * Curve setup, pricing, projection overflow, or epoch validation
  pub fn rebalance_buy_conversion(
    &self,
    lst_sol_price: &LstSolPrice,
    usdc_usd_price: PriceRange<N9>,
    lst_amount: UFix64<N9>,
  ) -> Result<LstRebalanceConversion> {
    let usd_sol_price = self.collateral_oracle_price().spot;
    let lst_sol_price = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    let sol_delta = lst_amount
      .mul_div_floor(lst_sol_price, UFix64::one())
      .ok_or(RebalanceAmountExceeded)?;
    let new_total_sol = self
      .total_sol
      .checked_add(&sol_delta)
      .ok_or(DestinationCollateral)?;
    let stablecoin_delta = Conversion::spot(usd_sol_price, lst_sol_price)
      .lst_to_token(lst_amount, self.stablecoin_nav()?)?;
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_add(&stablecoin_delta)
      .ok_or(DestinationStablecoin)?;
    let projected_cr =
      collateral_ratio(new_total_sol, usd_sol_price, new_stablecoin)?;
    let curve = self.rebalance_buy_curve()?;
    let sol_usd_price = curve.price(projected_cr)?;
    Ok(LstRebalanceConversion::new(
      lst_sol_price,
      sol_usd_price,
      usdc_usd_price,
    ))
  }
}
