//! Marginal rate derivations for exchange context operations.

use fix::prelude::*;

use super::exo::ExoExchangeContext;
use super::lst::LstExchangeContext;
use super::ExchangeContext;
use crate::calculus::{chain_rule, positive, positive_rate, quotient_rule};
use crate::error::CoreError;
use crate::fees::curve_controller::{narrow_cr, InterpolatedFeeController};
use crate::lst::sol_price::LstSolPrice;
use crate::pyth::PriceRange;
use crate::rebalance::pricing::RebalancePriceController;
use crate::solana_clock::SolanaClock;

impl<C: SolanaClock> ExoExchangeContext<C> {
  /// Marginal stablecoin output per collateral input at
  /// `collateral_amount`, in tokens.
  ///
  /// ```txt
  /// f'(x) = nav_rate * (1 - fee - x * fee_slope * cr'(x))
  /// ```
  ///
  /// # Errors
  /// * Projection, curve, NAV, or marginal rate failure
  pub fn stablecoin_mint_marginal(
    &self,
    collateral_amount: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let projected = self.projected_mint_state(collateral_amount)?;
    let cr = narrow_cr(projected.collateral_ratio)?;
    let fee = self.stablecoin_mint_fees.fee_inner(cr)?.to_f64();
    let fee_slope = self.stablecoin_mint_fees.fee_slope(cr)?.to_f64();
    let collateral_usd_lower = positive(self.collateral_usd_price.lower)?;
    let nav = positive(self.stablecoin_nav()?)?;

    // hyusd_out(x) = x * nav_rate * (1 - fee(cr(x)))
    //   where nav_rate = collateral_usd_lower / nav
    let nav_rate = collateral_usd_lower.get() / nav.get();

    // total_collateral(x)  = vault  + x             => d = 1
    // stablecoin_supply(x) = supply + x * nav_rate  => d = nav_rate
    let d_total_collateral = 1.0;
    let d_stablecoin_supply = nav_rate;

    // cr(x)  = total_collateral * collateral_usd_lower / stablecoin_supply
    // cr'(x) = (d_C * S - C * d_S) * collateral_usd_lower / S^2
    let d_cr = quotient_rule(
      positive(projected.total_collateral)?,
      d_total_collateral,
      positive(projected.stablecoin_supply)?,
      d_stablecoin_supply,
      collateral_usd_lower,
    );

    // d fee(cr(x)) = fee'(cr) * cr'(x)
    let d_fee = chain_rule(fee_slope, d_cr);

    // hyusd_out'(x) = nav_rate * (1 - fee) - x * nav_rate * d_fee
    let rate = nav_rate * (1.0 - fee);
    positive_rate(rate - collateral_amount.to_f64() * nav_rate * d_fee)
  }

  /// Marginal collateral output per stablecoin input at
  /// `amount_stablecoin`, in tokens.
  ///
  /// # Errors
  /// * Projection, curve, NAV, or marginal rate failure
  pub fn stablecoin_redeem_marginal(
    &self,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<f64, CoreError> {
    let nav = positive(self.stablecoin_nav()?)?;
    let collateral_out = self
      .exo_conversion()
      .token_to_exo(amount_stablecoin, self.stablecoin_nav()?)?;
    let projected = self.projected_redeem_state(collateral_out)?;
    let cr = narrow_cr(projected.collateral_ratio)?;
    let fee = self.stablecoin_redeem_fees.fee_inner(cr)?.to_f64();
    let fee_slope = self.stablecoin_redeem_fees.fee_slope(cr)?.to_f64();
    let collateral_usd_lower = positive(self.collateral_usd_price.lower)?;
    let collateral_usd_upper = positive(self.collateral_usd_price.upper)?;

    // collateral_out(x) = x * nav_rate * (1 - fee(cr(x)))
    //   where nav_rate = nav / collateral_usd_upper
    let nav_rate = nav.get() / collateral_usd_upper.get();

    // total_collateral(x)  = vault  - x * nav_rate
    // stablecoin_supply(x) = supply - x * collateral_usd_lower /
    // collateral_usd_upper
    let d_total_collateral = -nav_rate;
    let d_stablecoin_supply =
      -(collateral_usd_lower.get() / collateral_usd_upper.get());

    // cr(x)  = total_collateral * collateral_usd_lower / stablecoin_supply
    // cr'(x) = (d_C * S - C * d_S) * collateral_usd_lower / S^2
    let d_cr = quotient_rule(
      positive(projected.total_collateral)?,
      d_total_collateral,
      positive(projected.stablecoin_supply)?,
      d_stablecoin_supply,
      collateral_usd_lower,
    );

    // d fee(cr(x)) = fee'(cr) * cr'(x)
    let d_fee = chain_rule(fee_slope, d_cr);

    // collateral_out'(x) = nav_rate * (1 - fee) - x * nav_rate * d_fee
    let rate = nav_rate * (1.0 - fee);
    positive_rate(rate - amount_stablecoin.to_f64() * nav_rate * d_fee)
  }

  /// Marginal USDC output per collateral input at `collateral_amount`,
  /// in tokens.
  ///
  /// ```txt
  /// f'(x) = R(cr(x)) + x * R'(cr(x)) * cr'(x)
  /// ```
  ///
  /// where `R` is the buy-side price curve scaled into USDC.
  ///
  /// # Errors
  /// * Projection, curve, NAV, or marginal rate failure
  pub fn rebalance_buy_marginal(
    &self,
    usdc_usd_price: PriceRange<N9>,
    collateral_amount: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let projected = self.projected_rebalance_buy_state(collateral_amount)?;
    let curve = self.rebalance_buy_curve()?;
    let curve_price = curve.price(projected.collateral_ratio)?.to_f64();
    let curve_slope = curve.price_slope(projected.collateral_ratio)?.to_f64();
    let collateral_spot = positive(self.collateral_oracle_price().spot)?;
    let nav = positive(self.stablecoin_nav()?)?;
    let usdc_usd_upper = positive(usdc_usd_price.upper)?;

    // usdc_out(x) = x * curve_price(cr(x)) / usdc_usd_upper

    // total_collateral(x)  = vault  + x                          => d = 1
    // stablecoin_supply(x) = supply + x * collateral_spot / nav
    let d_total_collateral = 1.0;
    let d_stablecoin_supply = collateral_spot.get() / nav.get();

    // cr(x)  = total_collateral * collateral_spot / stablecoin_supply
    // cr'(x) = (d_C * S - C * d_S) * collateral_spot / S^2
    let d_cr = quotient_rule(
      positive(projected.total_collateral)?,
      d_total_collateral,
      positive(projected.stablecoin_supply)?,
      d_stablecoin_supply,
      collateral_spot,
    );

    // d curve_price(cr(x)) = curve_price'(cr) * cr'(x)
    let d_curve_price = chain_rule(curve_slope, d_cr);

    // usdc_out'(x) = (curve_price + x * d_curve_price) / usdc_usd_upper
    let rate = curve_price / usdc_usd_upper.get();
    positive_rate(
      rate + collateral_amount.to_f64() * d_curve_price / usdc_usd_upper.get(),
    )
  }

  /// Marginal collateral output per USDC input at `usdc_amount`, in
  /// tokens.
  ///
  /// The sell-side curve price sits in the denominator of the rate, so
  ///
  /// ```txt
  /// R'(c) = -usdc_usd * p'(c) / p(c)^2
  /// ```
  ///
  /// # Errors
  /// * Projection, curve, NAV, or marginal rate failure
  pub fn rebalance_sell_marginal(
    &self,
    usdc_usd_price: PriceRange<N9>,
    usdc_amount: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let projected =
      self.projected_rebalance_sell_state(usdc_usd_price, usdc_amount)?;
    let curve = self.rebalance_sell_curve()?;
    let curve_price = positive(curve.price(projected.collateral_ratio)?)?;
    let curve_slope = curve.price_slope(projected.collateral_ratio)?.to_f64();
    let collateral_spot = positive(self.collateral_oracle_price().spot)?;
    let nav = positive(self.stablecoin_nav()?)?;
    let usdc_usd_lower = positive(usdc_usd_price.lower)?;

    // collateral_out(x) = x * usdc_usd_lower / curve_price(cr(x))

    // total_collateral(x)  = vault  - x * usdc_usd_lower / collateral_spot
    // stablecoin_supply(x) = supply - x * usdc_usd_lower / nav
    let d_total_collateral = -(usdc_usd_lower.get() / collateral_spot.get());
    let d_stablecoin_supply = -(usdc_usd_lower.get() / nav.get());

    // cr(x)  = total_collateral * collateral_spot / stablecoin_supply
    // cr'(x) = (d_C * S - C * d_S) * collateral_spot / S^2
    let d_cr = quotient_rule(
      positive(projected.total_collateral)?,
      d_total_collateral,
      positive(projected.stablecoin_supply)?,
      d_stablecoin_supply,
      collateral_spot,
    );

    // d curve_price(cr(x)) = curve_price'(cr) * cr'(x)
    let d_curve_price = chain_rule(curve_slope, d_cr);

    // reciprocal rule on the divisor:
    // collateral_out'(x) = usdc_usd_lower
    //   * (1 / curve_price - x * d_curve_price / curve_price^2)
    let rate = usdc_usd_lower.get() / curve_price.get();
    positive_rate(
      rate
        - usdc_amount.to_f64() * usdc_usd_lower.get() * d_curve_price
          / (curve_price.get() * curve_price.get()),
    )
  }
}

impl<C: SolanaClock> LstExchangeContext<C> {
  /// Marginal stablecoin output per LST input at `amount_lst`, in
  /// tokens.
  ///
  /// ```txt
  /// f'(x) = nav_rate * (1 - fee - x * fee_slope * cr'(x))
  /// ```
  ///
  /// # Errors
  /// * Projection, curve, NAV, or marginal rate failure
  pub fn stablecoin_mint_marginal(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_lst: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let projected = self.projected_mint_state(lst_sol_price, amount_lst)?;
    let cr = narrow_cr(projected.collateral_ratio)?;
    let fee = self.stablecoin_mint_fees.fee_inner(cr)?.to_f64();
    let fee_slope = self.stablecoin_mint_fees.fee_slope(cr)?.to_f64();
    let lst_sol = positive(lst_sol_price.get_epoch_price(self.clock.epoch())?)?;
    let sol_usd_lower = positive(self.sol_usd_price.lower)?;
    let nav = positive(self.stablecoin_nav()?)?;

    // hyusd_out(x) = x * nav_rate * (1 - fee(cr(x)))
    //   where nav_rate = lst_sol * sol_usd_lower / nav
    let nav_rate = lst_sol.get() * sol_usd_lower.get() / nav.get();

    // total_collateral(x)  = total_sol + x * lst_sol   (vault counts SOL)
    // stablecoin_supply(x) = supply    + x * nav_rate
    let d_total_collateral = lst_sol.get();
    let d_stablecoin_supply = nav_rate;

    // cr(x)  = total_collateral * sol_usd_lower / stablecoin_supply
    // cr'(x) = (d_C * S - C * d_S) * sol_usd_lower / S^2
    let d_cr = quotient_rule(
      positive(projected.total_collateral)?,
      d_total_collateral,
      positive(projected.stablecoin_supply)?,
      d_stablecoin_supply,
      sol_usd_lower,
    );

    // d fee(cr(x)) = fee'(cr) * cr'(x)
    let d_fee = chain_rule(fee_slope, d_cr);

    // hyusd_out'(x) = nav_rate * (1 - fee) - x * nav_rate * d_fee
    let rate = nav_rate * (1.0 - fee);
    positive_rate(rate - amount_lst.to_f64() * nav_rate * d_fee)
  }

  /// Marginal LST output per stablecoin input at `amount_stablecoin`,
  /// in tokens.
  ///
  /// # Errors
  /// * Projection, curve, NAV, or marginal rate failure
  pub fn stablecoin_redeem_marginal(
    &self,
    lst_sol_price: &LstSolPrice,
    amount_stablecoin: UFix64<N6>,
  ) -> Result<f64, CoreError> {
    let nav = positive(self.stablecoin_nav()?)?;
    let lst_out = self
      .token_conversion(lst_sol_price)?
      .token_to_lst(amount_stablecoin, self.stablecoin_nav()?)?;
    let projected = self.projected_redeem_state(lst_sol_price, lst_out)?;
    let cr = narrow_cr(projected.collateral_ratio)?;
    let fee = self.stablecoin_redeem_fees.fee_inner(cr)?.to_f64();
    let fee_slope = self.stablecoin_redeem_fees.fee_slope(cr)?.to_f64();
    let lst_sol = positive(lst_sol_price.get_epoch_price(self.clock.epoch())?)?;
    let sol_usd_lower = positive(self.sol_usd_price.lower)?;
    let sol_usd_upper = positive(self.sol_usd_price.upper)?;

    // lst_out(x) = x * nav_rate * (1 - fee(cr(x)))
    //   where nav_rate = nav / (sol_usd_upper * lst_sol)
    let nav_rate = nav.get() / (sol_usd_upper.get() * lst_sol.get());

    // total_collateral(x)  = total_sol - x * nav / sol_usd_upper
    // stablecoin_supply(x) = supply - x * sol_usd_lower / sol_usd_upper
    let d_total_collateral = -(nav.get() / sol_usd_upper.get());
    let d_stablecoin_supply = -(sol_usd_lower.get() / sol_usd_upper.get());

    // cr(x)  = total_collateral * sol_usd_lower / stablecoin_supply
    // cr'(x) = (d_C * S - C * d_S) * sol_usd_lower / S^2
    let d_cr = quotient_rule(
      positive(projected.total_collateral)?,
      d_total_collateral,
      positive(projected.stablecoin_supply)?,
      d_stablecoin_supply,
      sol_usd_lower,
    );

    // d fee(cr(x)) = fee'(cr) * cr'(x)
    let d_fee = chain_rule(fee_slope, d_cr);

    // lst_out'(x) = nav_rate * (1 - fee) - x * nav_rate * d_fee
    let rate = nav_rate * (1.0 - fee);
    positive_rate(rate - amount_stablecoin.to_f64() * nav_rate * d_fee)
  }

  /// Marginal USDC output per LST input at `lst_amount`, in tokens.
  ///
  /// ```txt
  /// f'(x) = R(cr(x)) + x * R'(cr(x)) * cr'(x)
  /// ```
  ///
  /// where `R` is the buy-side price curve scaled into USDC.
  ///
  /// # Errors
  /// * Projection, curve, NAV, or marginal rate failure
  pub fn rebalance_buy_marginal(
    &self,
    lst_sol_price: &LstSolPrice,
    usdc_usd_price: PriceRange<N9>,
    lst_amount: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let projected =
      self.projected_rebalance_buy_state(lst_sol_price, lst_amount)?;
    let curve = self.rebalance_buy_curve()?;
    let curve_price = curve.price(projected.collateral_ratio)?.to_f64();
    let curve_slope = curve.price_slope(projected.collateral_ratio)?.to_f64();
    let lst_sol = positive(lst_sol_price.get_epoch_price(self.clock.epoch())?)?;
    let sol_spot = positive(self.collateral_oracle_price().spot)?;
    let nav = positive(self.stablecoin_nav()?)?;
    let usdc_usd_upper = positive(usdc_usd_price.upper)?;

    // usdc_out(x) = x * lst_sol * curve_price(cr(x)) / usdc_usd_upper

    // total_collateral(x)  = total_sol + x * lst_sol   (vault counts SOL)
    // stablecoin_supply(x) = supply + x * lst_sol * sol_spot / nav
    let d_total_collateral = lst_sol.get();
    let d_stablecoin_supply = lst_sol.get() * sol_spot.get() / nav.get();

    // cr(x)  = total_collateral * sol_spot / stablecoin_supply
    // cr'(x) = (d_C * S - C * d_S) * sol_spot / S^2
    let d_cr = quotient_rule(
      positive(projected.total_collateral)?,
      d_total_collateral,
      positive(projected.stablecoin_supply)?,
      d_stablecoin_supply,
      sol_spot,
    );

    // d curve_price(cr(x)) = curve_price'(cr) * cr'(x)
    let d_curve_price = chain_rule(curve_slope, d_cr);

    // usdc_out'(x) = lst_sol * (curve_price + x * d_curve_price) /
    // usdc_usd_upper
    let rate = lst_sol.get() * curve_price / usdc_usd_upper.get();
    positive_rate(
      rate
        + lst_amount.to_f64() * lst_sol.get() * d_curve_price
          / usdc_usd_upper.get(),
    )
  }

  /// Marginal LST output per USDC input at `usdc_amount`, in tokens.
  ///
  /// The sell-side curve price sits in the denominator of the rate, so
  ///
  /// ```txt
  /// R'(c) = -usdc_usd * p'(c) / (p(c)^2 * lst_sol)
  /// ```
  ///
  /// # Errors
  /// * Projection, curve, NAV, or marginal rate failure
  pub fn rebalance_sell_marginal(
    &self,
    lst_sol_price: &LstSolPrice,
    usdc_usd_price: PriceRange<N9>,
    usdc_amount: UFix64<N9>,
  ) -> Result<f64, CoreError> {
    let projected = self.projected_rebalance_sell_state(
      lst_sol_price,
      usdc_usd_price,
      usdc_amount,
    )?;
    let curve = self.rebalance_sell_curve()?;
    let curve_price = positive(curve.price(projected.collateral_ratio)?)?;
    let curve_slope = curve.price_slope(projected.collateral_ratio)?.to_f64();
    let lst_sol = positive(lst_sol_price.get_epoch_price(self.clock.epoch())?)?;
    let sol_spot = positive(self.collateral_oracle_price().spot)?;
    let nav = positive(self.stablecoin_nav()?)?;
    let usdc_usd_lower = positive(usdc_usd_price.lower)?;

    // lst_out(x) = x * usdc_usd_lower / (curve_price(cr(x)) * lst_sol)

    // total_collateral(x)  = total_sol - x * usdc_usd_lower / sol_spot
    // stablecoin_supply(x) = supply - x * usdc_usd_lower / nav
    let d_total_collateral = -(usdc_usd_lower.get() / sol_spot.get());
    let d_stablecoin_supply = -(usdc_usd_lower.get() / nav.get());

    // cr(x)  = total_collateral * sol_spot / stablecoin_supply
    // cr'(x) = (d_C * S - C * d_S) * sol_spot / S^2
    let d_cr = quotient_rule(
      positive(projected.total_collateral)?,
      d_total_collateral,
      positive(projected.stablecoin_supply)?,
      d_stablecoin_supply,
      sol_spot,
    );

    // d curve_price(cr(x)) = curve_price'(cr) * cr'(x)
    let d_curve_price = chain_rule(curve_slope, d_cr);

    // reciprocal rule on the divisor:
    // lst_out'(x) = usdc_usd_lower / lst_sol
    //   * (1 / curve_price - x * d_curve_price / curve_price^2)
    let rate = usdc_usd_lower.get() / (curve_price.get() * lst_sol.get());
    positive_rate(
      rate
        - usdc_amount.to_f64() * usdc_usd_lower.get() * d_curve_price
          / (curve_price.get() * curve_price.get() * lst_sol.get()),
    )
  }
}
