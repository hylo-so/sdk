//! Exchange context trait and implementations.
//!
//! [`ExchangeContext`] abstracts over collateral source and provides
//! default implementations for NAVs, stability modes, swap fees, and
//! validations.

mod exo;
mod lst;

use anchor_lang::prelude::*;
use fix::prelude::*;

pub use self::exo::ExoExchangeContext;
pub use self::lst::LstExchangeContext;
use crate::conversion::SwapConversion;
use crate::error::CoreError::{
  DestinationFeeStablecoin, LevercoinNav, RequestedStablecoinOverMaxMintable,
  StabilityValidation,
};
use crate::exchange_math::{
  collateral_ratio, depeg_stablecoin_nav, max_mintable_stablecoin,
  max_swappable_stablecoin, next_levercoin_mint_nav, next_levercoin_redeem_nav,
  total_value_locked,
};
use crate::fee_controller::{FeeExtract, LevercoinFees};
use crate::pyth::PriceRange;
use crate::stability_mode::{StabilityController, StabilityMode};
use crate::stability_pool_math::stability_pool_cap;

/// Ensures ST1 is strictly above ST2 (derived from the redeem fee curve).
///
/// # Errors
/// * Thresholds fail validation
pub fn validate_stability_thresholds(
  stability_threshold_1: UFix64<N2>,
  stability_threshold_2: UFix64<N2>,
) -> Result<()> {
  (stability_threshold_1 > stability_threshold_2)
    .then_some(())
    .ok_or(StabilityValidation.into())
}

/// Shared interface for exchange context implementations.
pub trait ExchangeContext {
  /// Total collateral in N9 precision.
  fn total_collateral(&self) -> UFix64<N9>;

  /// Collateral/USD oracle price range.
  fn collateral_usd_price(&self) -> PriceRange<N8>;

  /// Virtual stablecoin supply.
  fn virtual_stablecoin_supply(&self) -> Result<UFix64<N6>>;

  /// Current levercoin supply.
  fn levercoin_supply(&self) -> Result<UFix64<N6>>;

  /// Stability controller configuration.
  fn stability_controller(&self) -> &StabilityController;

  /// Cached stability mode, computed at construction.
  fn stability_mode(&self) -> StabilityMode;

  /// Cached collateral ratio, computed at construction.
  fn collateral_ratio(&self) -> UFix64<N9>;

  /// Levercoin fee configuration.
  fn levercoin_fees(&self) -> &LevercoinFees;

  /// TVL in USD at N9 precision.
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
    match self.stability_mode() {
      StabilityMode::Depeg => depeg_stablecoin_nav(
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

  /// Projects stability mode after changing collateral and stablecoin
  /// totals.
  ///
  /// # Errors
  /// * Collateral ratio computation failure
  fn projected_stability_mode(
    &self,
    new_total: UFix64<N9>,
    new_stablecoin: UFix64<N6>,
  ) -> Result<StabilityMode> {
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price().lower,
      new_stablecoin,
    )?;
    self.stability_controller().stability_mode(projected_cr)
  }

  /// Returns the worse of current vs projected mode for fee
  /// purposes. Transactions that improve stability only pay fees at
  /// the current mode.
  fn select_stability_mode_for_fees(
    &self,
    projected: StabilityMode,
  ) -> StabilityMode {
    if projected < self.stability_mode() {
      self.stability_mode()
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

  /// Total capitalization of stablecoin and levercoin in stability
  /// pool.
  ///
  /// # Errors
  /// * NAV or arithmetic failure
  fn stability_pool_cap(
    &self,
    stablecoin_in_pool: UFix64<N6>,
    levercoin_in_pool: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    stability_pool_cap(
      self.stablecoin_nav()?,
      stablecoin_in_pool,
      self.levercoin_mint_nav()?,
      levercoin_in_pool,
    )
  }

  /// Maximum mintable stablecoin before hitting the lowest CR
  /// threshold.
  ///
  /// # Errors
  /// * Arithmetic overflow
  fn max_mintable_stablecoin(&self) -> Result<UFix64<N6>> {
    max_mintable_stablecoin(
      self.stability_controller().min_stability_threshold(),
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
    max_swappable_stablecoin(
      self.stability_controller().min_stability_threshold(),
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
      .ok_or(DestinationFeeStablecoin)?;
    let projected =
      self.projected_stability_mode(self.total_collateral(), new_stablecoin)?;
    let mode = self.select_stability_mode_for_fees(projected);
    let fee = self.levercoin_fees().swap_to_stablecoin_fee(mode)?;
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
      .ok_or(DestinationFeeStablecoin)?;
    let projected =
      self.projected_stability_mode(self.total_collateral(), new_stablecoin)?;
    let mode = self.select_stability_mode_for_fees(projected);
    let fee = self.levercoin_fees().swap_from_stablecoin_fee(mode)?;
    FeeExtract::new(fee, amount_stablecoin)
  }
}
