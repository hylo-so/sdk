use crate::error::CoreError::{
  CollateralRatio, MaxMintable, MaxSwappable, StablecoinNav,
  TargetCollateralRatioTooLow, TotalValueLocked,
};

use anchor_lang::prelude::*;
use fix::prelude::*;

/// Computes the current collateral ratio (CR) of the protocol.
///   `CR = total_sol_usd / stablecoin_cap`
///
/// NB: If stablecoin supply is zero, returns `u64::MAX` to simulate infinity.
pub fn collateral_ratio(
  total_sol: UFix64<N9>,
  usd_sol_price: UFix64<N8>,
  amount_stablecoin: UFix64<N6>,
) -> Result<UFix64<N9>> {
  if amount_stablecoin == UFix64::zero() {
    Ok(UFix64::new(u64::MAX))
  } else {
    total_sol
      .mul_div_floor(usd_sol_price, amount_stablecoin.convert())
      .ok_or(CollateralRatio.into())
  }
}

/// Multiples total SOL by the given spot price to get TVL.
pub fn total_value_locked(
  total_sol: UFix64<N9>,
  sol_usd_price: UFix64<N8>,
) -> Result<UFix64<N9>> {
  total_sol
    .mul_div_floor(sol_usd_price, UFix64::one())
    .ok_or(TotalValueLocked.into())
}

/// Given the next collateral ratio threshold below the current, determines the
/// amount of stablecoin that can safely be minted.
///
/// Finds `max_stablecoin` assuming stablecoin NAV is $1.
///   `max_stablecoin = (tvl - target_cr * cur_stablecoin) / (target_cr - 1)`
pub fn max_mintable_stablecoin(
  target_collateral_ratio: UFix64<N2>,
  total_sol: UFix64<N9>,
  usd_sol_price: UFix64<N8>,
  stablecoin_supply: UFix64<N6>,
) -> Result<UFix64<N6>> {
  if target_collateral_ratio > UFix64::one() {
    let numerator = {
      let target_supply =
        stablecoin_supply.mul_div_ceil(target_collateral_ratio, UFix64::one());
      let tvl_usd = total_sol.mul_div_floor(usd_sol_price, UFix64::one());
      tvl_usd
        .zip(target_supply)
        .and_then(|(tvl, target)| tvl.checked_sub(&target.convert()))
    };
    let denominator = target_collateral_ratio.checked_sub(&UFix64::<N2>::one());
    numerator
      .zip(denominator)
      .and_then(|(n, d)| n.checked_div(&d))
      .map(UFix64::convert)
      .ok_or(MaxMintable.into())
  } else {
    Err(TargetCollateralRatioTooLow.into())
  }
}

/// Without changing TVL, computes how much stablecoin can be swapped from
/// levercoin.
///
/// ```txt
///                   total_value_locked
/// max_swappable = -----------------------  - stablecoin_supply
///                 target_collateral_ratio
/// ```
pub fn max_swappable_stablecoin(
  target_collateral_ratio: UFix64<N2>,
  total_value_locked: UFix64<N9>,
  stablecoin_supply: UFix64<N6>,
) -> Result<UFix64<N6>> {
  total_value_locked
    .checked_div(&target_collateral_ratio)
    .and_then(|l| l.checked_sub(&stablecoin_supply.convert()))
    .map(UFix64::convert)
    .ok_or(MaxSwappable.into())
}

/// Computes the next net asset value for the Hylo levercoin, given the total
/// collateral stored in the protocol.
///
/// If the current supply of the levercoin is zero, the price is $1.
///
/// Otherwise its NAV is computed as:
///   `free_collateral = (n_collateral * p_collateral) - (n_stable * p_stable)`
///   `new_nav = free_collateral / n_lever`
#[must_use]
pub fn next_levercoin_nav(
  total_sol: UFix64<N9>,
  sol_usd_price: UFix64<N8>,
  stablecoin_supply: UFix64<N6>,
  stablecoin_nav: UFix64<N6>,
  levercoin_supply: UFix64<N6>,
) -> Option<UFix64<N6>> {
  if levercoin_supply == UFix64::zero() {
    Some(UFix64::one())
  } else {
    let collateral_value =
      total_sol.mul_div_floor(sol_usd_price, UFix64::one())?;
    let stablecoin_value =
      stablecoin_supply.mul_div_ceil(stablecoin_nav, UFix64::one())?;
    let free_collateral =
      collateral_value.checked_sub(&stablecoin_value.convert())?;
    let nav = free_collateral.mul_div_ceil(UFix64::one(), levercoin_supply)?;
    Some(nav.convert())
  }
}

/// Computes stablecoin NAV during a depeg scenario.
/// In all other modes, the price of the stablecoin is fixed to $1.
///   `NAV = total_sol * sol_usd_price / supply`
pub fn depeg_stablecoin_nav(
  total_collateral_sol: UFix64<N9>,
  sol_usd_price: UFix64<N8>,
  stablecoin_supply: UFix64<N6>,
) -> Result<UFix64<N6>> {
  total_collateral_sol
    .mul_div_floor(sol_usd_price.convert(), stablecoin_supply.convert::<N9>())
    .map(UFix64::convert)
    .ok_or(StablecoinNav.into())
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::eq_tolerance;
  use crate::error::CoreError::LevercoinNav;
  use crate::util::proptest::*;

  use anchor_lang::prelude::Result;
  use fix::prelude::typenum::N8;
  use fix::prelude::UFix64;
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn max_mintable_props(
      state in protocol_state(()),
    ) {
      if let Some(target) = state.next_target_collateral_ratio() {
        // Skip unless target CR is above 100%, not realistic otherwise
        if target > UFix64::one() {
          let total_sol = state.total_sol().expect("total_sol");
          let max = max_mintable_stablecoin(
            target,
            total_sol,
            state.usd_sol_price,
            state.stablecoin_amount,
          )?;
          let new_total_sol =
            max.mul_div_ceil(UFix64::one(), state.usd_sol_price)
            .and_then(|new_sol| new_sol.convert().checked_add(&total_sol))
            .expect("new_total");
          let new_stable = state.stablecoin_amount.checked_add(&max).expect("new_stable");
          let new_cr = collateral_ratio(new_total_sol, state.usd_sol_price, new_stable)?;
          // Checks new CR is within tolerance of 0.01
          prop_assert!(
            eq_tolerance!(target, new_cr, N2, UFix64::new(1))
          );
        }
      }
    }
  }

  #[test]
  fn max_mintable_simple() -> Result<()> {
    let target = UFix64::<N2>::new(101);
    let total_sol = UFix64::<N9>::new(1_474_848_711_762_305);
    let usd_sol_price = UFix64::<N8>::new(159_786_642_951);
    let stablecoin_supply = UFix64::<N6>::new(100_000_000);
    let max = max_mintable_stablecoin(
      target,
      total_sol,
      usd_sol_price,
      stablecoin_supply,
    )?;
    assert_eq!(UFix64::new(235_661_114_413_105_743), max);
    Ok(())
  }

  proptest! {
    #[test]
    fn levercoin_nav_invariant(
      state in protocol_state(()),
    ) {
      let total_sol = state.total_sol().expect("total_sol");
      let nav = next_levercoin_nav(
        total_sol,
        state.usd_sol_price,
        state.stablecoin_amount,
        state.stablecoin_nav,
        state.levercoin_amount
      ).expect("nav");
      prop_assert!(eq_tolerance!(state.levercoin_nav, nav, N6, UFix64::new(1)));
    }
  }

  #[test]
  fn levercoin_supply_zero() -> Result<()> {
    let jitosol_amount = UFix64::new(1010u64);
    let jitosol_price = UFix64::new(20_133_670_123_u64);
    let stablecoin_supply = UFix64::new(100u64);
    let stablecoin_nav = UFix64::one();
    let levercoin_supply = UFix64::new(0u64);
    let nav = next_levercoin_nav(
      jitosol_amount,
      jitosol_price,
      stablecoin_supply,
      stablecoin_nav,
      levercoin_supply,
    )
    .ok_or(LevercoinNav)?;
    assert_eq!(UFix64::one(), nav);
    Ok(())
  }

  #[test]
  fn collateral_ratio_low() -> Result<()> {
    let total_sol = UFix64::<N9>::new(8_217_712_567_008);
    let usd_sol_price = UFix64::<N8>::new(13_770_492_000);
    let amount_stablecoin = UFix64::<N6>::new(1_150_380_112_112);
    let cr = collateral_ratio(total_sol, usd_sol_price, amount_stablecoin)?;
    assert_eq!(UFix64::new(983_691_772), cr);
    Ok(())
  }

  #[test]
  fn collateral_ratio_high() -> Result<()> {
    let total_sol = UFix64::<N9>::new(976_123_127_719);
    let usd_sol_price = UFix64::<N8>::new(13_770_492_000);
    let amount_stablecoin = UFix64::<N6>::new(97_411_342_200);
    let cr = collateral_ratio(total_sol, usd_sol_price, amount_stablecoin)?;
    assert_eq!(UFix64::new(1_379_890_207), cr);
    Ok(())
  }

  #[test]
  fn depeg_stablecoin_low() -> Result<()> {
    let total_sol = UFix64::<N9>::new(1_666_312_671);
    let usd_sol_price = UFix64::<N8>::new(770_492_000);
    let amount_stablecoin = UFix64::<N6>::new(974_113_420_200);
    let nav =
      depeg_stablecoin_nav(total_sol, usd_sol_price, amount_stablecoin)?;
    assert_eq!(UFix64::new(13), nav);
    Ok(())
  }

  #[test]
  fn depeg_stablecoin_high() -> Result<()> {
    let total_sol = UFix64::<N9>::new(10_666_312_671);
    let usd_sol_price = UFix64::<N8>::new(770_492_000);
    let amount_stablecoin = UFix64::<N6>::new(97_411_342);
    let nav =
      depeg_stablecoin_nav(total_sol, usd_sol_price, amount_stablecoin)?;
    assert_eq!(UFix64::new(843_670), nav);
    Ok(())
  }

  #[test]
  fn max_swappable_stablecoin_normal() -> Result<()> {
    let tvl = UFix64::<N9>::new(7_552_002_260_000_000);
    let target_cr = UFix64::<N2>::new(150);
    let stablecoin = UFix64::<N6>::new(5_001_326_000_000);
    let expected = UFix64::new(33_342_173_333);
    let got = max_swappable_stablecoin(target_cr, tvl, stablecoin)?;
    assert_eq!(expected, got);
    Ok(())
  }

  #[test]
  fn max_swappable_stablecoin_mode1() -> Result<()> {
    let tvl = UFix64::<N9>::new(7_894_510_000_000);
    let target_cr = UFix64::<N2>::new(130);
    let stablecoin = UFix64::<N6>::new(5_343_990_000);
    let expected = UFix64::new(728_710_000);
    let got = max_swappable_stablecoin(target_cr, tvl, stablecoin)?;
    assert_eq!(expected, got);
    Ok(())
  }

  #[test]
  fn max_swappable_stablecoin_mode2() -> Result<()> {
    let tvl = UFix64::<N9>::new(1_000_335_000_000_000);
    let target_cr = UFix64::<N2>::new(100);
    let stablecoin = UFix64::<N6>::new(1_000_000_000_000);
    let expected = UFix64::new(335_000_000);
    let got = max_swappable_stablecoin(target_cr, tvl, stablecoin)?;
    assert_eq!(expected, got);
    Ok(())
  }
}
