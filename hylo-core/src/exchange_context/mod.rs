//! Exchange context trait and implementations.
//!
//! [`ExchangeContext`] abstracts over collateral source and provides
//! default implementations for NAVs, rebalance modes, swap fees, and
//! validations.

mod exo;
mod lst;

use anchor_lang::prelude::*;
use fix::prelude::*;

pub use self::exo::ExoExchangeContext;
pub use self::lst::LstExchangeContext;
use crate::conversion::SwapConversion;
use crate::error::CoreError::{
  DestinationStablecoin, LevercoinNav, MaxMintable, MaxSwappable,
  RebalanceBuySideTarget, RebalanceSellSideLiquidity,
  RequestedStablecoinOverMaxMintable,
};
use crate::exchange_math::{
  collateral_ratio, depeg_stablecoin_nav, max_mintable_stablecoin,
  max_swappable_stablecoin, next_levercoin_mint_nav, next_levercoin_redeem_nav,
  total_value_locked,
};
use crate::fee_controller::{FeeExtract, LevercoinFees};
use crate::pyth::{OraclePrice, PriceRange};
use crate::rebalance_math::{max_buyable_collateral, max_sellable_collateral};
use crate::rebalance_mode::RebalanceMode;
use crate::rebalance_pricing::{
  BuyPriceCurve, RebalanceCurveConfig, RebalancePriceController, SellPriceCurve,
};

/// Shared interface for exchange context implementations.
pub trait ExchangeContext {
  /// Total collateral in N9 precision.
  fn total_collateral(&self) -> UFix64<N9>;

  /// Collateral/USD oracle price range.
  fn collateral_usd_price(&self) -> PriceRange<N9>;

  /// Raw oracle spot + confidence.
  fn collateral_oracle_price(&self) -> OraclePrice;

  /// Sell-side rebalance curve configuration.
  fn sell_curve_config(&self) -> &RebalanceCurveConfig;

  /// Buy-side rebalance curve configuration.
  fn buy_curve_config(&self) -> &RebalanceCurveConfig;

  /// Collateral ratio defining target leverage and stablecoin mint ability.
  fn mint_threshold(&self) -> UFix64<N9>;

  /// Confirm stablecoin mint capability based on configured normal mode CR.
  fn mint_enabled(&self) -> bool {
    self.collateral_ratio() >= self.mint_threshold()
  }

  /// Sell-side rebalance price curve from oracle confidence.
  ///
  /// # Errors
  /// * Curve construction failure
  fn rebalance_sell_curve(&self) -> Result<SellPriceCurve> {
    SellPriceCurve::new(
      self.collateral_oracle_price(),
      self.sell_curve_config(),
    )
  }

  /// Buy-side rebalance price curve from oracle confidence.
  ///
  /// # Errors
  /// * Curve construction failure
  fn rebalance_buy_curve(&self) -> Result<BuyPriceCurve> {
    BuyPriceCurve::new(self.collateral_oracle_price(), self.buy_curve_config())
  }

  /// Returns true if sell-side rebalancing is active at the current CR.
  fn rebalance_sell_active(&self) -> bool {
    self
      .rebalance_sell_curve()
      .is_ok_and(|c| c.is_active(self.collateral_ratio()))
  }

  /// Returns true if buy-side rebalancing is active at the current CR.
  fn rebalance_buy_active(&self) -> bool {
    self
      .rebalance_buy_curve()
      .is_ok_and(|c| c.is_active(self.collateral_ratio()))
  }

  /// Available collateral liquidity to sell off for CR rebalancing.
  ///
  /// # Errors
  /// * Arithmetic or invalid stablecoin supply
  fn rebalance_sell_liquidity(&self) -> Result<UFix64<N9>> {
    let target_cr = RebalanceMode::Neutral.active_range().start;
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

  /// Collateral the protocol is willing to buy for CR rebalancing.
  ///
  /// # Errors
  /// * Arithmetic or invalid stablecoin supply
  fn rebalance_buy_target(&self) -> Result<UFix64<N9>> {
    let target_cr = RebalanceMode::BuyZone1.active_range().start;
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

  /// Virtual stablecoin supply.
  fn virtual_stablecoin_supply(&self) -> Result<UFix64<N6>>;

  /// Current levercoin supply.
  fn levercoin_supply(&self) -> Result<UFix64<N6>>;

  /// Current rebalance mode, computed at construction.
  fn rebalance_mode(&self) -> RebalanceMode;

  /// Cached collateral ratio, computed at construction.
  fn collateral_ratio(&self) -> UFix64<N9>;

  /// Levercoin fee configuration.
  fn levercoin_fees(&self) -> &LevercoinFees;

  /// TVL in USD.
  ///
  /// # Errors
  /// * Arithmetic overflow
  fn total_value_locked(&self) -> Result<UFix64<N9>> {
    total_value_locked(
      self.total_collateral(),
      self.collateral_usd_price().lower,
    )
  }

  /// Stablecoin NAV — $1 in all modes except Depeg.
  ///
  /// # Errors
  /// * Arithmetic failure in depeg path
  fn stablecoin_nav(&self) -> Result<UFix64<N9>> {
    match self.rebalance_mode() {
      RebalanceMode::Depeg => depeg_stablecoin_nav(
        self.total_collateral(),
        self.collateral_usd_price().lower,
        self.virtual_stablecoin_supply()?,
      ),
      _ => Ok(UFix64::one()),
    }
  }

  /// Upper-bound levercoin NAV for minting.
  ///
  /// # Errors
  /// * Missing supply or arithmetic failure
  fn levercoin_mint_nav(&self) -> Result<UFix64<N9>> {
    next_levercoin_mint_nav(
      self.total_collateral(),
      self.collateral_usd_price(),
      self.virtual_stablecoin_supply()?,
      self.stablecoin_nav()?,
      self.levercoin_supply()?,
    )
    .ok_or(LevercoinNav.into())
  }

  /// Lower-bound levercoin NAV for redemption.
  ///
  /// # Errors
  /// * Missing supply or arithmetic failure
  fn levercoin_redeem_nav(&self) -> Result<UFix64<N9>> {
    next_levercoin_redeem_nav(
      self.total_collateral(),
      self.collateral_usd_price(),
      self.virtual_stablecoin_supply()?,
      self.stablecoin_nav()?,
      self.levercoin_supply()?,
    )
    .ok_or(LevercoinNav.into())
  }

  /// Projects rebalance mode after changing collateral and stablecoin
  /// totals.
  ///
  /// # Errors
  /// * Collateral ratio computation failure
  fn projected_rebalance_mode(
    &self,
    new_total: UFix64<N9>,
    new_stablecoin: UFix64<N6>,
  ) -> Result<RebalanceMode> {
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price().lower,
      new_stablecoin,
    )?;
    Ok(RebalanceMode::from_cr(projected_cr))
  }

  /// Returns the worse of current vs projected mode for fee
  /// purposes. Transactions that improve CR only pay fees at
  /// the current mode.
  fn select_rebalance_mode_for_fees(
    &self,
    projected: RebalanceMode,
  ) -> RebalanceMode {
    if projected < self.rebalance_mode() {
      self.rebalance_mode()
    } else {
      projected
    }
  }

  /// Swap conversion between stablecoin and levercoin NAVs.
  ///
  /// # Errors
  /// * NAV computation failure
  fn swap_conversion(&self) -> Result<SwapConversion> {
    let levercoin_nav =
      PriceRange::new(self.levercoin_redeem_nav()?, self.levercoin_mint_nav()?);
    Ok(SwapConversion::new(self.stablecoin_nav()?, levercoin_nav))
  }

  /// Maximum mintable stablecoin before hitting the lowest CR
  /// threshold.
  ///
  /// # Errors
  /// * Arithmetic overflow
  fn max_mintable_stablecoin(&self) -> Result<UFix64<N6>> {
    let target = RebalanceMode::SellZone1
      .active_range()
      .end
      .checked_convert()
      .ok_or(MaxMintable)?;
    max_mintable_stablecoin(
      target,
      self.total_collateral(),
      self.collateral_usd_price().upper,
      self.virtual_stablecoin_supply()?,
    )
  }

  /// Maximum stablecoin swappable from levercoin before hitting the
  /// lowest CR threshold.
  ///
  /// # Errors
  /// * TVL computation or arithmetic failure
  fn max_swappable_stablecoin(&self) -> Result<UFix64<N6>> {
    let target = RebalanceMode::SellZone1
      .active_range()
      .end
      .checked_convert()
      .ok_or(MaxSwappable)?;
    max_swappable_stablecoin(
      target,
      self.total_value_locked()?,
      self.virtual_stablecoin_supply()?,
    )
  }

  /// Validates a stablecoin mint amount against the protocol max.
  ///
  /// # Errors
  /// * Amount exceeds max mintable
  fn validate_stablecoin_amount(
    &self,
    requested: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    let max = self.max_mintable_stablecoin()?;
    if requested <= max {
      Ok(requested)
    } else {
      Err(RequestedStablecoinOverMaxMintable.into())
    }
  }

  /// Validates a stablecoin swap amount against the protocol max.
  ///
  /// # Errors
  /// * Amount exceeds max swappable
  fn validate_stablecoin_swap_amount(
    &self,
    requested: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    let max = self.max_swappable_stablecoin()?;
    if requested <= max {
      Ok(requested)
    } else {
      Err(RequestedStablecoinOverMaxMintable.into())
    }
  }

  /// Swap fee for levercoin-to-stablecoin direction.
  ///
  /// # Errors
  /// * Projection overflow or mode-based fee lookup
  fn levercoin_to_stablecoin_fee(
    &self,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<FeeExtract<N6>> {
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_add(&amount_stablecoin)
      .ok_or(DestinationStablecoin)?;
    let projected =
      self.projected_rebalance_mode(self.total_collateral(), new_stablecoin)?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    let fee = self.levercoin_fees().convert_to_stablecoin_fee(mode)?;
    FeeExtract::new(fee, amount_stablecoin)
  }

  /// Swap fee for stablecoin-to-levercoin direction.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  fn stablecoin_to_levercoin_fee(
    &self,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<FeeExtract<N6>> {
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&amount_stablecoin)
      .ok_or(DestinationStablecoin)?;
    let projected =
      self.projected_rebalance_mode(self.total_collateral(), new_stablecoin)?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    let fee = self.levercoin_fees().convert_from_stablecoin_fee(mode)?;
    FeeExtract::new(fee, amount_stablecoin)
  }
}
