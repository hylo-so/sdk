use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fix::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::conversion::{ExoConversion, SwapConversion};
use crate::error::CoreError::{
  ExoDestinationCollateral, ExoDestinationStablecoin, ExoUpconvert,
  LevercoinNav, RequestedStablecoinOverMaxMintable, StabilityValidation,
};
use crate::exchange_math::{
  collateral_ratio, depeg_stablecoin_nav, max_mintable_stablecoin,
  max_swappable_stablecoin, next_levercoin_mint_nav, next_levercoin_redeem_nav,
  total_value_locked,
};
use crate::fee_controller::{FeeController, FeeExtract, LevercoinFees};
use crate::fee_curves::{mint_fee_curve, redeem_fee_curve};
use crate::interpolated_fees::{
  InterpolatedFeeController, InterpolatedMintFees, InterpolatedRedeemFees,
};
use crate::pyth::{query_pyth_price, OracleConfig, PriceRange};
use crate::solana_clock::SolanaClock;
use crate::stability_mode::{StabilityController, StabilityMode};
use crate::virtual_stablecoin::VirtualStablecoin;

/// Math context for exogenous collateral exchange pairs.
pub struct ExoExchangeContext<C> {
  pub clock: C,
  pub collateral_amount: UFix64<N8>,
  pub collateral_usd_price: PriceRange<N8>,
  pub virtual_stablecoin: VirtualStablecoin,
  levercoin_supply: Option<UFix64<N6>>,
  pub stability_controller: StabilityController,
  levercoin_fees: LevercoinFees,
  mint_fees: InterpolatedMintFees,
  redeem_fees: InterpolatedRedeemFees,
}

impl<C: SolanaClock> ExoExchangeContext<C> {
  /// Builds context from account data.
  ///
  /// # Errors
  /// * Oracle, curve, or stability controller validation
  #[allow(clippy::too_many_arguments)]
  pub fn load(
    clock: C,
    collateral_amount: UFix64<N8>,
    stability_threshold_1: UFix64<N2>,
    oracle_config: OracleConfig<N8>,
    levercoin_fees: LevercoinFees,
    collateral_usd_pyth_feed: &PriceUpdateV2,
    virtual_stablecoin: VirtualStablecoin,
    levercoin_mint: Option<&Mint>,
  ) -> Result<ExoExchangeContext<C>> {
    let collateral_usd_price =
      query_pyth_price(&clock, collateral_usd_pyth_feed, oracle_config)?;
    let mint_fees = InterpolatedMintFees::new(mint_fee_curve()?);
    let redeem_fees = InterpolatedRedeemFees::new(redeem_fee_curve()?);
    let stability_threshold_2 = mint_fees.stability_threshold_2()?;
    Self::validate_stability_thresholds(
      stability_threshold_1,
      stability_threshold_2,
      redeem_fees.stability_threshold_2()?,
    )?;
    let stability_controller =
      StabilityController::new(stability_threshold_1, stability_threshold_2)?;
    let levercoin_supply = levercoin_mint.map(|m| UFix64::new(m.supply));
    Ok(ExoExchangeContext {
      clock,
      collateral_amount,
      collateral_usd_price,
      virtual_stablecoin,
      levercoin_supply,
      stability_controller,
      levercoin_fees,
      mint_fees,
      redeem_fees,
    })
  }

  /// Ensures validity of configured and static stability thresholds.
  ///
  /// * ST1 and ST2 should be in strict order
  /// * ST2 implied by fee curves should be equivalent
  pub fn validate_stability_thresholds(
    stability_threshold_1: UFix64<N2>,
    mint_stability_threshold_2: UFix64<N2>,
    redeem_stability_threshold_2: UFix64<N2>,
  ) -> Result<()> {
    (mint_stability_threshold_2 == redeem_stability_threshold_2
      && stability_threshold_1 > mint_stability_threshold_2)
      .then_some(())
      .ok_or(StabilityValidation.into())
  }

  /// Stablecoin supply from the virtual stablecoin counter.
  ///
  /// # Errors
  /// * Invalid supply data
  pub fn virtual_stablecoin_supply(&self) -> Result<UFix64<N6>> {
    self.virtual_stablecoin.supply()
  }

  /// Upconverts collateral to N9 for exchange math.
  fn total_collateral(&self) -> Result<UFix64<N9>> {
    self
      .collateral_amount
      .checked_convert::<N9>()
      .ok_or(ExoUpconvert.into())
  }

  /// Current collateral ratio of this exo pair.
  ///
  /// # Errors
  /// * Upconversion or arithmetic failure
  pub fn collateral_ratio(&self) -> Result<UFix64<N9>> {
    collateral_ratio(
      self.total_collateral()?,
      self.collateral_usd_price.lower,
      self.virtual_stablecoin_supply()?,
    )
  }

  /// Total value locked in USD for this exo pair.
  ///
  /// # Errors
  /// * Upconversion or arithmetic failure
  pub fn total_value_locked(&self) -> Result<UFix64<N9>> {
    total_value_locked(
      self.total_collateral()?,
      self.collateral_usd_price.lower,
    )
  }

  /// Levercoin supply if a mint was provided at construction.
  ///
  /// # Errors
  /// * No levercoin mint was provided
  pub fn levercoin_supply(&self) -> Result<UFix64<N6>> {
    self.levercoin_supply.ok_or(LevercoinNav.into())
  }

  /// Upper-bound levercoin NAV for minting.
  ///
  /// # Errors
  /// * Missing supply or arithmetic failure
  pub fn levercoin_mint_nav(&self) -> Result<UFix64<N9>> {
    next_levercoin_mint_nav(
      self.total_collateral()?,
      self.collateral_usd_price,
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
  pub fn levercoin_redeem_nav(&self) -> Result<UFix64<N9>> {
    next_levercoin_redeem_nav(
      self.total_collateral()?,
      self.collateral_usd_price,
      self.virtual_stablecoin_supply()?,
      self.stablecoin_nav()?,
      self.levercoin_supply()?,
    )
    .ok_or(LevercoinNav.into())
  }

  /// Stablecoin NAV is $1 unless in Depeg.
  ///
  /// # Errors
  /// * Upconversion or arithmetic failure in depeg path
  pub fn stablecoin_nav(&self) -> Result<UFix64<N9>> {
    match self.stability_mode()? {
      StabilityMode::Depeg => depeg_stablecoin_nav(
        self.total_collateral()?,
        self.collateral_usd_price.lower,
        self.virtual_stablecoin_supply()?,
      ),
      _ => Ok(UFix64::<N9>::one()),
    }
  }

  /// Current stability mode derived from collateral ratio.
  ///
  /// # Errors
  /// * CR computation or threshold validation
  pub fn stability_mode(&self) -> Result<StabilityMode> {
    let cr = self.collateral_ratio()?;
    self.stability_controller.stability_mode(cr)
  }

  /// Projected stability mode after changing totals.
  ///
  /// # Errors
  /// * CR computation or threshold validation
  pub fn projected_stability_mode(
    &self,
    new_total: UFix64<N9>,
    new_stablecoin: UFix64<N6>,
  ) -> Result<StabilityMode> {
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price.lower,
      new_stablecoin,
    )?;
    self.stability_controller.stability_mode(projected_cr)
  }

  /// Selects the worse of current vs projected mode for fee
  /// purposes.
  ///
  /// # Errors
  /// * Stability mode computation
  pub fn select_stability_mode_for_fees(
    &self,
    projected: StabilityMode,
  ) -> Result<StabilityMode> {
    let current = self.stability_mode()?;
    if projected < current {
      Ok(current)
    } else {
      Ok(projected)
    }
  }

  /// Stablecoin mint fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection overflow, interpolation, or fee extraction
  pub fn stablecoin_mint_fee(
    &self,
    amount: UFix64<N8>,
  ) -> Result<FeeExtract<N8>> {
    let collateral_added: UFix64<N9> =
      amount.checked_convert::<N9>().ok_or(ExoUpconvert)?;
    let new_total = self
      .total_collateral()?
      .checked_add(&collateral_added)
      .ok_or(ExoDestinationCollateral)?;
    let stablecoin_minted = self
      .exo_conversion()
      .exo_to_token(amount, self.stablecoin_nav()?)?;
    let new_stablecoin = stablecoin_minted
      .checked_add(&self.virtual_stablecoin_supply()?)
      .ok_or(ExoDestinationStablecoin)?;
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price.lower,
      new_stablecoin,
    )?;
    self.mint_fees.apply_fee(projected_cr, amount)
  }

  /// Stablecoin redeem fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection underflow, interpolation, or fee extraction
  pub fn stablecoin_redeem_fee(
    &self,
    amount: UFix64<N8>,
  ) -> Result<FeeExtract<N8>> {
    let collateral_removed: UFix64<N9> =
      amount.checked_convert::<N9>().ok_or(ExoUpconvert)?;
    let new_total = self
      .total_collateral()?
      .checked_sub(&collateral_removed)
      .ok_or(ExoDestinationCollateral)?;
    let stablecoin_redeemed = self
      .exo_conversion()
      .exo_to_token(amount, self.stablecoin_nav()?)?;
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&stablecoin_redeemed)
      .ok_or(ExoDestinationStablecoin)?;
    let projected_cr = collateral_ratio(
      new_total,
      self.collateral_usd_price.lower,
      new_stablecoin,
    )?;
    self.redeem_fees.apply_fee(projected_cr, amount)
  }

  /// Levercoin mint fee based on projected stability mode.
  ///
  /// # Errors
  /// * Projection overflow or mode-based fee lookup
  pub fn levercoin_mint_fee(
    &self,
    amount: UFix64<N8>,
  ) -> Result<FeeExtract<N8>> {
    let collateral_added: UFix64<N9> =
      amount.checked_convert::<N9>().ok_or(ExoUpconvert)?;
    let new_total = self
      .total_collateral()?
      .checked_add(&collateral_added)
      .ok_or(ExoDestinationCollateral)?;
    let projected = self
      .projected_stability_mode(new_total, self.virtual_stablecoin_supply()?)?;
    let mode = self.select_stability_mode_for_fees(projected)?;
    let fee = self.levercoin_fees.mint_fee(mode)?;
    FeeExtract::new(fee, amount)
  }

  /// Levercoin redeem fee based on projected stability mode.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  pub fn levercoin_redeem_fee(
    &self,
    amount: UFix64<N8>,
  ) -> Result<FeeExtract<N8>> {
    let collateral_removed: UFix64<N9> =
      amount.checked_convert::<N9>().ok_or(ExoUpconvert)?;
    let new_total = self
      .total_collateral()?
      .checked_sub(&collateral_removed)
      .ok_or(ExoDestinationCollateral)?;
    let projected = self
      .projected_stability_mode(new_total, self.virtual_stablecoin_supply()?)?;
    let mode = self.select_stability_mode_for_fees(projected)?;
    let fee = self.levercoin_fees.redeem_fee(mode)?;
    FeeExtract::new(fee, amount)
  }

  /// Swap fee for levercoin-to-stablecoin direction.
  ///
  /// # Errors
  /// * Projection overflow or mode-based fee lookup
  pub fn levercoin_to_stablecoin_fee(
    &self,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<FeeExtract<N6>> {
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_add(&amount_stablecoin)
      .ok_or(ExoDestinationStablecoin)?;
    let projected = self
      .projected_stability_mode(self.total_collateral()?, new_stablecoin)?;
    let mode = self.select_stability_mode_for_fees(projected)?;
    let fee = self.levercoin_fees.swap_to_stablecoin_fee(mode)?;
    FeeExtract::<N6>::new::<N4>(fee, amount_stablecoin)
  }

  /// Swap fee for stablecoin-to-levercoin direction.
  ///
  /// # Errors
  /// * Projection underflow or mode-based fee lookup
  pub fn stablecoin_to_levercoin_fee(
    &self,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<FeeExtract<N6>> {
    let new_stablecoin = self
      .virtual_stablecoin_supply()?
      .checked_sub(&amount_stablecoin)
      .ok_or(ExoDestinationStablecoin)?;
    let projected = self
      .projected_stability_mode(self.total_collateral()?, new_stablecoin)?;
    let mode = self.select_stability_mode_for_fees(projected)?;
    let fee = self.levercoin_fees.swap_from_stablecoin_fee(mode)?;
    FeeExtract::<N6>::new::<N4>(fee, amount_stablecoin)
  }

  /// Builds an exo collateral conversion helper.
  #[must_use]
  pub fn exo_conversion(&self) -> ExoConversion {
    ExoConversion {
      collateral_usd_price: self.collateral_usd_price,
    }
  }

  /// Builds a swap conversion between stablecoin and levercoin.
  ///
  /// # Errors
  /// * NAV computation failure
  pub fn swap_conversion(&self) -> Result<SwapConversion> {
    let levercoin_nav =
      PriceRange::new(self.levercoin_redeem_nav()?, self.levercoin_mint_nav()?);
    Ok(SwapConversion::new(self.stablecoin_nav()?, levercoin_nav))
  }

  /// Maximum mintable stablecoin until lowest CR threshold.
  ///
  /// # Errors
  /// * Upconversion or arithmetic failure
  pub fn max_mintable_stablecoin(&self) -> Result<UFix64<N6>> {
    max_mintable_stablecoin(
      self.stability_controller.min_stability_threshold(),
      self.total_collateral()?,
      self.collateral_usd_price.upper,
      self.virtual_stablecoin_supply()?,
    )
  }

  /// Maximum stablecoin swappable from levercoin.
  ///
  /// # Errors
  /// * TVL computation or arithmetic failure
  pub fn max_swappable_stablecoin(&self) -> Result<UFix64<N6>> {
    max_swappable_stablecoin(
      self.stability_controller.min_stability_threshold(),
      self.total_value_locked()?,
      self.virtual_stablecoin_supply()?,
    )
  }

  /// Validates requested stablecoin mint amount against max.
  ///
  /// # Errors
  /// * Amount exceeds max mintable
  pub fn validate_stablecoin_amount(
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

  /// Validates requested stablecoin swap amount against max.
  ///
  /// # Errors
  /// * Amount exceeds max swappable
  pub fn validate_stablecoin_swap_amount(
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
}
