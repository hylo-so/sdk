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
  pub(super) stablecoin_mint_fees: InterpolatedMintFees,
  pub(super) stablecoin_redeem_fees: InterpolatedRedeemFees,
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
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let projected = self.projected_mint_state(lst_sol_price, amount_lst)?;
    self
      .stablecoin_mint_fees
      .apply_fee(projected.collateral_ratio, amount_lst)
  }

  /// Post-trade state used by the stablecoin mint fee projection.
  pub(super) fn projected_mint_state(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let new_sol =
      lst_sol_price.convert_lst_to_sol(amount_lst, self.clock.epoch())?;
    let total_collateral = self
      .total_sol
      .checked_add(&new_sol)
      .ok_or(DestinationCollateral)?;
    let stablecoin_supply = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst, self.stablecoin_nav()?)?
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

  /// Stablecoin redeem fee via interpolated curve at projected CR.
  ///
  /// # Errors
  /// * Projection underflow, interpolation, or fee extraction
  pub fn stablecoin_redeem_fee(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
    let projected = self.projected_redeem_state(lst_sol_price, amount_lst)?;
    self
      .stablecoin_redeem_fees
      .apply_fee(projected.collateral_ratio, amount_lst)
  }

  /// Post-trade state used by the stablecoin redeem fee projection.
  pub(super) fn projected_redeem_state(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<ProjectedState, CoreError> {
    let sol_rm =
      lst_sol_price.convert_lst_to_sol(amount_lst, self.clock.epoch())?;
    let total_collateral = self
      .total_sol
      .checked_sub(&sol_rm)
      .ok_or(DestinationCollateral)?;
    let stablecoin_redeemed = self
      .token_conversion(lst_sol_price)?
      .lst_to_token(amount_lst, self.stablecoin_nav()?)?;
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
    amount_lst: UFix64<N9>,
  ) -> Result<FeeExtract<N9>, CoreError> {
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
  ) -> Result<FeeExtract<N9>, CoreError> {
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
  pub(super) fn projected_rebalance_sell_state(
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
  pub(super) fn projected_rebalance_buy_state(
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

#[cfg(all(test, feature = "offchain"))]
mod tests {
  use anchor_lang::prelude::Clock;
  use more_asserts::{assert_gt, assert_lt};

  use super::*;
  use crate::calculus::{positive, quotient_rule};
  use crate::fees::controller::FeePair;
  use crate::fees::curve_controller::narrow_cr;

  const SOL_USD_SPOT: UFix64<N9> = UFix64::constant(150_000_000_000);
  const LST_SOL: UFix64<N9> = UFix64::constant(1_050_000_000);

  fn curve_config(floor: u64, ceil: u64) -> RebalanceCurveConfig {
    RebalanceCurveConfig {
      floor_pct: UFixValue64 {
        bits: floor,
        exp: -9,
      },
      ceil_pct: UFixValue64 {
        bits: ceil,
        exp: -9,
      },
    }
  }

  /// Synthetic context; `total_sol` and `stablecoin_supply` position
  /// the CR for each scenario.
  fn context(
    total_sol: UFix64<N9>,
    stablecoin_supply: UFix64<N6>,
  ) -> Result<LstExchangeContext<Clock>, CoreError> {
    let sol_usd_price = PriceRange {
      lower: UFix64::constant(149_900_000_000),
      upper: UFix64::constant(150_100_000_000),
    };
    let cr =
      collateral_ratio(total_sol, sol_usd_price.lower, stablecoin_supply)?;
    let flat_fee = UFixValue64 { bits: 50, exp: -4 };
    let fee_pair = FeePair {
      mint: flat_fee,
      redeem: flat_fee,
    };
    Ok(LstExchangeContext {
      clock: Clock::default(),
      total_sol,
      sol_usd_oracle: OraclePrice {
        spot: SOL_USD_SPOT,
        conf: UFix64::constant(100_000_000),
      },
      sol_usd_price,
      virtual_stablecoin: VirtualStablecoin {
        supply: stablecoin_supply.into(),
      },
      levercoin_supply: Some(UFix64::constant(30_000_000_000_000)),
      collateral_ratio: cr,
      stablecoin_mint_threshold: UFix64::constant(1_500_000_000),
      rebalance_mode: RebalanceMode::from_cr(cr),
      stablecoin_mint_fees: InterpolatedMintFees::new(mint_fee_curve()?),
      stablecoin_redeem_fees: InterpolatedRedeemFees::new(redeem_fee_curve()?),
      levercoin_fees: LevercoinFees::new(fee_pair, fee_pair, fee_pair),
      sell_curve_config: curve_config(10_000_000, 5_000_000),
      buy_curve_config: curve_config(5_000_000, 10_000_000),
    })
  }

  fn lst_price() -> LstSolPrice {
    LstSolPrice::new(LST_SOL.into(), 0)
  }

  /// CR ~1.545: interior of both stablecoin fee curves.
  fn mid_cr_context() -> Result<LstExchangeContext<Clock>, CoreError> {
    context(
      UFix64::constant(1_030_000_000_000_000),
      UFix64::constant(100_000_000_000_000),
    )
  }

  fn assert_rel_close(analytic: f64, probe: f64, tolerance: f64) {
    let rel = ((analytic - probe) / analytic).abs();
    assert!(
      rel < tolerance,
      "analytic {analytic} vs probe {probe}: rel error {rel}"
    );
  }

  fn mint_output(
    ctx: &LstExchangeContext<Clock>,
    amount_lst: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let FeeExtract {
      amount_remaining, ..
    } = ctx.stablecoin_mint_fee(&lst_price(), amount_lst)?;
    let out = ctx
      .token_conversion(&lst_price())?
      .lst_to_token(amount_remaining, ctx.stablecoin_nav()?)?;
    Ok(out.to_f64())
  }

  fn redeem_output(
    ctx: &LstExchangeContext<Clock>,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<f64, CoreError> {
    let lst_out = ctx
      .token_conversion(&lst_price())?
      .token_to_lst(amount_stablecoin, ctx.stablecoin_nav()?)?;
    let FeeExtract {
      amount_remaining, ..
    } = ctx.stablecoin_redeem_fee(&lst_price(), lst_out)?;
    Ok(amount_remaining.to_f64())
  }

  /// Central finite difference `(f(x+d) - f(x-d)) / 2d` — the probe
  /// demoted from production to test oracle.
  fn central_diff(
    f: impl Fn(u64) -> Result<f64, CoreError>,
    x: u64,
    delta: u64,
  ) -> Result<f64, CoreError> {
    #[allow(clippy::cast_precision_loss)]
    let width = (2 * delta) as f64;
    Ok((f(x + delta)? - f(x - delta)?) / (width * 1e-9))
  }

  #[test]
  fn mint_marginal_matches_finite_difference() -> Result<(), CoreError> {
    let ctx = mid_cr_context()?;
    // 1 to 10k LST at $157/LST; tolerance bounded by the fee curve's
    // N5 quantum amplified by x / 2delta = 50
    [1_000_000_000u64, 1e11 as u64, 1e12 as u64, 1e13 as u64]
      .into_iter()
      .try_for_each(|x| {
        let analytic =
          ctx.stablecoin_mint_marginal(&lst_price(), UFix64::new(x))?;
        let probe =
          central_diff(|v| mint_output(&ctx, UFix64::new(v)), x, x / 100)?;
        assert_rel_close(analytic, probe, 1e-3);
        Ok(())
      })
  }

  #[test]
  fn redeem_marginal_matches_finite_difference() -> Result<(), CoreError> {
    let ctx = mid_cr_context()?;
    // 100 to 10M hyUSD
    [100_000_000u64, 1e10 as u64, 1e12 as u64, 1e13 as u64]
      .into_iter()
      .try_for_each(|x| {
        let analytic =
          ctx.stablecoin_redeem_marginal(&lst_price(), UFix64::new(x))?;
        let probe = central_diff(
          |v| redeem_output(&ctx, UFix64::<N6>::new(v).convert()),
          x,
          x / 100,
        )?;
        // Redeem output is N6 tokens over N6 input: rescale probe width
        assert_rel_close(analytic, probe * 1e-3, 1e-3);
        Ok(())
      })
  }

  #[test]
  fn mint_marginal_decreases_with_size() -> Result<(), CoreError> {
    let ctx = mid_cr_context()?;
    let small = ctx.stablecoin_mint_marginal(
      &lst_price(),
      UFix64::constant(1_000_000_000),
    )?;
    let large = ctx.stablecoin_mint_marginal(
      &lst_price(),
      UFix64::constant(10_000_000_000_000),
    )?;
    assert_lt!(large, small);
    Ok(())
  }

  #[test]
  fn mint_fee_slope_is_negative() -> Result<(), CoreError> {
    let ctx = mid_cr_context()?;
    let cr = narrow_cr(ctx.collateral_ratio)?;
    let slope = ctx.stablecoin_mint_fees.fee_slope(cr)?;
    assert_lt!(slope, IFix64::zero());
    Ok(())
  }

  #[test]
  fn mint_cr_impact_is_negative_above_par() -> Result<(), CoreError> {
    let ctx = mid_cr_context()?;
    let projected = ctx
      .projected_mint_state(&lst_price(), UFix64::constant(1_000_000_000))?;
    let lst_sol = LST_SOL.to_f64();
    let sol_usd_lower = ctx.sol_usd_price.lower.to_f64();
    let nav_rate = lst_sol * sol_usd_lower / ctx.stablecoin_nav()?.to_f64();
    let impact = quotient_rule(
      positive(projected.total_collateral)?,
      lst_sol,
      positive(projected.stablecoin_supply)?,
      nav_rate,
      positive(ctx.sol_usd_price.lower)?,
    );
    assert_lt!(impact, 0.0);
    Ok(())
  }

  #[test]
  fn rebalance_sell_curve_slope_is_positive() -> Result<(), CoreError> {
    // CR ~1.28: sell zone
    let ctx = context(
      UFix64::constant(1_030_000_000_000_000),
      UFix64::constant(120_500_000_000_000),
    )?;
    let slope = ctx
      .rebalance_sell_curve()?
      .price_slope(ctx.collateral_ratio)?;
    assert_gt!(slope, IFix64::zero());
    Ok(())
  }

  const USDC_USD: PriceRange<N9> = PriceRange {
    lower: UFix64::constant(999_000_000),
    upper: UFix64::constant(1_001_000_000),
  };

  fn buy_output(
    ctx: &LstExchangeContext<Clock>,
    lst_amount: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let conversion =
      ctx.rebalance_buy_conversion(&lst_price(), USDC_USD, lst_amount)?;
    Ok(conversion.lst_to_usdc(lst_amount)?.to_f64())
  }

  fn sell_output(
    ctx: &LstExchangeContext<Clock>,
    usdc_amount: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let conversion =
      ctx.rebalance_sell_conversion(&lst_price(), USDC_USD, usdc_amount)?;
    Ok(conversion.usdc_to_lst(usdc_amount)?.to_f64())
  }

  #[test]
  fn rebalance_buy_marginal_matches_finite_difference() -> Result<(), CoreError>
  {
    // CR ~1.71: buy zone interior
    let ctx = context(
      UFix64::constant(1_030_000_000_000_000),
      UFix64::constant(90_500_000_000_000),
    )?;
    // 100 to 10k LST
    [1e11 as u64, 1e12 as u64, 1e13 as u64]
      .into_iter()
      .try_for_each(|x| {
        let analytic =
          ctx.rebalance_buy_marginal(&lst_price(), USDC_USD, UFix64::new(x))?;
        let probe =
          central_diff(|v| buy_output(&ctx, UFix64::new(v)), x, x / 100)?;
        assert_rel_close(analytic, probe, 1e-3);
        Ok(())
      })
  }

  #[test]
  fn rebalance_sell_marginal_matches_finite_difference() -> Result<(), CoreError>
  {
    // CR ~1.28: sell zone interior
    let ctx = context(
      UFix64::constant(1_030_000_000_000_000),
      UFix64::constant(120_500_000_000_000),
    )?;
    // 1k to 1M USDC
    [1e12 as u64, 1e13 as u64, 1e15 as u64]
      .into_iter()
      .try_for_each(|x| {
        let analytic = ctx.rebalance_sell_marginal(
          &lst_price(),
          USDC_USD,
          UFix64::new(x),
        )?;
        let probe =
          central_diff(|v| sell_output(&ctx, UFix64::new(v)), x, x / 100)?;
        assert_rel_close(analytic, probe, 1e-3);
        Ok(())
      })
  }

  #[test]
  fn small_mint_marginal_approaches_spot_rate() -> Result<(), CoreError> {
    let ctx = mid_cr_context()?;
    let amount = UFix64::constant(1_000_000);
    let marginal = ctx.stablecoin_mint_marginal(&lst_price(), amount)?;
    let out = mint_output(&ctx, amount)?;
    assert_rel_close(marginal, out / amount.to_f64(), 1e-3);
    Ok(())
  }
}
