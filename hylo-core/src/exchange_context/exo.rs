use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fix::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use super::{validate_stability_thresholds, ExchangeContext};
use crate::conversion::{ExoConversion, ExoRebalanceConversion};
use crate::error::CoreError::{
  ExoDestinationCollateral, ExoDestinationStablecoin, LevercoinNav,
  RebalanceBuySideTarget, RebalanceSellSideLiquidity,
};
use crate::exchange_math::collateral_ratio;
use crate::fee_controller::{FeeController, FeeExtract, LevercoinFees};
use crate::fee_curves::{mint_fee_curve, redeem_fee_curve};
use crate::interpolated_fees::{
  InterpolatedFeeController, InterpolatedMintFees, InterpolatedRedeemFees,
};
use crate::pyth::{query_pyth_oracle, OracleConfig, OraclePrice, PriceRange};
use crate::rebalance_math::{max_buyable_collateral, max_sellable_collateral};
use crate::rebalance_pricing::{
  RebalanceCurveConfig, RebalancePriceController,
};
use crate::solana_clock::SolanaClock;
use crate::stability_mode::{StabilityController, StabilityMode};
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
  stability_mode: StabilityMode,
  pub stability_controller: StabilityController,
  levercoin_fees: LevercoinFees,
  stablecoin_mint_fees: InterpolatedMintFees,
  stablecoin_redeem_fees: InterpolatedRedeemFees,
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

impl<C: SolanaClock> ExoExchangeContext<C> {
  /// Builds context from account data.
  ///
  /// # Errors
  /// * Oracle, curve, or stability controller validation
  #[allow(clippy::too_many_arguments)]
  pub fn load(
    clock: C,
    total_collateral: UFix64<N9>,
    stability_threshold_1: UFix64<N2>,
    oracle_config: OracleConfig,
    levercoin_fees: LevercoinFees,
    collateral_usd_pyth_feed: &PriceUpdateV2,
    virtual_stablecoin: VirtualStablecoin,
    levercoin_mint: Option<&Mint>,
  ) -> Result<ExoExchangeContext<C>> {
    let collateral_oracle =
      query_pyth_oracle(&clock, collateral_usd_pyth_feed, oracle_config)?;
    let collateral_usd_price = collateral_oracle.price_range()?;
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
    let levercoin_supply = levercoin_mint.map(|m| UFix64::new(m.supply));
    let stablecoin_supply = virtual_stablecoin.supply()?;
    let collateral_ratio = collateral_ratio(
      total_collateral,
      collateral_usd_price.lower,
      stablecoin_supply,
    )?;
    let stability_mode =
      stability_controller.stability_mode(collateral_ratio)?;
    Ok(ExoExchangeContext {
      clock,
      total_collateral,
      collateral_oracle,
      collateral_usd_price,
      virtual_stablecoin,
      levercoin_supply,
      collateral_ratio,
      stability_mode,
      stability_controller,
      levercoin_fees,
      stablecoin_mint_fees,
      stablecoin_redeem_fees,
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
      .ok_or(ExoDestinationCollateral)?;
    let stablecoin_minted = self
      .exo_conversion()
      .exo_to_token(collateral_amount, self.stablecoin_nav()?)?;
    let new_stablecoin = stablecoin_minted
      .checked_add(&self.virtual_stablecoin_supply()?)
      .ok_or(ExoDestinationStablecoin)?;
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
      .ok_or(ExoDestinationCollateral)?;
    let stablecoin_redeemed = self
      .exo_conversion()
      .exo_to_token(collateral_amount, self.stablecoin_nav()?)?;
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_redeemed)
      .ok_or(ExoDestinationStablecoin)?;
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price.lower,
      new_stablecoin,
    )?;
    self
      .stablecoin_redeem_fees
      .apply_fee(projected_cr, collateral_amount)
  }

  /// Levercoin mint fee based on projected stability mode.
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
      .ok_or(ExoDestinationCollateral)?;
    let projected = self
      .projected_stability_mode(new_total, self.virtual_stablecoin_supply()?)?;
    let mode = self.select_stability_mode_for_fees(projected);
    let fee = self.levercoin_fees.mint_fee(mode)?;
    FeeExtract::new(fee, collateral_amount)
  }

  /// Levercoin redeem fee based on projected stability mode.
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
      .ok_or(ExoDestinationCollateral)?;
    let projected = self
      .projected_stability_mode(new_total, self.virtual_stablecoin_supply()?)?;
    let mode = self.select_stability_mode_for_fees(projected);
    let fee = self.levercoin_fees.redeem_fee(mode)?;
    FeeExtract::new(fee, collateral_amount)
  }

  /// Builds conversion helper between exogenous collateral and token.
  #[must_use]
  pub fn exo_conversion(&self) -> ExoConversion {
    ExoConversion {
      collateral_usd_price: self.collateral_usd_price,
    }
  }

  /// Builds conversion for sell side rebalancing
  ///
  /// # Errors
  /// * Curve setup or pricing
  pub fn rebalance_sell_conversion(
    &self,
    config: &RebalanceCurveConfig,
    usdc_usd_price: PriceRange<N9>,
  ) -> Result<ExoRebalanceConversion> {
    let curve = self.rebalance_sell_curve(config)?;
    let collateral_rebalance_usd_price =
      curve.price(self.collateral_ratio())?;
    Ok(ExoRebalanceConversion {
      collateral_rebalance_usd_price,
      usdc_usd_price,
    })
  }

  /// Builds conversion for buy side rebalancing
  ///
  /// # Errors
  /// * Curve setup or pricing
  pub fn rebalance_buy_conversion(
    &self,
    config: &RebalanceCurveConfig,
    usdc_usd_price: PriceRange<N9>,
  ) -> Result<ExoRebalanceConversion> {
    let curve = self.rebalance_buy_curve(config)?;
    let collateral_rebalance_usd_price =
      curve.price(self.collateral_ratio())?;
    Ok(ExoRebalanceConversion {
      collateral_rebalance_usd_price,
      usdc_usd_price,
    })
  }

  /// Determines amount of available collateral liquidity to sell off for CR
  /// rebalancing.
  ///
  /// # Errors
  /// * Arithmetic
  /// * Invalid stablecoin supply
  pub fn rebalance_sell_liquidity(&self) -> Result<UFix64<N9>> {
    let target_cr = self.stability_controller().stability_threshold_1;
    let virtual_stablecoin = self.virtual_stablecoin_supply()?;
    let collateral_usd_price = self.collateral_oracle_price().spot;
    let total_collateral = self.total_collateral();
    max_sellable_collateral(
      target_cr,
      virtual_stablecoin,
      collateral_usd_price,
      total_collateral,
    )
    .ok_or(RebalanceSellSideLiquidity.into())
  }

  /// Determines amount of collateral protocol is willing to buy for CR
  /// rebalancing.
  ///
  /// # Errors
  /// * Arithmetic
  /// * Invalid stablecoin supply
  pub fn rebalance_buy_target(&self) -> Result<UFix64<N9>> {
    let target_cr = self.stability_controller().stability_threshold_1;
    let virtual_stablecoin = self.virtual_stablecoin_supply()?;
    let collateral_usd_price = self.collateral_oracle_price().spot;
    let total_collateral = self.total_collateral();
    max_buyable_collateral(
      target_cr,
      virtual_stablecoin,
      collateral_usd_price,
      total_collateral,
    )
    .ok_or(RebalanceBuySideTarget.into())
  }
}
