use fix::prelude::*;

use crate::conversion::SwapConversion;
use crate::error::CoreError;
use crate::error::CoreError::{LpTokenNav, LpTokenOut, TokenWithdraw};
use crate::fees::controller::FeeExtract;
use crate::pyth::PriceRange;
#[cfg(any(test, feature = "offchain"))]
use crate::util::max_scaled_input;

/// Computes NAV for the earn pool's LP token.
///
/// ```txt
///                  stablecoin_in_pool
/// lp_token_nav =  --------------------
///                   lp_token_supply
/// ```
pub fn lp_token_nav(
  stablecoin_in_pool: UFix64<N6>,
  lp_token_supply: UFix64<N6>,
) -> Result<UFix64<N6>, CoreError> {
  if lp_token_supply == UFix64::zero() {
    Ok(UFix64::one())
  } else {
    stablecoin_in_pool
      .mul_div_ceil(UFix64::one(), lp_token_supply)
      .ok_or(LpTokenNav)
  }
}

/// Simply divides the amount of stablecoin being deposited by the LP token NAV.
pub fn lp_token_out(
  amount_stablecoin_in: UFix64<N6>,
  lp_token_nav: UFix64<N6>,
) -> Result<UFix64<N6>, CoreError> {
  lp_token_out_inner(amount_stablecoin_in, lp_token_nav).ok_or(LpTokenOut)
}

fn lp_token_out_inner(
  amount_stablecoin_in: UFix64<N6>,
  lp_token_nav: UFix64<N6>,
) -> Option<UFix64<N6>> {
  (lp_token_nav != UFix64::zero())
    .then_some(amount_stablecoin_in)
    .and_then(|amt| amt.mul_div_floor(UFix64::one(), lp_token_nav))
}

/// Computes amount of token to withdraw, given a user's LP equity in the pool.
pub fn amount_token_to_withdraw(
  user_lp_token_amount: UFix64<N6>,
  lp_token_supply: UFix64<N6>,
  pool_amount: UFix64<N6>,
) -> Result<UFix64<N6>, CoreError> {
  amount_token_to_withdraw_inner(
    user_lp_token_amount,
    lp_token_supply,
    pool_amount,
  )
  .ok_or(TokenWithdraw)
}

fn amount_token_to_withdraw_inner(
  user_lp_token_amount: UFix64<N6>,
  lp_token_supply: UFix64<N6>,
  pool_amount: UFix64<N6>,
) -> Option<UFix64<N6>> {
  (lp_token_supply != UFix64::zero())
    .then_some(user_lp_token_amount)
    .and_then(|amt| amt.mul_div_floor(pool_amount, lp_token_supply))
}

/// Largest LP token input withdrawing at most `cap` from the pool:
/// inverse of [`amount_token_to_withdraw`].
///
/// # Errors
/// * Zero pool amount
#[cfg(any(test, feature = "offchain"))]
pub fn max_lp_token_for_withdrawal(
  cap: UFix64<N6>,
  lp_token_supply: UFix64<N6>,
  pool_amount: UFix64<N6>,
) -> Result<UFix64<N6>, CoreError> {
  max_scaled_input(cap, pool_amount, lp_token_supply).ok_or(TokenWithdraw)
}

/// Computes a stablecoin target based on levercoin in pool.
/// Compares to max mintable stablecoin and returns lesser of the two.
pub fn amount_lever_to_swap(
  levercoin_in_pool: UFix64<N6>,
  levercoin_nav: PriceRange<N9>,
  max_swappable_stablecoin: UFix64<N6>,
) -> Result<UFix64<N6>, CoreError> {
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

/// Extracts withdrawal fee from stablecoin amount.
pub fn stablecoin_withdrawal_fee(
  stablecoin_to_withdraw: UFix64<N6>,
  withdrawal_fee: UFix64<N4>,
) -> Result<FeeExtract<N6>, CoreError> {
  FeeExtract::new(withdrawal_fee, stablecoin_to_withdraw)
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

    /// `amount_token_to_withdraw` rounds down: `payout * supply <= user_lp * pool`.
    #[test]
    fn amount_token_to_withdraw_floor_favors_protocol(
      user_lp in token_amount(),
      supply in token_amount(),
      pool in token_amount(),
    ) {
      if let Ok(payout) = amount_token_to_withdraw(user_lp, supply, pool) {
        let withdrawn = u128::from(payout.bits) * u128::from(supply.bits);
        let share = u128::from(user_lp.bits) * u128::from(pool.bits);
        prop_assert!(withdrawn <= share);
      }
    }

    /// `lp_token_out` rounds down: `tokens * nav <= amount * one`.
    #[test]
    fn lp_token_out_floor_favors_protocol(
      amount in token_amount(),
      nav in token_amount(),
    ) {
      if let Ok(tokens) = lp_token_out(amount, nav) {
        let issued = u128::from(tokens.bits) * u128::from(nav.bits);
        let owed = u128::from(amount.bits) * UFix128::<N6>::one().bits;
        prop_assert!(issued <= owed);
      }
    }
  }

  #[allow(dead_code)]
  #[derive(Debug)]
  struct EarnPoolState {
    pub stablecoin_in_pool: UFix64<N6>,
    pub lp_token_supply: UFix64<N6>,
  }

  fn pct_staked(min: UFix64<N2>, max: UFix64<N2>) -> BoxedStrategy<UFix64<N2>> {
    (min.bits..max.bits).prop_map(UFix64::new).boxed()
  }

  prop_compose! {
    pub fn make_earn_pool_state(protocol_state: ProtocolState)(
      lp_token_supply in 0..protocol_state.stablecoin_amount.bits,
      stablecoin_staked in pct_staked(UFix64::new(30), UFix64::new(99)),
    ) -> EarnPoolState {
      let stablecoin_in_pool =
        protocol_state
        .stablecoin_amount
        .mul_div_floor(stablecoin_staked, UFix64::one())
        .expect("stablecoin_in_pool");
      EarnPoolState {
        stablecoin_in_pool,
        lp_token_supply: UFix64::new(lp_token_supply),
      }
    }
  }

  #[test]
  fn amount_lever_to_swap_none() -> Result<(), CoreError> {
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
  fn amount_lever_to_swap_more() -> Result<(), CoreError> {
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
  fn amount_lever_to_swap_less() -> Result<(), CoreError> {
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

  proptest! {
    #[test]
    fn lp_token_nav_ok(
      EarnPoolState {
        stablecoin_in_pool,
        lp_token_supply,
      } in protocol_state(()).prop_flat_map(make_earn_pool_state),
    ) {
      let nav = lp_token_nav(
        stablecoin_in_pool,
        lp_token_supply,
      );
      assert!(nav.is_ok_and(|x| x > UFix64::zero()));
    }

    #[test]
    fn lp_token_nav_proportional(
      EarnPoolState {
        stablecoin_in_pool,
        lp_token_supply,
      } in protocol_state(()).prop_flat_map(make_earn_pool_state),
    ) {
      let two: UFix64<N6> = UFix64::new(2_000_000);
      let double_stable = stablecoin_in_pool
        .mul_div_floor(two, UFix64::one())
        .expect("double_stable");
      let nav = lp_token_nav(
        stablecoin_in_pool,
        lp_token_supply,
      ).expect("lp_token_nav");
      let double_nav = lp_token_nav(
        double_stable,
        lp_token_supply,
      ).expect("lp_token_nav");
      let double_nav_expect = nav
        .mul_div_floor(two, UFix64::one())
        .expect("double_nav_expect");
      // NAV should upscale proportionally with stake
      assert!(eq_tolerance!(double_nav_expect, double_nav, N6, UFix64::new(2)));

      let double_supply = lp_token_supply
        .mul_div_floor(two, UFix64::one())
        .expect("double_supply");
      let half_nav = lp_token_nav(
        stablecoin_in_pool,
        double_supply,
      ).expect("lp_token_nav");
      let half_nav_expect = nav
        .mul_div_floor(UFix64::one(), two)
        .expect("half_nav_expect");
      // NAV should downscale proportionally with LP token supply doubling
      assert!(eq_tolerance!(half_nav_expect, half_nav, N6, UFix64::new(1)));
    }
  }
}
