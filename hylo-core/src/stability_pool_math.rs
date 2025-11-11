use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::conversion::SwapConversion;
use crate::error::CoreError::{
  LpTokenNav, LpTokenOut, StabilityPoolCap, StablecoinToSwap, TokenWithdraw,
};
use crate::fee_controller::FeeExtract;
use crate::oracle::PriceRange;

/// Calculates total dollar value of stablecoin and levercoin in stability pool.
///
/// ```txt                
/// stability_pool_cap = stable_nav * stable_in_pool + lever_nav * lever_in_pool
/// ```
pub fn stability_pool_cap(
  stablecoin_nav: UFix64<N9>,
  stablecoin_in_pool: UFix64<N6>,
  levercoin_nav: UFix64<N9>,
  levercoin_in_pool: UFix64<N6>,
) -> Result<UFix64<N6>> {
  let stable_cap =
    stablecoin_in_pool.mul_div_ceil(stablecoin_nav, UFix64::one());
  let lever_cap = levercoin_in_pool.mul_div_ceil(levercoin_nav, UFix64::one());
  stable_cap
    .zip(lever_cap)
    .and_then(|(s, l)| s.checked_add(&l))
    .ok_or(StabilityPoolCap.into())
}

/// Computes NAV for the stability pool's LP token, based on the amount of each
/// protocol token in pools and their current NAV.
///
/// ```txt
///                  stability_pool_cap
/// lp_token_nav =  --------------------
///                   lp_token_supply
/// ```
pub fn lp_token_nav(
  stablecoin_nav: UFix64<N9>,
  stablecoin_in_pool: UFix64<N6>,
  levercoin_nav: UFix64<N9>,
  levercoin_in_pool: UFix64<N6>,
  lp_token_supply: UFix64<N6>,
) -> Result<UFix64<N6>> {
  if lp_token_supply == UFix64::zero() {
    Ok(UFix64::one())
  } else {
    let total_cap = stability_pool_cap(
      stablecoin_nav,
      stablecoin_in_pool,
      levercoin_nav,
      levercoin_in_pool,
    )?;
    total_cap
      .mul_div_ceil(UFix64::one(), lp_token_supply)
      .ok_or(LpTokenNav.into())
  }
}

/// Simply divides the amount of stablecoin being deposited by the LP token NAV.
pub fn lp_token_out(
  amount_stablecoin_in: UFix64<N6>,
  lp_token_nav: UFix64<N6>,
) -> Result<UFix64<N6>> {
  amount_stablecoin_in
    .mul_div_floor(UFix64::one(), lp_token_nav)
    .ok_or(LpTokenOut.into())
}

/// Computes amount of token to withdraw, given a user's LP equity in the pool.
pub fn amount_token_to_withdraw(
  user_lp_token_amount: UFix64<N6>,
  lp_token_supply: UFix64<N6>,
  pool_amount: UFix64<N6>,
) -> Result<UFix64<N6>> {
  user_lp_token_amount
    .mul_div_floor(pool_amount, lp_token_supply)
    .ok_or(TokenWithdraw.into())
}

/// Given the next target highest stability threshold, determines the amount
/// of stablecoin to swap out from the pool.
pub fn amount_stable_to_swap(
  stablecoin_in_pool: UFix64<N6>,
  target_stability_threshold: UFix64<N2>,
  current_stablecoin_supply: UFix64<N6>,
  total_value_locked: UFix64<N9>,
) -> Result<UFix64<N6>> {
  total_value_locked
    .checked_div(&target_stability_threshold.convert::<N3>())
    .and_then(|target_supply| {
      current_stablecoin_supply.checked_sub(&target_supply)
    })
    .map(|stablecoin_to_swap| {
      if stablecoin_in_pool >= stablecoin_to_swap {
        stablecoin_to_swap
      } else {
        stablecoin_in_pool
      }
    })
    .ok_or(StablecoinToSwap.into())
}

/// Computes a stablecoin target based on levercoin in pool.
/// Compares to max mintable stablecoin and returns lesser of the two.
pub fn amount_lever_to_swap(
  levercoin_in_pool: UFix64<N6>,
  levercoin_nav: PriceRange<N9>,
  max_swappable_stablecoin: UFix64<N6>,
) -> Result<UFix64<N6>> {
  let conversion = SwapConversion::new(UFix64::one(), levercoin_nav);
  let target_stablecoin = conversion.lever_to_stable(levercoin_in_pool)?;
  if target_stablecoin <= max_swappable_stablecoin {
    Ok(levercoin_in_pool)
  } else {
    let less_levercoin =
      conversion.stable_to_lever(max_swappable_stablecoin)?;
    Ok(less_levercoin)
  }
}

/// Extracts single-sided fees in terms of stablecoin for user withdrawals.
/// * Computes total cap of user's allocation (stablecoin + levercoin)
/// * Extracts withdrawal fee in stablecoin
/// * Validates fee amount against total stablecoin in pool
/// * Returns extracted fees and the remaining stablecoin after fee deduction
pub fn stablecoin_withdrawal_fee(
  stablecoin_in_pool: UFix64<N6>,
  stablecoin_to_withdraw: UFix64<N6>,
  stablecoin_nav: UFix64<N9>,
  levercoin_to_withdraw: UFix64<N6>,
  levercoin_nav: UFix64<N9>,
  withdrawal_fee: UFix64<N4>,
) -> Result<FeeExtract<N6>> {
  let allocation_cap = stability_pool_cap(
    stablecoin_nav,
    stablecoin_to_withdraw,
    levercoin_nav,
    levercoin_to_withdraw,
  )?;
  let FeeExtract {
    fees_extracted: proposed_fee_stablecoin,
    ..
  } = FeeExtract::new(withdrawal_fee, allocation_cap)?;
  let fees_extracted = proposed_fee_stablecoin.min(stablecoin_in_pool);
  let amount_remaining = stablecoin_to_withdraw.saturating_sub(&fees_extracted);
  Ok(FeeExtract {
    fees_extracted,
    amount_remaining,
  })
}

#[cfg(test)]
mod tests {
  use proptest::prelude::*;

  use super::*;
  use crate::eq_tolerance;
  use crate::util::proptest::{protocol_state, ProtocolState};

  fn token_amount() -> BoxedStrategy<UFix64<N6>> {
    (1u64..u64::MAX).prop_map(UFix64::new).boxed()
  }

  proptest! {
    #[test]
    fn amount_withdraw_ok(
      user_lp_token_amount in token_amount(),
      lp_token_supply in token_amount(),
      pool_amount in token_amount(),
    ) {
      prop_assume!(user_lp_token_amount <= lp_token_supply);
      prop_assert!(
        amount_token_to_withdraw(user_lp_token_amount, lp_token_supply, pool_amount).is_ok()
      );
    }
  }

  #[allow(dead_code)]
  #[derive(Debug)]
  struct StabilityPoolState {
    pub stablecoin_in_pool: UFix64<N6>,
    pub levercoin_in_pool: UFix64<N6>,
    pub lp_token_supply: UFix64<N6>,
    pub stablecoin_nav: UFix64<N9>,
    pub levercoin_nav: UFix64<N9>,
  }

  fn pct_staked(min: UFix64<N2>, max: UFix64<N2>) -> BoxedStrategy<UFix64<N2>> {
    (min.bits..max.bits).prop_map(UFix64::new).boxed()
  }

  prop_compose! {
    pub fn make_stability_pool_state(protocol_state: ProtocolState)(
      lp_token_supply in 0..protocol_state.stablecoin_amount.bits,
      stablecoin_staked in pct_staked(UFix64::new(30), UFix64::new(99)),
      levercoin_staked in pct_staked(UFix64::new(0), UFix64::new(5)),
    ) -> StabilityPoolState {
      let stablecoin_in_pool =
        protocol_state
        .stablecoin_amount
        .mul_div_floor(stablecoin_staked, UFix64::one())
        .expect("stablecoin_in_pool");
      let levercoin_in_pool =
        protocol_state
        .stablecoin_amount
        .mul_div_floor(levercoin_staked, UFix64::one())
        .expect("levercoin_in_pool");
      StabilityPoolState {
        stablecoin_in_pool,
        levercoin_in_pool,
        lp_token_supply: UFix64::new(lp_token_supply),
        stablecoin_nav: protocol_state.stablecoin_nav,
        levercoin_nav: protocol_state.levercoin_nav,
      }
    }
  }

  proptest! {
    #[test]
    fn lp_token_nav_ok(
      StabilityPoolState {
        stablecoin_in_pool,
        levercoin_in_pool,
        lp_token_supply,
        stablecoin_nav,
        levercoin_nav,
      } in protocol_state(()).prop_flat_map(make_stability_pool_state),
    ) {
      let nav = lp_token_nav(
        stablecoin_nav,
        stablecoin_in_pool,
        levercoin_nav,
        levercoin_in_pool,
        lp_token_supply,
      );
      assert!(nav.is_ok_and(|x| x > UFix64::zero()));
    }

    #[test]
    fn lp_token_nav_additive(
      StabilityPoolState {
        stablecoin_in_pool,
        levercoin_in_pool,
        lp_token_supply,
        stablecoin_nav,
        levercoin_nav,
      } in protocol_state(()).prop_flat_map(make_stability_pool_state),
    ) {
      let nav = lp_token_nav(
        stablecoin_nav,
        stablecoin_in_pool,
        levercoin_nav,
        levercoin_in_pool,
        lp_token_supply,
      ).expect("lp_token_nav");
      let stable_nav = lp_token_nav(
        stablecoin_nav,
        stablecoin_in_pool,
        levercoin_nav,
        UFix64::zero(),
        lp_token_supply,
      ).expect("nav_stable");
      let lever_nav = lp_token_nav(
        stablecoin_nav,
        UFix64::zero(),
        levercoin_nav,
        levercoin_in_pool,
        lp_token_supply,
      ).expect("nav_stable");
      // An only stablecoin pool and only levercoin pool should sum to the same
      // NAV given equivalent token supply
      let sum = stable_nav.checked_add(&lever_nav).expect("sum");
      assert!(eq_tolerance!(nav, sum, N6, UFix64::new(1)));
    }

    #[test]
    fn lp_token_nav_proportional(
      StabilityPoolState {
        stablecoin_in_pool,
        levercoin_in_pool,
        lp_token_supply,
        stablecoin_nav,
        levercoin_nav,
      } in protocol_state(()).prop_flat_map(make_stability_pool_state),
    ) {
      let two: UFix64<N6> = UFix64::new(2_000_000);
      let double_stable = stablecoin_in_pool
        .mul_div_floor(two, UFix64::one())
        .expect("double_stable");
      let double_lever = levercoin_in_pool
        .mul_div_floor(two, UFix64::one())
        .expect("double_lever");
      let nav = lp_token_nav(
        stablecoin_nav,
        stablecoin_in_pool,
        levercoin_nav,
        levercoin_in_pool,
        lp_token_supply,
      ).expect("lp_token_nav");
      let double_nav = lp_token_nav(
        stablecoin_nav,
        double_stable,
        levercoin_nav,
        double_lever,
        lp_token_supply,
      ).expect("lp_token_nav");
      let double_nav_expect = nav
        .mul_div_floor(two, UFix64::one())
        .expect("double_nav_expect");
      // NAV should upscale proportionally with stake
      assert!(eq_tolerance!(double_nav_expect, double_nav, N6, UFix64::new(2)));

      let double_supply = lp_token_supply
        .mul_div_floor(two, UFix64::one())
        .expect("half_supply");
      let half_nav = lp_token_nav(
        stablecoin_nav,
        stablecoin_in_pool,
        levercoin_nav,
        levercoin_in_pool,
        double_supply,
      ).expect("lp_token_nav");
      let half_nav_expect = nav
        .mul_div_floor(UFix64::one(), two)
        .expect("half_nav_expect");
      // NAV should downscale proportionally with LP token supply doubling
      assert!(eq_tolerance!(half_nav_expect, half_nav, N6, UFix64::new(1)));
    }
  }

  #[test]
  fn amount_stable_to_swap_low_staked() -> Result<()> {
    let stablecoin_in_pool: UFix64<N6> = UFix64::new(69_999_000);
    let target_stability_threshold: UFix64<N2> = UFix64::new(130);
    let current_stablecoin_supply: UFix64<N6> = UFix64::new(12_677_000_000);
    let total_value_locked: UFix64<N9> = UFix64::new(14_578_550_000_000);
    let amount = amount_stable_to_swap(
      stablecoin_in_pool,
      target_stability_threshold,
      current_stablecoin_supply,
      total_value_locked,
    )?;
    assert_eq!(amount, stablecoin_in_pool);
    Ok(())
  }

  #[test]
  fn amount_stable_to_swap_some_staked() -> Result<()> {
    let stablecoin_in_pool: UFix64<N6> = UFix64::new(9_006_000_000);
    let target_stability_threshold: UFix64<N2> = UFix64::new(130);
    let current_stablecoin_supply: UFix64<N6> = UFix64::new(12_677_000_000);
    let total_value_locked: UFix64<N9> = UFix64::new(14_578_550_000_000);
    let amount = amount_stable_to_swap(
      stablecoin_in_pool,
      target_stability_threshold,
      current_stablecoin_supply,
      total_value_locked,
    )?;
    assert_eq!(amount, UFix64::new(1_462_730_770));
    Ok(())
  }

  #[test]
  fn amount_stable_to_swap_all_staked() -> Result<()> {
    let stablecoin_in_pool: UFix64<N6> = UFix64::new(11_896_111_000);
    let target_stability_threshold: UFix64<N2> = UFix64::new(130);
    let current_stablecoin_supply: UFix64<N6> = stablecoin_in_pool;
    let total_value_locked: UFix64<N9> = UFix64::new(14_578_550_000_000);
    let amount = amount_stable_to_swap(
      stablecoin_in_pool,
      target_stability_threshold,
      current_stablecoin_supply,
      total_value_locked,
    )?;
    assert_eq!(amount, UFix64::new(681_841_770));
    Ok(())
  }

  #[test]
  fn amount_lever_to_swap_none() -> Result<()> {
    let levercoin_in_pool = UFix64::zero();
    let max_swappable_stablecoin = UFix64::new(619_882_000);
    let levercoin_nav = PriceRange::one(UFix64::new(14_591_006));
    let got = amount_lever_to_swap(
      levercoin_in_pool,
      levercoin_nav,
      max_swappable_stablecoin,
    )?;
    assert_eq!(levercoin_in_pool, got);
    Ok(())
  }

  #[test]
  fn amount_lever_to_swap_more() -> Result<()> {
    let levercoin_in_pool = UFix64::new(78_119_200_118);
    let max_swappable_stablecoin = UFix64::new(619_882_000);
    let levercoin_nav = PriceRange::one(UFix64::new(149_106_000));
    let expected = max_swappable_stablecoin
      .mul_div_floor(UFix64::one(), levercoin_nav.lower)
      .expect("max_levercoin");
    let got = amount_lever_to_swap(
      levercoin_in_pool,
      levercoin_nav,
      max_swappable_stablecoin,
    )?;
    assert_eq!(expected, got);
    Ok(())
  }

  #[test]
  fn amount_lever_to_swap_less() -> Result<()> {
    let levercoin_in_pool = UFix64::new(19_200_118);
    let max_swappable_stablecoin = UFix64::new(619_882_000);
    let levercoin_nav = PriceRange::one(UFix64::new(149_106));
    let got = amount_lever_to_swap(
      levercoin_in_pool,
      levercoin_nav,
      max_swappable_stablecoin,
    )?;
    assert_eq!(levercoin_in_pool, got);
    Ok(())
  }
}
