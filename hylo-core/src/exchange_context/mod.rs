//! Exchange context trait and implementations.
//!
//! [`ExchangeContext`] abstracts over collateral source and provides
//! default implementations for NAVs, rebalance modes, swap fees, and
//! validations.

mod exo;
mod lst;
#[cfg(feature = "offchain")]
pub mod marginal;

use fix::prelude::*;

pub use self::exo::ExoExchangeContext;
pub use self::lst::LstExchangeContext;
use crate::conversion::SwapConversion;
use crate::error::CoreError;
#[cfg(any(test, feature = "offchain"))]
use crate::error::CoreError::{CollateralRatio, DestinationCollateral};
use crate::error::CoreError::{
  DestinationStablecoin, LevercoinNav, MaxMintable, MaxSwappable,
  RebalanceBuySideTarget, RebalanceSellSideLiquidity,
  RequestedStablecoinOverMaxMintable, VirtualStablecoinOverhang,
  VirtualStablecoinSurplus,
};
use crate::exchange_math::{
  collateral_ratio, depeg_stablecoin_nav, levercoin_market_cap,
  max_mintable_stablecoin, max_swappable_stablecoin, next_levercoin_mint_nav,
  next_levercoin_redeem_nav, total_value_locked,
};
use crate::fees::controller::{FeeExtract, LevercoinFees};
use crate::pyth::{OraclePrice, PriceRange};
use crate::rebalance::math::{
  max_buyable_collateral, max_sellable_collateral, midpoint,
};
use crate::rebalance::mode::RebalanceMode;
use crate::rebalance::pricing::{
  BuyPriceCurve, RebalanceCurveConfig, RebalancePriceController, SellPriceCurve,
};
#[cfg(any(test, feature = "offchain"))]
use crate::util::max_scaled_input;

/// Post-trade totals and collateral ratio from a fee projection.
/// Totals feed the offchain marginal rate math.
#[cfg_attr(not(feature = "offchain"), allow(dead_code))]
pub(crate) struct ProjectedState {
  pub total_collateral: UFix64<N9>,
  pub stablecoin_supply: UFix64<N6>,
  pub collateral_ratio: UFix64<N9>,
}

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
  fn stablecoin_mint_threshold(&self) -> UFix64<N9>;

  /// Confirm stablecoin mint capability based on configured normal mode CR.
  fn stablecoin_mint_enabled(&self) -> bool {
    self.collateral_ratio() >= self.stablecoin_mint_threshold()
  }

  /// Confirm levercoin mint capability; disabled only during Depeg.
  fn levercoin_mint_enabled(&self) -> bool {
    self.rebalance_mode() != RebalanceMode::Depeg
  }

  /// Sell-side rebalance price curve from oracle spot.
  ///
  /// # Errors
  /// * Curve construction failure
  fn rebalance_sell_curve(&self) -> Result<SellPriceCurve, CoreError> {
    SellPriceCurve::new(
      self.collateral_oracle_price(),
      self.sell_curve_config(),
    )
  }

  /// Buy-side rebalance price curve from oracle spot.
  ///
  /// # Errors
  /// * Curve construction failure
  fn rebalance_buy_curve(&self) -> Result<BuyPriceCurve, CoreError> {
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
  fn rebalance_sell_liquidity(&self) -> Result<UFix64<N9>, CoreError> {
    let target_cr = RebalanceMode::Neutral.active_range().start()?;
    let virtual_stablecoin = self.virtual_stablecoin_supply()?;
    let collateral_usd_price = self.collateral_oracle_price().spot;
    let total_collateral = self.total_collateral();
    max_sellable_collateral(
      target_cr,
      virtual_stablecoin,
      collateral_usd_price,
      total_collateral,
    )
    .ok_or(RebalanceSellSideLiquidity)
  }

  /// Collateral the protocol is willing to buy for CR rebalancing.
  ///
  /// # Errors
  /// * Arithmetic or invalid stablecoin supply
  fn rebalance_buy_target(&self) -> Result<UFix64<N9>, CoreError> {
    let target_cr = RebalanceMode::BuyZone1.active_range().start()?;
    let virtual_stablecoin = self.virtual_stablecoin_supply()?;
    let collateral_usd_price = self.collateral_oracle_price().spot;
    let total_collateral = self.total_collateral();
    max_buyable_collateral(
      target_cr,
      virtual_stablecoin,
      collateral_usd_price,
      total_collateral,
    )
    .ok_or(RebalanceBuySideTarget)
  }

  /// Target collateral ratio for a buy-side rebalance.
  ///
  /// # Errors
  /// * Mode is not a buy zone
  /// * Curve or arithmetic failure
  fn rebalance_buy_target_cr(&self) -> Result<UFix64<N9>, CoreError> {
    match self.rebalance_mode() {
      RebalanceMode::BuyZone2 => RebalanceMode::BuyZone1.active_range().end(),
      RebalanceMode::BuyZone1 => {
        let spot = self.collateral_oracle_price().spot;
        let cr_spot = self.rebalance_buy_curve()?.cr_at_price(spot)?;
        midpoint(self.collateral_ratio(), cr_spot).ok_or(RebalanceBuySideTarget)
      }
      _ => Err(RebalanceBuySideTarget),
    }
  }

  /// Collateral to buy at a premium to spot.
  ///
  /// # Errors
  /// * Mode is not a buy zone
  /// * Curve or arithmetic failure
  fn rebalance_buy_amount_at_premium(&self) -> Result<UFix64<N9>, CoreError> {
    max_buyable_collateral(
      self.rebalance_buy_target_cr()?,
      self.virtual_stablecoin_supply()?,
      self.collateral_oracle_price().spot,
      self.total_collateral(),
    )
    .ok_or(RebalanceBuySideTarget)
  }

  /// Target collateral ratio for a sell-side rebalance.
  ///
  /// # Errors
  /// * Mode is not a sell zone
  /// * Curve or arithmetic failure
  fn rebalance_sell_target_cr(&self) -> Result<UFix64<N9>, CoreError> {
    match self.rebalance_mode() {
      RebalanceMode::SellZone2 => {
        RebalanceMode::SellZone1.active_range().start()
      }
      RebalanceMode::SellZone1 => {
        let spot = self.collateral_oracle_price().spot;
        let cr_spot = self.rebalance_sell_curve()?.cr_at_price(spot)?;
        midpoint(self.collateral_ratio(), cr_spot)
          .ok_or(RebalanceSellSideLiquidity)
      }
      _ => Err(RebalanceSellSideLiquidity),
    }
  }

  /// Collateral to sell at a discount to spot.
  ///
  /// # Errors
  /// * Mode is not a sell zone
  /// * Curve or arithmetic failure
  fn rebalance_sell_amount_at_discount(&self) -> Result<UFix64<N9>, CoreError> {
    max_sellable_collateral(
      self.rebalance_sell_target_cr()?,
      self.virtual_stablecoin_supply()?,
      self.collateral_oracle_price().spot,
      self.total_collateral(),
    )
    .ok_or(RebalanceSellSideLiquidity)
  }

  /// Virtual stablecoin supply.
  fn virtual_stablecoin_supply(&self) -> Result<UFix64<N6>, CoreError>;

  /// Current levercoin supply.
  fn levercoin_supply(&self) -> Result<UFix64<N6>, CoreError>;

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
  fn total_value_locked(&self) -> Result<UFix64<N9>, CoreError> {
    total_value_locked(
      self.total_collateral(),
      self.collateral_usd_price().lower,
    )
  }

  /// Current levercoin market cap in USD.
  ///
  /// # Errors
  /// * Missing supply, NAV failure, or arithmetic overflow
  fn levercoin_market_cap(&self) -> Result<UFix64<N9>, CoreError> {
    levercoin_market_cap(self.levercoin_supply()?, self.levercoin_mint_nav()?)
  }

  /// Stablecoin NAV — $1 in all modes except Depeg.
  ///
  /// # Errors
  /// * Arithmetic failure in depeg path
  fn stablecoin_nav(&self) -> Result<UFix64<N9>, CoreError> {
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
  fn levercoin_mint_nav(&self) -> Result<UFix64<N9>, CoreError> {
    next_levercoin_mint_nav(
      self.total_collateral(),
      self.collateral_usd_price(),
      self.virtual_stablecoin_supply()?,
      self.stablecoin_nav()?,
      self.levercoin_supply()?,
    )
    .ok_or(LevercoinNav)
  }

  /// Lower-bound levercoin NAV for redemption.
  ///
  /// # Errors
  /// * Missing supply or arithmetic failure
  fn levercoin_redeem_nav(&self) -> Result<UFix64<N9>, CoreError> {
    next_levercoin_redeem_nav(
      self.total_collateral(),
      self.collateral_usd_price(),
      self.virtual_stablecoin_supply()?,
      self.stablecoin_nav()?,
      self.levercoin_supply()?,
    )
    .ok_or(LevercoinNav)
  }

  /// Delta of current virtual stablecoin supply and TVL.
  ///
  /// # Errors
  /// * Virtual stablecoin not depegged
  /// * Underflow
  fn virtual_stablecoin_overhang(&self) -> Result<UFix64<N6>, CoreError> {
    let tvl = self.total_value_locked()?;
    let virtual_stablecoin = self.virtual_stablecoin_supply()?;
    tvl
      .checked_convert::<N6>()
      .and_then(|tvl| virtual_stablecoin.checked_sub(&tvl))
      .ok_or(VirtualStablecoinOverhang)
  }

  /// Delta of TVL and current virtual stablecoin supply.
  ///
  /// # Errors
  /// * Virtual stablecoin in depeg
  /// * Underflow
  fn virtual_stablecoin_surplus(&self) -> Result<UFix64<N6>, CoreError> {
    let tvl = self.total_value_locked()?;
    let virtual_stablecoin = self.virtual_stablecoin_supply()?;
    tvl
      .checked_convert::<N6>()
      .and_then(|tvl| tvl.checked_sub(&virtual_stablecoin))
      .ok_or(VirtualStablecoinSurplus)
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
  ) -> Result<RebalanceMode, CoreError> {
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price().lower,
      new_stablecoin,
    )?;
    Ok(RebalanceMode::from_cr(projected_cr))
  }

  /// Returns the worse of current vs projected mode for fee
  /// purposes. Transactions that improve CR only pay fees at
  /// the current mode; transactions that worsen CR pay at the
  /// projected mode.
  fn select_rebalance_mode_for_fees(
    &self,
    projected: RebalanceMode,
  ) -> RebalanceMode {
    projected.min(self.rebalance_mode())
  }

  /// Swap conversion between stablecoin and levercoin NAVs.
  ///
  /// # Errors
  /// * NAV computation failure
  fn swap_conversion(&self) -> Result<SwapConversion, CoreError> {
    let levercoin_nav =
      PriceRange::new(self.levercoin_redeem_nav()?, self.levercoin_mint_nav()?);
    Ok(SwapConversion::new(self.stablecoin_nav()?, levercoin_nav))
  }

  /// Maximum mintable stablecoin before hitting the configured
  /// stablecoin mint threshold.
  ///
  /// # Errors
  /// * Arithmetic overflow
  fn max_mintable_stablecoin(&self) -> Result<UFix64<N6>, CoreError> {
    let target = self
      .stablecoin_mint_threshold()
      .checked_convert()
      .ok_or(MaxMintable)?;
    max_mintable_stablecoin(
      target,
      self.total_collateral(),
      self.collateral_usd_price().lower,
      self.virtual_stablecoin_supply()?,
    )
  }

  /// Maximum stablecoin swappable from levercoin before hitting the
  /// lowest CR threshold.
  ///
  /// # Errors
  /// * TVL computation or arithmetic failure
  fn max_swappable_stablecoin(&self) -> Result<UFix64<N6>, CoreError> {
    let target = RebalanceMode::SellZone1
      .active_range()
      .end()?
      .checked_convert()
      .ok_or(MaxSwappable)?;
    max_swappable_stablecoin(
      target,
      self.total_value_locked()?,
      self.virtual_stablecoin_supply()?,
    )
  }

  /// Collateral removable before the projected rebalance mode reaches
  /// [`RebalanceMode::Depeg`].
  ///
  /// # Errors
  /// * Arithmetic overflow
  /// * Current state already below the Depeg exit
  #[cfg(any(test, feature = "offchain"))]
  fn max_collateral_removal(&self) -> Result<UFix64<N9>, CoreError> {
    let supply = self.virtual_stablecoin_supply()?;
    if supply == UFix64::zero() {
      Ok(self.total_collateral())
    } else {
      let atom = UFix64::new(1);
      let last_depeg_cr = RebalanceMode::SellZone2
        .active_range()
        .start()?
        .checked_sub(&atom)
        .ok_or(CollateralRatio)?;
      let min_collateral = supply
        .checked_convert::<N9>()
        .and_then(|supply| {
          max_scaled_input(
            last_depeg_cr,
            self.collateral_usd_price().lower,
            supply,
          )
        })
        .and_then(|last_depeg_collateral| {
          last_depeg_collateral.checked_add(&atom)
        })
        .ok_or(CollateralRatio)?;
      self
        .total_collateral()
        .checked_sub(&min_collateral)
        .ok_or(DestinationCollateral)
    }
  }

  /// Stablecoin removable before the projected collateral ratio
  /// overflows its representation.
  ///
  /// # Errors
  /// * Arithmetic overflow
  #[cfg(any(test, feature = "offchain"))]
  fn max_stablecoin_removal(&self) -> Result<UFix64<N6>, CoreError> {
    let min_supply = self
      .total_collateral()
      .mul_div_ceil(
        self.collateral_usd_price().lower,
        UFix64::<N9>::new(u64::MAX),
      )
      .and_then(UFix64::checked_convert_ceil::<N6>)
      .ok_or(CollateralRatio)?;
    Ok(
      self
        .virtual_stablecoin_supply()?
        .checked_sub(&min_supply)
        .unwrap_or_default(),
    )
  }

  /// Validates a stablecoin mint amount against the protocol max.
  ///
  /// # Errors
  /// * Amount exceeds max mintable
  fn validate_stablecoin_amount(
    &self,
    requested: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let max = self.max_mintable_stablecoin()?;
    if requested <= max {
      Ok(requested)
    } else {
      Err(RequestedStablecoinOverMaxMintable)
    }
  }

  /// Validates `PnL` stablecoin profit against `SellZone2` threshold.
  ///
  /// # Errors
  /// * Arithmetic overflow
  fn validate_stablecoin_pnl_profit(
    &self,
    requested: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let target = RebalanceMode::SellZone2
      .active_range()
      .end()?
      .checked_convert()
      .ok_or(MaxMintable)?;
    let max = max_swappable_stablecoin(
      target,
      self.total_value_locked()?,
      self.virtual_stablecoin_supply()?,
    )
    .unwrap_or_default();
    Ok(requested.min(max))
  }

  /// Validates a stablecoin swap amount against the protocol max.
  ///
  /// # Errors
  /// * Amount exceeds max swappable
  fn validate_stablecoin_swap_amount(
    &self,
    requested: UFix64<N6>,
  ) -> Result<UFix64<N6>, CoreError> {
    let max = self.max_swappable_stablecoin()?;
    if requested <= max {
      Ok(requested)
    } else {
      Err(RequestedStablecoinOverMaxMintable)
    }
  }

  /// Swap fee for levercoin-to-stablecoin direction.
  ///
  /// # Errors
  /// * Projection overflow or mode-based fee lookup
  fn levercoin_to_stablecoin_fee(
    &self,
    amount_stablecoin_out: UFix64<N6>,
  ) -> Result<FeeExtract<N6>, CoreError> {
    let rate = self.levercoin_to_stablecoin_fee_rate(amount_stablecoin_out)?;
    FeeExtract::new(rate, amount_stablecoin_out)
  }

  /// Mode-based fee rate for the levercoin-to-stablecoin direction.
  ///
  /// # Errors
  /// * Projection overflow or mode-based fee lookup
  fn levercoin_to_stablecoin_fee_rate(
    &self,
    amount_stablecoin_out: UFix64<N6>,
  ) -> Result<UFix64<N4>, CoreError> {
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_add(&amount_stablecoin_out)
      .ok_or(DestinationStablecoin)?;
    let projected =
      self.projected_rebalance_mode(self.total_collateral(), new_stablecoin)?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    self.levercoin_fees().convert_to_stablecoin_fee(mode)
  }

  /// Swap fee for stablecoin-to-levercoin direction.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  fn stablecoin_to_levercoin_fee(
    &self,
    amount_stablecoin_in: UFix64<N6>,
  ) -> Result<FeeExtract<N6>, CoreError> {
    let rate = self.stablecoin_to_levercoin_fee_rate(amount_stablecoin_in)?;
    FeeExtract::new(rate, amount_stablecoin_in)
  }

  /// Mode-based fee rate for the stablecoin-to-levercoin direction.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  fn stablecoin_to_levercoin_fee_rate(
    &self,
    amount_stablecoin_in: UFix64<N6>,
  ) -> Result<UFix64<N4>, CoreError> {
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&amount_stablecoin_in)
      .ok_or(DestinationStablecoin)?;
    let projected =
      self.projected_rebalance_mode(self.total_collateral(), new_stablecoin)?;
    let mode = self.select_rebalance_mode_for_fees(projected);
    self.levercoin_fees().convert_from_stablecoin_fee(mode)
  }
}
