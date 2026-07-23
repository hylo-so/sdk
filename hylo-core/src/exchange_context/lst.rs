use anchor_spl::token::Mint;
use fix::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use super::{ExchangeContext, ProjectedState};
use crate::conversion::{
  Conversion, LstRebalanceConversion, UsdcStablecoinConversion,
};
use crate::error::CoreError;
use crate::error::CoreError::{
  DestinationCollateral, DestinationStablecoin, LevercoinNav,
  RebalanceAmountExceeded, RebalanceSwapPnl, VirtualStablecoinBurnLimit,
};
use crate::exchange_math::collateral_ratio;
use crate::fees::controller::{FeeController, FeeExtract, LevercoinFees};
use crate::fees::curve_controller::{
  InterpolatedFeeController, InterpolatedMintFees, InterpolatedRedeemFees,
};
use crate::fees::curves::{mint_fee_curve, redeem_fee_curve};
use crate::lst::sol_price::LstSolPrice;
use crate::lst::stake_pool::SplStakePool;
use crate::lst::total_sol_cache::TotalSolCache;
use crate::pyth::{query_pyth_oracle, OracleConfig, OraclePrice, PriceRange};
use crate::rebalance::mode::RebalanceMode;
use crate::rebalance::pnl::RebalancePnl;
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
  pub stablecoin_mint_fees: InterpolatedMintFees,
  pub stablecoin_redeem_fees: InterpolatedRedeemFees,
  levercoin_fees: LevercoinFees,
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

  fn virtual_stablecoin_supply(&self) -> Result<UFix64<N6>, CoreError> {
    self.virtual_stablecoin.supply()
  }

  fn levercoin_supply(&self) -> Result<UFix64<N6>, CoreError> {
    self.levercoin_supply.ok_or(LevercoinNav)
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
    sell_curve_config: RebalanceCurveConfig,
    buy_curve_config: RebalanceCurveConfig,
  ) -> Result<LstExchangeContext<C>, CoreError> {
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
    amount_lst_in: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let projected = self.projected_mint_state(lst_sol_price, amount_lst_in)?;
    self
      .stablecoin_mint_fees
      .apply_fee(projected.collateral_ratio, amount_lst_in)
  }

  /// Post-trade state used by the stablecoin mint fee projection.
  pub fn projected_mint_state(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_in: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let new_sol =
      lst_sol_price.convert_lst_to_sol(amount_lst_in, self.clock.epoch())?;
    let total_collateral = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationCollateral)?;
    let stablecoin_supply = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst_in, self.stablecoin_nav()?)?
      .checked_add(&self.virtual_stablecoin_supply()?)
      .ok_or(DestinationStablecoin)?;
    let collateral_ratio = collateral_ratio(
      total_collateral,
      self.sol_usd_price.lower,
      stablecoin_supply,
    )?;
    Ok(ProjectedState {
      total_collateral,
      stablecoin_supply,
      collateral_ratio,
    })
  }

  /// Stablecoin mint fee rate at the projected CR.
  ///
  /// # Errors
  /// * Projection overflow or curve lookup
  #[cfg(feature = "offchain")]
  pub fn stablecoin_mint_fee_rate(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_in: UFix64<N9>,
  ) -> Result<UFix64<N5>, CoreError> {
    let projected = self.projected_mint_state(lst_sol_price, amount_lst_in)?;
    self
      .stablecoin_mint_fees
      .fee_rate(projected.collateral_ratio)
  }

  /// Stablecoin redeem fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection underflow, interpolation, or fee extraction
  pub fn stablecoin_redeem_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_out: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let projected =
      self.projected_redeem_state(lst_sol_price, amount_lst_out)?;
    self
      .stablecoin_redeem_fees
      .apply_fee(projected.collateral_ratio, amount_lst_out)
  }

  /// Stablecoin redeem fee rate at the projected CR.
  ///
  /// # Errors
  /// * Projection underflow or curve lookup
  #[cfg(feature = "offchain")]
  pub fn stablecoin_redeem_fee_rate(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_out: UFix64<N9>,
  ) -> Result<UFix64<N5>, CoreError> {
    let projected =
      self.projected_redeem_state(lst_sol_price, amount_lst_out)?;
    self
      .stablecoin_redeem_fees
      .fee_rate(projected.collateral_ratio)
  }

  /// Post-trade state used by the stablecoin redeem fee projection.
  pub fn projected_redeem_state(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_out: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let sol_rm =
      lst_sol_price.convert_lst_to_sol(amount_lst_out, self.clock.epoch())?;
    let total_collateral = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationCollateral)?;
    let stablecoin_redeemed = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst_out, self.stablecoin_nav()?)?;
    let stablecoin_supply = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_redeemed)
      .ok_or(DestinationStablecoin)?;
    let collateral_ratio = collateral_ratio(
      total_collateral,
      self.sol_usd_price.lower,
      stablecoin_supply,
    )?;
    Ok(ProjectedState {
      total_collateral,
      stablecoin_supply,
      collateral_ratio,
    })
  }

  /// Levercoin mint fee based on projected rebalance mode.
  ///
  /// # Errors
  /// * Projection overflow or fee lookup
  pub fn levercoin_mint_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_in: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let rate = self.levercoin_mint_fee_rate(lst_sol_price, amount_lst_in)?;
    FeeExtract::new(rate, amount_lst_in)
  }

  /// Levercoin mint fee rate at the projected rebalance mode.
  ///
  /// # Errors
  /// * Projection overflow or mode-based fee lookup
  pub fn levercoin_mint_fee_rate(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_in: UFix64<N9>,
  ) -> Result<UFix64<N4>, CoreError> {
    let new_sol =
      lst_sol_price.convert_lst_to_sol(amount_lst_in, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationCollateral)?;
    let projected = self.projected_rebalance_mode(
      new_total_sol,
      self.virtual_stablecoin_supply()?,
    )?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    self.levercoin_fees.mint_fee(mode)
  }

  /// Levercoin redeem fee based on projected rebalance mode.
  ///
  /// # Errors
  /// * Projection underflow or fee lookup
  pub fn levercoin_redeem_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_out: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let rate = self.levercoin_redeem_fee_rate(lst_sol_price, amount_lst_out)?;
    FeeExtract::new(rate, amount_lst_out)
  }

  /// Levercoin redeem fee rate at the projected rebalance mode.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  pub fn levercoin_redeem_fee_rate(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst_out: UFix64<N9>,
  ) -> Result<UFix64<N4>, CoreError> {
    let sol_rm =
      lst_sol_price.convert_lst_to_sol(amount_lst_out, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationCollateral)?;
    let projected = self.projected_rebalance_mode(
      new_total_sol,
      self.virtual_stablecoin_supply()?,
    )?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    self.levercoin_fees.redeem_fee(mode)
  }

  /// Overflow frontier for adding an LST deposit to total SOL.
  ///
  /// # Errors
  /// * Price outdated or degenerate
  #[cfg(feature = "offchain")]
  pub fn max_collateral_deposit(
    &self,
    lst_sol_price: &LstSolPrice,
  ) -> Result<UFix64<N9>, CoreError> {
    let headroom = UFix64::new(u64::MAX)
      .checked_sub(&self.total_sol)
      .ok_or(DestinationCollateral)?;
    lst_sol_price.max_lst_for_sol(headroom, self.clock.epoch())
  }

  /// LST/SOL token conversion helper.
  ///
  /// # Errors
  /// * Epoch price lookup failure
  pub fn token_conversion(
    &self,
    lst_sol_price: &LstSolPrice,
  ) -> Result<Conversion, CoreError> {
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
  ) -> Result<UFix64<N6>, CoreError> {
    let nav = self.stablecoin_nav()?;
    let conversion = Conversion::new(self.sol_usd_price, UFix64::one());
    conversion.lst_to_token(amount_sol, nav)
  }

  /// Special case conversion from raw SOL to levercoin.
  ///
  /// # Errors
  /// * NAV or arithmetic failure
  pub fn sol_to_levercoin(
    &self,
    amount_sol: UFix64<N9>,
  ) -> Result<UFix64<N6>, CoreError> {
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
  ) -> Result<LstRebalanceConversion, CoreError> {
    let lst_sol = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    let projected = self.projected_rebalance_sell_state(
      lst_sol_price,
      usdc_usd_price,
      usdc_amount,
    )?;
    let sol_usd_price = self
      .rebalance_sell_curve()?
      .price(projected.collateral_ratio)?;
    Ok(LstRebalanceConversion::new(
      lst_sol,
      sol_usd_price,
      usdc_usd_price,
    ))
  }

  /// Post-trade state used by the sell-side rebalance projection.
  pub fn projected_rebalance_sell_state(
    &self,
    lst_sol_price: &LstSolPrice,
    usdc_usd_price: PriceRange<N9>,
    usdc_amount: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let sol_spot_price = self.collateral_oracle_price().spot;
    let lst_sol = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    let lst_delta =
      LstRebalanceConversion::new(lst_sol, sol_spot_price, usdc_usd_price)
        .usdc_to_lst(usdc_amount)?;
    let sol_delta =
      lst_sol_price.convert_lst_to_sol(lst_delta, self.clock.epoch())?;
    let total_collateral = self
      .total_sol
      .checked_sub(&sol_delta)
      .ok_or(RebalanceAmountExceeded)?;
    let stablecoin_delta = Conversion::spot(sol_spot_price, lst_sol)
      .lst_to_token(lst_delta, self.stablecoin_nav()?)?;
    let stablecoin_supply = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_delta)
      .ok_or(DestinationStablecoin)?;
    let collateral_ratio =
      collateral_ratio(total_collateral, sol_spot_price, stablecoin_supply)?;
    Ok(ProjectedState {
      total_collateral,
      stablecoin_supply,
      collateral_ratio,
    })
  }

  /// Maximum USDC input for a sell-side rebalancing swap.
  ///
  /// # Errors
  /// * LST price is outdated
  /// * Virtual stablecoin is below the floor
  /// * Arithmetic overflow
  pub fn max_rebalance_sell_usdc(
    &self,
    stake_pool: SplStakePool,
    rebalance_fee: UFix64<N5>,
    lst_vault_balance: UFix64<N9>,
    usdc_usd_price: PriceRange<N9>,
    virtual_stablecoin_supply_floor: UFix64<N6>,
  ) -> Result<UFix64<N9>, CoreError> {
    // Sellable total collateral as LST capped by vault balance
    let true_price = stake_pool.true_price()?;
    let adjusted_price = true_price.adjust_price(rebalance_fee)?;
    let sellable_lst = adjusted_price
      .convert_sol_to_lst(self.rebalance_sell_liquidity()?, self.clock.epoch())?
      .min(lst_vault_balance);

    // Convert to USDC at spot
    let lst_sol = adjusted_price.get_epoch_price(self.clock.epoch())?;
    let sol_spot_price = self.collateral_oracle_price().spot;
    let usdc_in_raw =
      LstRebalanceConversion::new(lst_sol, sol_spot_price, usdc_usd_price)
        .lst_to_usdc(sellable_lst)?;

    // Virtual stablecoin at or above the floor converted to USDC
    let max_burnable_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&virtual_stablecoin_supply_floor)
      .ok_or(VirtualStablecoinBurnLimit)?;
    let usdc_limit = UsdcStablecoinConversion::new(usdc_usd_price)
      .stablecoin_to_withdrawal(max_burnable_stablecoin)?;

    Ok(usdc_in_raw.min(usdc_limit))
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
  ) -> Result<LstRebalanceConversion, CoreError> {
    let lst_sol = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    let projected =
      self.projected_rebalance_buy_state(lst_sol_price, lst_amount)?;
    let sol_usd_price = self
      .rebalance_buy_curve()?
      .price(projected.collateral_ratio)?;
    Ok(LstRebalanceConversion::new(
      lst_sol,
      sol_usd_price,
      usdc_usd_price,
    ))
  }

  /// Post-trade state used by the buy-side rebalance projection.
  pub fn projected_rebalance_buy_state(
    &self,
    lst_sol_price: &LstSolPrice,
    lst_amount: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let usd_sol_price = self.collateral_oracle_price().spot;
    let lst_sol = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    let sol_delta = lst_amount
      .mul_div_floor(lst_sol, UFix64::one())
      .ok_or(RebalanceAmountExceeded)?;
    let total_collateral = self
      .total_sol
      .checked_add(&sol_delta)
      .ok_or(DestinationCollateral)?;
    let stablecoin_delta = Conversion::spot(usd_sol_price, lst_sol)
      .lst_to_token(lst_amount, self.stablecoin_nav()?)?;
    let stablecoin_supply = self
      .virtual_stablecoin_supply()?
      .checked_add(&stablecoin_delta)
      .ok_or(DestinationStablecoin)?;
    let collateral_ratio =
      collateral_ratio(total_collateral, usd_sol_price, stablecoin_supply)?;
    Ok(ProjectedState {
      total_collateral,
      stablecoin_supply,
      collateral_ratio,
    })
  }

  /// Converts LST amount to protocol stablecoin at SOL/USD spot price.
  ///
  /// # Errors
  /// * LST price not updated
  /// * NAV computation
  /// * Conversion arithmetic
  pub fn lst_to_stablecoin_spot(
    &self,
    lst_sol_price: &LstSolPrice,
    lst_amount: UFix64<N9>,
  ) -> Result<UFix64<N6>, CoreError> {
    let lst_sol = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    let usd_sol_price = self.collateral_oracle_price().spot;
    let stablecoin_nav = self.stablecoin_nav()?;
    let conversion = Conversion::spot(usd_sol_price, lst_sol);
    conversion.lst_to_token(lst_amount, stablecoin_nav)
  }

  /// Computes rebalance `PnL` for a buy-side LST swap.
  ///
  /// # Errors
  /// * Spot conversion arithmetic
  /// * `PnL` arithmetic overflow
  pub fn rebalance_pnl_buy_side(
    &self,
    lst_sol_price: &LstSolPrice,
    lst_in: UFix64<N9>,
    stablecoin_moved: UFix64<N6>,
  ) -> Result<RebalancePnl, CoreError> {
    let stablecoin_value_in =
      self.lst_to_stablecoin_spot(lst_sol_price, lst_in)?;
    RebalancePnl::from_stablecoin_flow(stablecoin_value_in, stablecoin_moved)
      .ok_or(RebalanceSwapPnl)
  }

  /// Computes rebalance `PnL` for a sell-side LST swap.
  ///
  /// # Errors
  /// * Spot conversion arithmetic
  /// * `PnL` arithmetic overflow
  pub fn rebalance_pnl_sell_side(
    &self,
    lst_sol_price: &LstSolPrice,
    lst_out: UFix64<N9>,
    stablecoin_moved: UFix64<N6>,
  ) -> Result<RebalancePnl, CoreError> {
    let stablecoin_value_out =
      self.lst_to_stablecoin_spot(lst_sol_price, lst_out)?;
    RebalancePnl::from_stablecoin_flow(stablecoin_moved, stablecoin_value_out)
      .ok_or(RebalanceSwapPnl)
  }
}
