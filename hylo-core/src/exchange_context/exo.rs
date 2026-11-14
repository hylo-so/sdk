use anchor_spl::token::Mint;
use fix::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use super::{ExchangeContext, ProjectedState};
use crate::collateral_ratio::{CollateralRatio, CR};
use crate::conversion::ExoConversion;
use crate::error::CoreError;
use crate::error::CoreError::{
  DestinationCollateral, DestinationStablecoin, LevercoinSupplyNotSet,
  RebalanceAmountExceeded, RebalanceSwapPnl, VirtualStablecoinBurnLimit,
};
use crate::fees::controller::{FeeController, FeeExtract, LevercoinFees};
use crate::fees::curve_controller::{
  InterpolatedFeeController, InterpolatedMintFees, InterpolatedRedeemFees,
};
use crate::fees::curves::{mint_fee_curve, redeem_fee_curve};
use crate::limiter::levercoin::LevercoinMarketCapLimiter;
use crate::pyth::{query_pyth_oracle, OracleConfig, OraclePrice, PriceRange};
use crate::rebalance::mode::RebalanceMode;
use crate::rebalance::pnl::RebalancePnl;
use crate::rebalance::pricing::{
  RebalanceCurveConfig, RebalancePriceController,
};
use crate::solana_clock::SolanaClock;
use crate::virtual_stablecoin::VirtualStablecoin;

/// Exchange context for exogenous collateral pairs.
#[derive(Clone)]
pub struct ExoExchangeContext<C> {
  pub clock: C,
  pub total_collateral: UFix64<N9>,
  pub collateral_oracle: OraclePrice,
  pub collateral_usd_price: PriceRange<N9>,
  pub virtual_stablecoin: VirtualStablecoin,
  levercoin_supply: Option<UFix64<N6>>,
  collateral_ratio: CR,
  stablecoin_mint_threshold: UFix64<N9>,
  rebalance_mode: RebalanceMode,
  levercoin_fees: LevercoinFees,
  pub stablecoin_mint_fees: InterpolatedMintFees,
  pub stablecoin_redeem_fees: InterpolatedRedeemFees,
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

  fn virtual_stablecoin_supply(&self) -> Result<UFix64<N6>, CoreError> {
    self.virtual_stablecoin.supply()
  }

  fn levercoin_supply(&self) -> Result<UFix64<N6>, CoreError> {
    self.levercoin_supply.ok_or(LevercoinSupplyNotSet)
  }

  fn rebalance_mode(&self) -> RebalanceMode {
    self.rebalance_mode
  }

  fn collateral_ratio(&self) -> CollateralRatio {
    self.collateral_ratio
  }

  fn levercoin_fees(&self) -> &LevercoinFees {
    &self.levercoin_fees
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
    sell_curve_config: RebalanceCurveConfig,
    buy_curve_config: RebalanceCurveConfig,
    levercoin_market_cap_limit: UFix64<N9>,
  ) -> Result<ExoExchangeContext<C>, CoreError> {
    let collateral_oracle =
      query_pyth_oracle(&clock, collateral_usd_pyth_feed, oracle_config)?;
    let collateral_usd_price = collateral_oracle.price_range()?;
    let stablecoin_mint_fees = InterpolatedMintFees::new(mint_fee_curve()?);
    let stablecoin_redeem_fees =
      InterpolatedRedeemFees::new(redeem_fee_curve()?);
    let levercoin_supply = levercoin_mint.map(|m| UFix64::new(m.supply));
    let stablecoin_supply = virtual_stablecoin.supply()?;
    let collateral_ratio = CollateralRatio::new(
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
    collateral_amount_in: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let projected = self.projected_mint_state(collateral_amount_in)?;
    self
      .stablecoin_mint_fees
      .apply_fee(projected.collateral_ratio, collateral_amount_in)
  }

  /// Stablecoin mint fee rate at the projected CR.
  ///
  /// # Errors
  /// * Projection overflow or curve lookup
  #[cfg(feature = "offchain")]
  pub fn stablecoin_mint_fee_rate(
    &self,
    collateral_amount_in: UFix64<N9>,
  ) -> Result<UFix64<N5>, CoreError> {
    let projected = self.projected_mint_state(collateral_amount_in)?;
    self
      .stablecoin_mint_fees
      .fee_rate(projected.collateral_ratio)
  }

  /// Post-trade state used by the stablecoin mint fee projection.
  pub fn projected_mint_state(
    &self,
    collateral_amount_in: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let total_collateral = self
      .total_collateral
      .checked_add(&collateral_amount_in)
      .ok_or(DestinationCollateral)?;
    let stablecoin_minted = self
      .exo_conversion()
      .exo_to_token(collateral_amount_in, self.stablecoin_nav()?)?;
    let stablecoin_supply = stablecoin_minted
      .checked_add(&self.virtual_stablecoin_supply()?)
      .ok_or(DestinationStablecoin)?;
    let collateral_ratio = CollateralRatio::new(
      total_collateral,
      self.collateral_usd_price.lower,
      stablecoin_supply,
    )?;
    Ok(ProjectedState {
      total_collateral,
      stablecoin_supply,
      collateral_ratio,
    })
  }

  /// Stablecoin redeem fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection underflow, interpolation, or fee extraction
  pub fn stablecoin_redeem_fee(
    &self,
    collateral_amount_out: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let projected = self.projected_redeem_state(collateral_amount_out)?;
    self
      .stablecoin_redeem_fees
      .apply_fee(projected.collateral_ratio, collateral_amount_out)
  }

  /// Stablecoin redeem fee rate at the projected CR.
  ///
  /// # Errors
  /// * Projection underflow or curve lookup
  #[cfg(feature = "offchain")]
  pub fn stablecoin_redeem_fee_rate(
    &self,
    collateral_amount_out: UFix64<N9>,
  ) -> Result<UFix64<N5>, CoreError> {
    let projected = self.projected_redeem_state(collateral_amount_out)?;
    self
      .stablecoin_redeem_fees
      .fee_rate(projected.collateral_ratio)
  }

  /// Stablecoin redeem fee at the projected CR, saturating at `y_max`.
  ///
  /// # Errors
  /// * Projection underflow, interpolation, or fee extraction
  #[cfg(feature = "offchain")]
  pub fn saturating_stablecoin_redeem_fee(
    &self,
    collateral_amount_out: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let projected = self.projected_redeem_state(collateral_amount_out)?;
    let fee_rate = self
      .stablecoin_redeem_fees
      .saturating_fee_rate(projected.collateral_ratio)?;
    FeeExtract::new(fee_rate, collateral_amount_out)
  }

  /// Post-trade state used by the stablecoin redeem fee projection.
  pub fn projected_redeem_state(
    &self,
    collateral_amount_out: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let total_collateral = self
      .total_collateral
      .checked_sub(&collateral_amount_out)
      .ok_or(DestinationCollateral)?;
    let stablecoin_redeemed = self
      .exo_conversion()
      .exo_to_token(collateral_amount_out, self.stablecoin_nav()?)?;
    let stablecoin_supply = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_redeemed)
      .ok_or(DestinationStablecoin)?;
    let collateral_ratio = CollateralRatio::new(
      total_collateral,
      self.collateral_usd_price.lower,
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
  /// * Projection overflow or mode-based fee lookup
  pub fn levercoin_mint_fee(
    &self,
    collateral_amount_in: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let rate = self.levercoin_mint_fee_rate(collateral_amount_in)?;
    FeeExtract::new(rate, collateral_amount_in)
  }

  /// Levercoin mint fee rate at the projected rebalance mode.
  ///
  /// # Errors
  /// * Projection overflow or mode-based fee lookup
  pub fn levercoin_mint_fee_rate(
    &self,
    collateral_amount_in: UFix64<N9>,
  ) -> Result<UFix64<N4>, CoreError> {
    let new_total = self
      .total_collateral
      .checked_add(&collateral_amount_in)
      .ok_or(DestinationCollateral)?;
    let projected = self
      .projected_rebalance_mode(new_total, self.virtual_stablecoin_supply()?)?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    self.levercoin_fees.mint_fee(mode)
  }

  /// Levercoin redeem fee based on projected rebalance mode.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  pub fn levercoin_redeem_fee(
    &self,
    collateral_amount_out: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let rate = self.levercoin_redeem_fee_rate(collateral_amount_out)?;
    FeeExtract::new(rate, collateral_amount_out)
  }

  /// Levercoin redeem fee rate at the projected rebalance mode.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  pub fn levercoin_redeem_fee_rate(
    &self,
    collateral_amount_out: UFix64<N9>,
  ) -> Result<UFix64<N4>, CoreError> {
    let new_total = self
      .total_collateral
      .checked_sub(&collateral_amount_out)
      .ok_or(DestinationCollateral)?;
    let projected = self
      .projected_rebalance_mode(new_total, self.virtual_stablecoin_supply()?)?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    self.levercoin_fees.redeem_fee(mode)
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
    usdc_amount: UFix64<N6>,
  ) -> Result<ExoConversion, CoreError> {
    let projected = self.projected_rebalance_sell_state(usdc_amount)?;
    let collateral_usd_price = self
      .rebalance_sell_curve()?
      .price(projected.collateral_ratio)?;
    Ok(ExoConversion::spot(collateral_usd_price))
  }

  /// Post-trade state used by the sell-side rebalance projection.
  pub fn projected_rebalance_sell_state(
    &self,
    usdc_amount: UFix64<N6>,
  ) -> Result<ProjectedState, CoreError> {
    let spot_price = self.collateral_oracle_price().spot;
    let collateral_delta = ExoConversion::spot(spot_price)
      .token_to_exo(usdc_amount, UFix64::one())?;
    let total_collateral = self
      .total_collateral
      .checked_sub(&collateral_delta)
      .ok_or(RebalanceAmountExceeded)?;
    let stablecoin_delta = ExoConversion::spot(spot_price)
      .exo_to_token(collateral_delta, self.stablecoin_nav()?)?;
    let stablecoin_supply = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_delta)
      .ok_or(DestinationStablecoin)?;
    let collateral_ratio =
      CollateralRatio::new(total_collateral, spot_price, stablecoin_supply)?;
    Ok(ProjectedState {
      total_collateral,
      stablecoin_supply,
      collateral_ratio,
    })
  }

  /// Maximum USDC input for a sell-side rebalancing swap.
  ///
  /// # Errors
  /// * Virtual stablecoin is below the floor
  /// * Arithmetic overflow
  pub fn max_rebalance_sell_usdc(
    &self,
    virtual_stablecoin_supply_floor: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    // Collateral the protocol can sell, priced as USDC at spot
    let sellable_collateral =
      self.rebalance_sell_liquidity()?.min(self.total_collateral);
    let spot_price = self.collateral_oracle_price().spot;
    let usdc_in_raw = ExoConversion::spot(spot_price)
      .exo_to_token(sellable_collateral, UFix64::one())?;

    let usdc_limit = self
      .virtual_stablecoin_supply()?
      .checked_sub(&virtual_stablecoin_supply_floor)
      .ok_or(VirtualStablecoinBurnLimit)?;

    Ok(usdc_in_raw.min(usdc_limit))
  }

  /// Builds conversion for buy side rebalancing
  ///
  /// # Errors
  /// * Curve setup, pricing, or projection overflow
  pub fn rebalance_buy_conversion(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Result<ExoConversion, CoreError> {
    let projected = self.projected_rebalance_buy_state(collateral_amount)?;
    let collateral_usd_price = self
      .rebalance_buy_curve()?
      .price(projected.collateral_ratio)?;
    Ok(ExoConversion::spot(collateral_usd_price))
  }

  /// Post-trade state used by the buy-side rebalance projection.
  pub fn projected_rebalance_buy_state(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let spot_price = self.collateral_oracle_price().spot;
    let total_collateral = self
      .total_collateral
      .checked_add(&collateral_amount)
      .ok_or(DestinationCollateral)?;
    let stablecoin_delta = ExoConversion::spot(spot_price)
      .exo_to_token(collateral_amount, self.stablecoin_nav()?)?;
    let stablecoin_supply = self
      .virtual_stablecoin_supply()?
      .checked_add(&stablecoin_delta)
      .ok_or(DestinationStablecoin)?;
    let collateral_ratio =
      CollateralRatio::new(total_collateral, spot_price, stablecoin_supply)?;
    Ok(ProjectedState {
      total_collateral,
      stablecoin_supply,
      collateral_ratio,
    })
  }

  /// Builds the levercoin market cap limiter from exchange inputs.
  ///
  /// # Errors
  /// * Levercoin supply not set
  /// * Levercoin mint NAV fails
  pub fn levercoin_market_cap_limiter(
    &self,
  ) -> Result<LevercoinMarketCapLimiter, CoreError> {
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
  ) -> Result<UFix64<N6>, CoreError> {
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
  ) -> Result<RebalancePnl, CoreError> {
    let stablecoin_value_in = self.exo_to_stablecoin_spot(exo_in)?;
    RebalancePnl::from_stablecoin_flow(stablecoin_value_in, stablecoin_moved)
      .ok_or(RebalanceSwapPnl)
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
  ) -> Result<RebalancePnl, CoreError> {
    let stablecoin_value_out = self.exo_to_stablecoin_spot(exo_out)?;
    RebalancePnl::from_stablecoin_flow(stablecoin_moved, stablecoin_value_out)
      .ok_or(RebalanceSwapPnl)
  }
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::Clock;

  use super::*;
  use crate::error::CoreError::RequestedStablecoinOverMaxMintable;
  use crate::fees::controller::FeePair;

  /// Mainnet xONYC pair at slot 469352016, CR 1.6988 after a levercoin mint.
  fn context() -> Result<ExoExchangeContext<Clock>, CoreError> {
    let total_collateral = UFix64::new(2_613_846_088_166);
    let collateral_oracle = OraclePrice {
      spot: UFix64::new(1_165_813_250),
      conf: UFix64::new(10),
    };
    let collateral_usd_price = collateral_oracle.price_range()?;
    let mut virtual_stablecoin = VirtualStablecoin::new();
    virtual_stablecoin.mint(UFix64::new(1_793_794_979))?;
    let collateral_ratio = CollateralRatio::new(
      total_collateral,
      collateral_usd_price.lower,
      virtual_stablecoin.supply()?,
    )?;
    let fees = |redeem| {
      FeePair::new(UFixValue64::new(0, -4), UFixValue64::new(redeem, -4))
    };
    let curve = |floor, ceil| {
      RebalanceCurveConfig::new(
        UFixValue64::new(floor, -9),
        UFixValue64::new(ceil, -9),
      )
    };
    Ok(ExoExchangeContext {
      clock: Clock::default(),
      total_collateral,
      collateral_oracle,
      collateral_usd_price,
      virtual_stablecoin,
      levercoin_supply: Some(UFix64::new(1_250_403_756)),
      collateral_ratio,
      stablecoin_mint_threshold: UFix64::new(1_500_000_000),
      rebalance_mode: RebalanceMode::from_cr(collateral_ratio),
      levercoin_fees: LevercoinFees {
        normal: fees(20),
        sell_zone_1: fees(20),
        sell_zone_2: fees(400),
      },
      stablecoin_mint_fees: InterpolatedMintFees::new(mint_fee_curve()?),
      stablecoin_redeem_fees: InterpolatedRedeemFees::new(redeem_fee_curve()?),
      sell_curve_config: curve(1_500_000, 200_000),
      buy_curve_config: curve(1, 200_000),
      levercoin_market_cap_limit: UFix64::new(1_000_000_000_000_000),
    })
  }

  #[test]
  fn swap_caps_at_mint_threshold() -> Result<(), CoreError> {
    let context = context()?;
    assert!(context.lever_to_stable_conversion_enabled());
    let max = context.max_swappable_stablecoin()?;
    assert_eq!(max, UFix64::new(237_709_272));
    let supply = context.virtual_stablecoin_supply()?.checked_add(&max);
    let projected = supply.map(|supply| {
      CollateralRatio::new(
        context.total_collateral,
        context.collateral_usd_price.lower,
        supply,
      )
    });
    assert_eq!(
      projected.map(|cr| cr.map(|cr| cr.at_least(UFix64::new(1_500_000_000)))),
      Some(Ok(true)),
    );
    Ok(())
  }

  /// This swap executed on mainnet and left the pair at CR 1.4910.
  #[test]
  fn swap_rejects_below_mint_threshold() -> Result<(), CoreError> {
    assert_eq!(
      context()?.validate_stablecoin_swap_amount(UFix64::new(249_991_704)),
      Err(RequestedStablecoinOverMaxMintable),
    );
    Ok(())
  }
}
