use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fix::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use super::ExchangeContext;
use crate::conversion::{
  ExoConversion, ExoRebalanceConversion, UsdcStablecoinConversion,
};
use crate::error::CoreError::{
  DestinationCollateral, DestinationStablecoin, LevercoinSupplyNotSet,
  RebalanceAmountExceeded, RebalanceSwapPnl, VirtualStablecoinBurnLimit,
};
use crate::exchange_math::collateral_ratio;
use crate::fees::controller::{FeeController, FeeExtract, LevercoinFees};
use crate::fees::curve_controller::{
  InterpolatedFeeController, InterpolatedMintFees, InterpolatedRedeemFees,
};
use crate::fees::curves::{mint_fee_curve, redeem_fee_curve};
use crate::levercoin_limiter::LevercoinMarketCapLimiter;
use crate::pyth::{query_pyth_oracle, OracleConfig, OraclePrice, PriceRange};
use crate::rebalance::mode::RebalanceMode;
use crate::rebalance::pnl::RebalancePnl;
use crate::rebalance::pricing::{
  RebalanceCurveConfig, RebalancePriceController,
};
use crate::solana_clock::SolanaClock;
use crate::virtual_stablecoin::VirtualStablecoin;

/// Exchange context for exogenous collateral pairs.
pub struct ExoExchangeContext<C> {
  pub clock: C,
  pub total_collateral: UFix64<N9>,
  pub collateral_oracle: OraclePrice,
  pub collateral_usd_price: PriceRange<N9>,
  pub virtual_stablecoin: VirtualStablecoin,
  levercoin_supply: Option<UFix64<N6>>,
  collateral_ratio: UFix64<N9>,
  stablecoin_mint_threshold: UFix64<N9>,
  rebalance_mode: RebalanceMode,
  levercoin_fees: LevercoinFees,
  stablecoin_mint_fees: InterpolatedMintFees,
  stablecoin_redeem_fees: InterpolatedRedeemFees,
  rebalance_deviation_tolerance: UFix64<N9>,
  sell_curve_config: RebalanceCurveConfig,
  buy_curve_config: RebalanceCurveConfig,
  levercoin_market_cap_limit: UFix64<N9>,
}

impl<C: SolanaClock> ExchangeContext for ExoExchangeContext<C> {
  fn total_collateral(&self) -> UFix64<N9> {
    self.total_collateral
  }

  fn collateral_usd_price(&self) -> PriceRange<N9> {
    self.collateral_usd_price
  }

  fn collateral_oracle_price(&self) -> OraclePrice {
    self.collateral_oracle
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
    self.levercoin_supply.ok_or(LevercoinSupplyNotSet.into())
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

impl<C: SolanaClock> ExoExchangeContext<C> {
  /// Builds context from account data.
  ///
  /// # Errors
  /// * Oracle, curve, or stability controller validation
  #[allow(clippy::too_many_arguments)]
  pub fn load(
    clock: C,
    total_collateral: UFix64<N9>,
    stablecoin_mint_threshold: UFix64<N9>,
    oracle_config: OracleConfig,
    levercoin_fees: LevercoinFees,
    collateral_usd_pyth_feed: &PriceUpdateV2,
    virtual_stablecoin: VirtualStablecoin,
    levercoin_mint: Option<&Mint>,
    rebalance_deviation_tolerance: UFix64<N9>,
    sell_curve_config: RebalanceCurveConfig,
    buy_curve_config: RebalanceCurveConfig,
    levercoin_market_cap_limit: UFix64<N9>,
  ) -> Result<ExoExchangeContext<C>> {
    let collateral_oracle =
      query_pyth_oracle(&clock, collateral_usd_pyth_feed, oracle_config)?;
    let collateral_usd_price = collateral_oracle.price_range()?;
    let stablecoin_mint_fees = InterpolatedMintFees::new(mint_fee_curve()?);
    let stablecoin_redeem_fees =
      InterpolatedRedeemFees::new(redeem_fee_curve()?);
    let levercoin_supply = levercoin_mint.map(|m| UFix64::new(m.supply));
    let stablecoin_supply = virtual_stablecoin.supply()?;
    let collateral_ratio = collateral_ratio(
      total_collateral,
      collateral_usd_price.lower,
      stablecoin_supply,
    )?;
    let rebalance_mode = RebalanceMode::from_cr(collateral_ratio);
    Ok(ExoExchangeContext {
      clock,
      total_collateral,
      collateral_oracle,
      collateral_usd_price,
      virtual_stablecoin,
      levercoin_supply,
      collateral_ratio,
      stablecoin_mint_threshold,
      rebalance_mode,
      levercoin_fees,
      stablecoin_mint_fees,
      stablecoin_redeem_fees,
      rebalance_deviation_tolerance,
      sell_curve_config,
      buy_curve_config,
      levercoin_market_cap_limit,
    })
  }

  /// Stablecoin mint fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection overflow, interpolation, or fee extraction
  pub fn stablecoin_mint_fee(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let new_total = self
      .total_collateral
      .checked_add(&collateral_amount)
      .ok_or(DestinationCollateral)?;
    let stablecoin_minted = self
      .exo_conversion()
      .exo_to_token(collateral_amount, self.stablecoin_nav()?)?;
    let new_stablecoin = stablecoin_minted
      .checked_add(&self.virtual_stablecoin_supply()?)
      .ok_or(DestinationStablecoin)?;
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price.lower,
      new_stablecoin,
    )?;
    self
      .stablecoin_mint_fees
      .apply_fee(projected_cr, collateral_amount)
  }

  /// Stablecoin redeem fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection underflow, interpolation, or fee extraction
  pub fn stablecoin_redeem_fee(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let new_total = self
      .total_collateral
      .checked_sub(&collateral_amount)
      .ok_or(DestinationCollateral)?;
    let stablecoin_redeemed = self
      .exo_conversion()
      .exo_to_token(collateral_amount, self.stablecoin_nav()?)?;
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_redeemed)
      .ok_or(DestinationStablecoin)?;
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price.lower,
      new_stablecoin,
    )?;
    self
      .stablecoin_redeem_fees
      .apply_fee(projected_cr, collateral_amount)
  }

  /// Levercoin mint fee based on projected rebalance mode.
  ///
  /// # Errors
  /// * Projection overflow or mode-based fee lookup
  pub fn levercoin_mint_fee(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let new_total = self
      .total_collateral
      .checked_add(&collateral_amount)
      .ok_or(DestinationCollateral)?;
    let projected = self
      .projected_rebalance_mode(new_total, self.virtual_stablecoin_supply()?)?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    let fee = self.levercoin_fees.mint_fee(mode)?;
    FeeExtract::new(fee, collateral_amount)
  }

  /// Levercoin redeem fee based on projected rebalance mode.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  pub fn levercoin_redeem_fee(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    let new_total = self
      .total_collateral
      .checked_sub(&collateral_amount)
      .ok_or(DestinationCollateral)?;
    let projected = self
      .projected_rebalance_mode(new_total, self.virtual_stablecoin_supply()?)?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    let fee = self.levercoin_fees.redeem_fee(mode)?;
    FeeExtract::new(fee, collateral_amount)
  }

  /// Builds conversion helper between exogenous collateral and token.
  #[must_use]
  pub fn exo_conversion(&self) -> ExoConversion {
    ExoConversion::new(self.collateral_usd_price)
  }

  /// Builds conversion for sell side rebalancing
  ///
  /// # Errors
  /// * Curve setup, pricing, or projection overflow
  pub fn rebalance_sell_conversion(
    &self,
    usdc_usd_price: PriceRange<N9>,
    usdc_amount: UFix64<N9>,
  ) -> Result<ExoRebalanceConversion> {
    let spot_price = self.collateral_oracle_price().spot;
    let collateral_delta =
      ExoRebalanceConversion::new(spot_price, usdc_usd_price)
        .usdc_to_collateral(usdc_amount)?;
    let new_total = self
      .total_collateral
      .checked_sub(&collateral_delta)
      .ok_or(RebalanceAmountExceeded)?;
    let stablecoin_delta = ExoConversion::spot(spot_price)
      .exo_to_token(collateral_delta, self.stablecoin_nav()?)?;
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_delta)
      .ok_or(DestinationStablecoin)?;
    let projected_cr = collateral_ratio(new_total, spot_price, new_stablecoin)?;
    let collateral_usd_price =
      self.rebalance_sell_curve()?.price(projected_cr)?;
    Ok(ExoRebalanceConversion::new(
      collateral_usd_price,
      usdc_usd_price,
    ))
  }

  /// Largest USDC input a sell-side rebalancing swap can take while staying
  /// within sellable collateral and the virtual stablecoin floor.
  ///
  /// # Errors
  /// * Virtual stablecoin is below the floor
  /// * Arithmetic overflow
  pub fn max_rebalance_sell_usdc(
    &self,
    usdc_usd_price: PriceRange<N9>,
    virtual_stablecoin_supply_floor: UFix64<N6>,
  ) -> Result<UFix64<N9>> {
    // Collateral the protocol can sell, priced as USDC at spot
    let sellable_collateral =
      self.rebalance_sell_liquidity()?.min(self.total_collateral);
    let spot_price = self.collateral_oracle_price().spot;
    let conversion = ExoRebalanceConversion::new(spot_price, usdc_usd_price);
    let usdc_in_raw = conversion.collateral_to_usdc(sellable_collateral)?;

    // Virtual stablecoin at or above the floor converted to USDC
    let virtual_stablecoin_supply = self.virtual_stablecoin_supply()?;
    let max_burnable_stablecoin = virtual_stablecoin_supply
      .checked_sub(&virtual_stablecoin_supply_floor)
      .ok_or(VirtualStablecoinBurnLimit)?;
    let usdc_limit = UsdcStablecoinConversion::new(usdc_usd_price)
      .stablecoin_to_withdrawal(max_burnable_stablecoin)?;

    Ok(usdc_in_raw.min(usdc_limit))
  }

  /// Builds conversion for buy side rebalancing
  ///
  /// # Errors
  /// * Curve setup, pricing, or projection overflow
  pub fn rebalance_buy_conversion(
    &self,
    usdc_usd_price: PriceRange<N9>,
    collateral_amount: UFix64<N9>,
  ) -> Result<ExoRebalanceConversion> {
    let spot_price = self.collateral_oracle_price().spot;
    let new_total = self
      .total_collateral
      .checked_add(&collateral_amount)
      .ok_or(DestinationCollateral)?;
    let stablecoin_delta = ExoConversion::spot(spot_price)
      .exo_to_token(collateral_amount, self.stablecoin_nav()?)?;
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_add(&stablecoin_delta)
      .ok_or(DestinationStablecoin)?;
    let projected_cr = collateral_ratio(new_total, spot_price, new_stablecoin)?;
    let curve = self.rebalance_buy_curve()?;
    let collateral_usd_price = curve.price(projected_cr)?;
    Ok(ExoRebalanceConversion::new(
      collateral_usd_price,
      usdc_usd_price,
    ))
  }

  /// Builds the levercoin market cap limiter from exchange inputs.
  ///
  /// # Errors
  /// * Levercoin supply not set
  /// * Levercoin mint NAV fails
  pub fn levercoin_market_cap_limiter(
    &self,
  ) -> Result<LevercoinMarketCapLimiter> {
    let levercoin_supply = self.levercoin_supply()?;
    let levercoin_nav = self.levercoin_mint_nav()?;
    Ok(LevercoinMarketCapLimiter::new(
      self.levercoin_market_cap_limit,
      levercoin_nav,
      levercoin_supply,
    ))
  }

  /// Stablecoin value of collateral at oracle spot price.
  ///
  /// # Errors
  /// * NAV computation
  /// * Conversion arithmetic
  pub fn exo_to_stablecoin_spot(
    &self,
    exo_amount: UFix64<N9>,
  ) -> Result<UFix64<N6>> {
    let spot = self.collateral_oracle_price().spot;
    let stablecoin_nav = self.stablecoin_nav()?;
    ExoConversion::spot(spot).exo_to_token(exo_amount, stablecoin_nav)
  }

  /// Computes rebalance `PnL` for a buy-side swap.
  ///
  /// # Errors
  /// * Spot conversion arithmetic
  /// * `PnL` arithmetic overflow
  pub fn rebalance_pnl_buy_side(
    &self,
    exo_in: UFix64<N9>,
    stablecoin_moved: UFix64<N6>,
  ) -> Result<RebalancePnl> {
    let stablecoin_value_in = self.exo_to_stablecoin_spot(exo_in)?;
    RebalancePnl::from_stablecoin_flow(stablecoin_value_in, stablecoin_moved)
      .ok_or(RebalanceSwapPnl.into())
  }

  /// Computes rebalance `PnL` for a sell-side swap.
  ///
  /// # Errors
  /// * Spot conversion arithmetic
  /// * `PnL` arithmetic overflow
  pub fn rebalance_pnl_sell_side(
    &self,
    exo_out: UFix64<N9>,
    stablecoin_moved: UFix64<N6>,
  ) -> Result<RebalancePnl> {
    let stablecoin_value_out = self.exo_to_stablecoin_spot(exo_out)?;
    RebalancePnl::from_stablecoin_flow(stablecoin_moved, stablecoin_value_out)
      .ok_or(RebalanceSwapPnl.into())
  }
}
