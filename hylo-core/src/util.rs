#[cfg(test)]
pub mod proptest {
  use crate::exchange_math::collateral_ratio;
  use proptest::prelude::*;

  use fix::prelude::*;
  use fix::typenum::{N2, N6, N8, N9};

  #[macro_export]
  macro_rules! eq_tolerance {
    ($l:expr, $r:expr, $place:ty, $tol:expr) => {{
      let diff = $l.convert::<$place>().abs_diff(&$r.convert::<$place>());
      diff <= $tol
    }};
  }

  /// Represents a possible state of the protocol, collateral, and tokens.
  /// Always holds the Hylo invariant: `ns * ps = nx * px + nh * ph`.
  #[derive(Debug)]
  pub struct ProtocolState {
    pub usd_sol_price: UFix64<N8>,
    pub stablecoin_amount: UFix64<N6>,
    pub stablecoin_nav: UFix64<N6>,
    pub levercoin_amount: UFix64<N6>,
    pub levercoin_nav: UFix64<N6>,
  }

  impl ProtocolState {
    #[must_use]
    pub fn total_sol(&self) -> Option<UFix64<N9>> {
      let stablecoin_cap = self
        .stablecoin_amount
        .mul_div_floor(self.stablecoin_nav, UFix64::one())?;
      let levercoin_cap = self
        .levercoin_amount
        .mul_div_floor(self.levercoin_nav, UFix64::one())?;
      let tvl = stablecoin_cap.checked_add(&levercoin_cap)?;
      tvl
        .convert()
        .mul_div_floor(UFix64::one(), self.usd_sol_price)
    }

    #[must_use]
    pub fn collateral_ratio(&self) -> Option<UFix64<N9>> {
      collateral_ratio(
        self.total_sol()?,
        self.usd_sol_price,
        self.stablecoin_amount,
      )
      .ok()
    }

    #[must_use]
    pub fn next_target_collateral_ratio(&self) -> Option<UFix64<N2>> {
      let current = self.collateral_ratio()?.convert::<N2>();
      let one = UFix64::<N2>::new(100);
      let next = current.mul_div_ceil(UFix64::new(90), one)?;
      if next < one {
        None
      } else {
        Some(next)
      }
    }
  }

  prop_compose! {
    /// Fixed stablecoin NAV at $1 only.
    pub fn protocol_state(_: ())
      (usd_sol_price in usd_sol_price(),
      stablecoin_amount in token_amount(),
      levercoin_amount in token_amount(),
      levercoin_nav in levercoin_nav()) -> ProtocolState {
       ProtocolState {
         usd_sol_price,
         stablecoin_amount,
         stablecoin_nav: UFix64::one(),
         levercoin_amount,
         levercoin_nav,
       }
    }
  }

  prop_compose! {
    /// Makes it possible to have depegged stablecoin NAV in the state.
    pub fn protocol_state_depeg(_: ())
      (usd_sol_price in usd_sol_price(),
      stablecoin_amount in token_amount(),
      stablecoin_nav in stablecoin_nav(),
      levercoin_amount in token_amount(),
      levercoin_nav in levercoin_nav()) -> ProtocolState {
       ProtocolState {
         usd_sol_price,
         stablecoin_amount,
         stablecoin_nav,
         levercoin_amount,
         levercoin_nav,
       }
    }
  }

  pub fn usd_sol_price() -> BoxedStrategy<UFix64<N8>> {
    (1_000_000_000u64..250_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  pub fn lst_sol_price() -> BoxedStrategy<UFix64<N9>> {
    (1_000_000_000u64..2_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  pub fn token_amount() -> BoxedStrategy<UFix64<N6>> {
    (1_0000u64..5_000_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  pub fn lst_amount() -> BoxedStrategy<UFix64<N9>> {
    (1_000u64..1_000_000_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  pub fn stablecoin_nav() -> BoxedStrategy<UFix64<N6>> {
    (800_000u64..1_000_000u64).prop_map(UFix64::new).boxed()
  }

  pub fn levercoin_nav() -> BoxedStrategy<UFix64<N6>> {
    (100u64..1_000_000_000u64).prop_map(UFix64::new).boxed()
  }
}

#[cfg(test)]
mod tests {
  use crate::error::CoreError::SlippageExceeded;
  use crate::slippage_config::SlippageConfig;

  use fix::aliases::si::{Micro, Nano};
  use fix::prelude::*;

  #[test]
  fn one_nano() {
    let one = Nano::<u64>::one();
    assert_eq!("1000000000x10^-9", format!("{one:?}"));
  }

  #[test]
  fn precision6_unit() {
    let one = Micro::<u64>::one();
    let unit = one * one;
    assert_eq!(one, unit.convert());
  }

  #[test]
  fn precision6_overflow_guard() {
    let max = Micro::new(u128::MAX);
    assert!(max.checked_mul(&max).is_none());
  }

  #[test]
  fn neg_sub_underflows() {
    let last = Nano::new(169_120_000_u64);
    let now = Nano::new(151_444_800_u64);
    let sub = now.checked_sub(&last);
    assert!(sub.is_none());
  }

  #[test]
  fn slippage_neg() {
    let config =
      SlippageConfig::new(UFix64::<N6>::new(1_201_346), UFix64::new(20));
    let amount = UFix64::<N6>::new(1_198_942);
    let out = config.validate_token_out(amount);
    assert_eq!(out, Err(SlippageExceeded.into()));
  }

  #[test]
  fn slippage_pos() {
    let config =
      SlippageConfig::new(UFix64::<N6>::new(99_411_501), UFix64::new(10));
    let amount = UFix64::<N6>::new(99_312_089);
    let out = config.validate_token_out(amount);
    assert!(out.is_ok());
  }
}
