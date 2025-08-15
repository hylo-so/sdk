use crate::conversion::{Conversion, SwapConversion};
use crate::error::CoreError::{
  DestinationFeeSol, DestinationFeeStablecoin, LevercoinNav,
  NoNextStabilityThreshold, RequestedStablecoinOverMaxMintable,
};
use crate::exchange_math::{
  collateral_ratio, depeg_stablecoin_nav, max_mintable_stablecoin,
  max_swappable_stablecoin, next_levercoin_nav, total_value_locked,
};
use crate::fee_controller::{
  FeeController, FeeExtract, LevercoinFees, StablecoinFees,
};
use crate::lst_sol_price::LstSolPrice;
use crate::pyth::{query_pyth_price, OracleConfig, PriceRange};
use crate::solana_clock::SolanaClock;
use crate::stability_mode::{StabilityController, StabilityMode};
use crate::stability_pool_math::stability_pool_cap;
use crate::total_sol_cache::TotalSolCache;

use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fix::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

/// Container for common values needed in an exchange transaction.
pub struct ExchangeContext<C> {
  pub clock: C,
  pub total_sol: UFix64<N9>,
  pub sol_usd_price: PriceRange<N8>,
  pub stablecoin_supply: UFix64<N6>,
  pub levercoin_supply: UFix64<N6>,
  pub collateral_ratio: UFix64<N9>,
  pub stability_controller: StabilityController,
  pub stability_mode: StabilityMode,
  stablecoin_fees: StablecoinFees,
  levercoin_fees: LevercoinFees,
}

impl<C: SolanaClock> ExchangeContext<C> {
  /// Creates main context for exchange operations from account data.
  #[allow(clippy::too_many_arguments)]
  pub fn load(
    clock: C,
    total_sol_cache: &TotalSolCache,
    stability_controller: StabilityController,
    oracle_config: OracleConfig<N8>,
    stablecoin_fees: StablecoinFees,
    levercoin_fees: LevercoinFees,
    sol_usd_pyth_feed: &PriceUpdateV2,
    stablecoin_mint: &Mint,
    levercoin_mint: &Mint,
  ) -> Result<ExchangeContext<C>> {
    let total_sol = total_sol_cache.get_validated(clock.epoch())?;
    let sol_usd_price =
      query_pyth_price(&clock, sol_usd_pyth_feed, oracle_config)?;
    let stablecoin_supply = UFix64::new(stablecoin_mint.supply);
    let levercoin_supply = UFix64::new(levercoin_mint.supply);
    let collateral_ratio =
      collateral_ratio(total_sol, sol_usd_price.lower, stablecoin_supply)?;
    let stability_mode =
      stability_controller.stability_mode(collateral_ratio)?;
    Ok(ExchangeContext {
      clock,
      total_sol,
      sol_usd_price,
      stablecoin_supply,
      levercoin_supply,
      collateral_ratio,
      stability_controller,
      stability_mode,
      stablecoin_fees,
      levercoin_fees,
    })
  }

  /// Computes TVL in USD, maintaining precision at 9 decimals.
  pub fn total_value_locked(&self) -> Result<UFix64<N9>> {
    total_value_locked(self.total_sol, self.sol_usd_price.lower)
  }

  pub fn levercoin_mint_nav(&self) -> Result<UFix64<N6>> {
    next_levercoin_nav(
      self.total_sol,
      self.sol_usd_price.upper,
      self.stablecoin_supply,
      self.stablecoin_nav()?,
      self.levercoin_supply,
    )
    .ok_or(LevercoinNav.into())
  }

  pub fn levercoin_redeem_nav(&self) -> Result<UFix64<N6>> {
    next_levercoin_nav(
      self.total_sol,
      self.sol_usd_price.lower,
      self.stablecoin_supply,
      self.stablecoin_nav()?,
      self.levercoin_supply,
    )
    .ok_or(LevercoinNav.into())
  }

  pub fn stablecoin_nav(&self) -> Result<UFix64<N6>> {
    match self.stability_mode {
      StabilityMode::Depeg => depeg_stablecoin_nav(
        self.total_sol,
        self.sol_usd_price.lower,
        self.stablecoin_supply,
      ),
      _ => Ok(UFix64::one()),
    }
  }

  /// Computes new collateral ratio and translates to a configured
  /// `StabilityMode`.
  pub fn projected_stability_mode(
    &self,
    new_total_sol: UFix64<N9>,
    new_total_stablecoin: UFix64<N6>,
  ) -> Result<StabilityMode> {
    let projected_cr = collateral_ratio(
      new_total_sol,
      self.sol_usd_price.lower,
      new_total_stablecoin,
    )?;
    self.stability_controller.stability_mode(projected_cr)
  }

  /// Extracts fees from input LST based on stability mode impact from minting
  /// new stablecoin.
  pub fn stablecoin_mint_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    // Total SOL being added
    let new_sol = lst_sol_price.convert_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationFeeSol)?;

    // Total stablecoin after mint
    let new_total_stablecoin = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst, self.stablecoin_nav()?)?
      .checked_add(&self.stablecoin_supply)
      .ok_or(DestinationFeeStablecoin)?;

    // Base fee calculation on projected stability mode
    let projected_stability_mode =
      self.projected_stability_mode(new_total_sol, new_total_stablecoin)?;

    self
      .stablecoin_fees
      .mint_fee(projected_stability_mode)
      .and_then(|fee| FeeExtract::new(fee, amount_lst))
  }

  /// Extracts fees from input LST based on stability mode impact from redeeming
  /// stablecoin.
  pub fn stablecoin_redeem_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    // Total SOL being removed from protocol
    let sol_rm = lst_sol_price.convert_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationFeeSol)?;

    // Total stablecoin after redeem
    let stablecoin_redeemed = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst, self.stablecoin_nav()?)?;
    let new_total_stablecoin = self
      .stablecoin_supply
      .checked_sub(&stablecoin_redeemed)
      .ok_or(DestinationFeeStablecoin)?;

    // Base fee calculation on projected stability mode
    let projected_stability_mode =
      self.projected_stability_mode(new_total_sol, new_total_stablecoin)?;

    self
      .stablecoin_fees
      .redeem_fee(projected_stability_mode)
      .and_then(|fee| FeeExtract::new(fee, amount_lst))
  }

  pub fn levercoin_mint_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    // Total SOL being added to protocol
    let new_sol = lst_sol_price.convert_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationFeeSol)?;

    // Base fee calculation on projected stability mode
    let projected_stability_mode =
      self.projected_stability_mode(new_total_sol, self.stablecoin_supply)?;

    self
      .levercoin_fees
      .mint_fee(projected_stability_mode)
      .and_then(|fee| FeeExtract::new(fee, amount_lst))
  }

  pub fn levercoin_redeem_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>> {
    // Total SOL being removed from protocol
    let sol_rm = lst_sol_price.convert_sol(amount_lst, self.clock.epoch())?;
    let new_total_sol = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationFeeSol)?;

    // Base fee calculation on projected stability mode
    let projected_stability_mode =
      self.projected_stability_mode(new_total_sol, self.stablecoin_supply)?;

    self
      .levercoin_fees
      .redeem_fee(projected_stability_mode)
      .and_then(|fee| FeeExtract::new(fee, amount_lst))
  }

  pub fn levercoin_to_stablecoin_fee(
    &self,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<FeeExtract<N6>> {
    // Total stablecoin after swap
    let new_total_stablecoin = self
      .stablecoin_supply
      .checked_add(&amount_stablecoin)
      .ok_or(DestinationFeeStablecoin)?;

    // Base fee calculation on projected stability mode
    let projected_stability_mode =
      self.projected_stability_mode(self.total_sol, new_total_stablecoin)?;

    self
      .levercoin_fees
      .swap_to_stablecoin_fee(projected_stability_mode)
      .and_then(|fee| FeeExtract::new(fee, amount_stablecoin))
  }

  pub fn stablecoin_to_levercoin_fee(
    &self,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<FeeExtract<N6>> {
    // Total stablecoin after swap
    let new_total_stablecoin = self
      .stablecoin_supply
      .checked_sub(&amount_stablecoin)
      .ok_or(DestinationFeeStablecoin)?;

    // Base fee calculation on projected stability mode
    let projected_stability_mode =
      self.projected_stability_mode(self.total_sol, new_total_stablecoin)?;

    self
      .levercoin_fees
      .swap_from_stablecoin_fee(projected_stability_mode)
      .and_then(|fee| FeeExtract::new(fee, amount_stablecoin))
  }

  /// Maximum mintable amount of stablecoin until lowest CR threshold is
  /// reached.
  pub fn max_mintable_stablecoin(&self) -> Result<UFix64<N6>> {
    max_mintable_stablecoin(
      self.stability_controller.min_stability_threshold(),
      self.total_sol,
      self.sol_usd_price.upper,
      self.stablecoin_supply,
    )
  }

  /// Maximum amount of stablecoin to swap into from levercoin, using the next
  /// lowest CR threshold as the limit.
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
      self.stablecoin_supply,
    )
  }

  /// Maximum amount of stablecoin possible to swap into from levercoin.
  /// Uses the second CR threshold as the limit.
  pub fn max_swappable_stablecoin(&self) -> Result<UFix64<N6>> {
    let total_value_locked = self.total_value_locked()?;
    max_swappable_stablecoin(
      self.stability_controller.min_stability_threshold(),
      total_value_locked,
      self.stablecoin_supply,
    )
  }

  /// Checks the requested amount of stablecoin swap against protocol's current
  /// max.
  pub fn validate_stablecoin_swap_amount(
    &self,
    requested_amount: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    let max_swappable = self.max_swappable_stablecoin()?;
    if requested_amount <= max_swappable {
      Ok(requested_amount)
    } else {
      Err(RequestedStablecoinOverMaxMintable.into())
    }
  }

  /// Checks the requested amount of stablecoin against protocol's current max.
  pub fn validate_stablecoin_amount(
    &self,
    requested_amount: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    let max = self.max_mintable_stablecoin()?;
    if requested_amount <= max {
      Ok(requested_amount)
    } else {
      Err(RequestedStablecoinOverMaxMintable.into())
    }
  }

  pub fn token_conversion(
    &self,
    lst_sol_price: &LstSolPrice,
  ) -> Result<Conversion> {
    let lst_sol = lst_sol_price.get_epoch_price(self.clock.epoch())?;
    Ok(Conversion::new(self.sol_usd_price, lst_sol))
  }

  pub fn swap_conversion(&self) -> Result<SwapConversion> {
    let levercoin_nav =
      PriceRange::new(self.levercoin_redeem_nav()?, self.levercoin_mint_nav()?);
    Ok(SwapConversion::new(self.stablecoin_nav()?, levercoin_nav))
  }

  /// Special case conversion from raw SOL to stablecoin.
  /// Reuses LST/SOL converter with a base conversion of 1:1 LST/SOL.
  pub fn sol_to_stablecoin(
    &self,
    amount_sol: UFix64<N9>,
  ) -> Result<UFix64<N6>> {
    let nav = self.stablecoin_nav()?;
    let conversion = Conversion::new(self.sol_usd_price, UFix64::one());
    conversion.lst_to_token(amount_sol, nav)
  }

  /// Computes total capitalization of stablecoin and levercoin in stability
  /// pool.
  pub fn stability_pool_cap(
    &self,
    stablecoin_in_pool: UFix64<N6>,
    levercoin_in_pool: UFix64<N6>,
  ) -> Result<UFix64<N6>> {
    let stablecoin_nav = self.stablecoin_nav()?;
    let levercoin_nav = self.levercoin_mint_nav()?;
    stability_pool_cap(
      stablecoin_nav,
      stablecoin_in_pool,
      levercoin_nav,
      levercoin_in_pool,
    )
  }
}
