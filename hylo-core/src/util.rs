use anchor_spl::token::Mint;
use fix::prelude::*;
#[cfg(any(test, feature = "offchain"))]
use fix::typenum::Integer;

use crate::error::CoreError;
use crate::error::CoreError::ExoAmountNormalization;

/// Largest `x` with `floor(x * num / den) <= cap`.
///
/// ```txt
/// x = ceil((cap + 1) * den / num) - 1
/// ```
#[cfg(any(test, feature = "offchain"))]
pub(crate) fn max_scaled_input<Exp: Integer, RExp: Integer>(
  cap: UFix64<Exp>,
  num: UFix64<RExp>,
  den: UFix64<RExp>,
) -> Option<UFix64<Exp>> {
  let atom = UFix64::new(1);
  (num != UFix64::zero())
    .then_some(cap)
    .and_then(|cap| cap.checked_add(&atom))
    .and_then(|bound| bound.mul_div_ceil(den, num))
    .and_then(|max| max.checked_sub(&atom))
}

/// Bridges runtime mint decimals to typed `UFix64<N9>`.
///
/// # Errors
/// * Unsupported decimal count or conversion overflow
pub fn normalize_mint_exp(
  mint: &Mint,
  amount: u64,
) -> Result<UFix64<N9>, CoreError> {
  match mint.decimals {
    2 => UFix64::<N2>::new(amount).checked_convert(),
    3 => UFix64::<N3>::new(amount).checked_convert(),
    4 => UFix64::<N4>::new(amount).checked_convert(),
    5 => UFix64::<N5>::new(amount).checked_convert(),
    6 => UFix64::<N6>::new(amount).checked_convert(),
    7 => UFix64::<N7>::new(amount).checked_convert(),
    8 => UFix64::<N8>::new(amount).checked_convert(),
    9 => Some(UFix64::<N9>::new(amount)),
    10 => UFix64::<N10>::new(amount).checked_convert(),
    _ => None,
  }
  .ok_or(ExoAmountNormalization)
}

/// Converts typed `UFix64<N9>` back to a raw `u64` in the mint's native
/// decimals.
///
/// # Errors
/// * Unsupported decimal count
pub fn denormalize_mint_exp(
  mint: &Mint,
  amount: UFix64<N9>,
) -> Result<u64, CoreError> {
  match mint.decimals {
    2 => amount.checked_convert::<N2>().map(|o| o.bits),
    3 => amount.checked_convert::<N3>().map(|o| o.bits),
    4 => amount.checked_convert::<N4>().map(|o| o.bits),
    5 => amount.checked_convert::<N5>().map(|o| o.bits),
    6 => amount.checked_convert::<N6>().map(|o| o.bits),
    7 => amount.checked_convert::<N7>().map(|o| o.bits),
    8 => amount.checked_convert::<N8>().map(|o| o.bits),
    9 => Some(amount.bits),
    10 => amount.checked_convert::<N10>().map(|o| o.bits),
    _ => None,
  }
  .ok_or(ExoAmountNormalization)
}

/// Converts typed `UFix64<N9>` back to a raw `u64` in the mint's native
/// decimals, rounding up.
///
/// When splitting one normalized amount, ceil exactly one part and floor
/// the rest so the denormalized parts sum to the original amount.
///
/// # Errors
/// * Unsupported decimal count
pub fn denormalize_mint_exp_ceil(
  mint: &Mint,
  amount: UFix64<N9>,
) -> Result<u64, CoreError> {
  match mint.decimals {
    2 => amount.checked_convert_ceil::<N2>().map(|o| o.bits),
    3 => amount.checked_convert_ceil::<N3>().map(|o| o.bits),
    4 => amount.checked_convert_ceil::<N4>().map(|o| o.bits),
    5 => amount.checked_convert_ceil::<N5>().map(|o| o.bits),
    6 => amount.checked_convert_ceil::<N6>().map(|o| o.bits),
    7 => amount.checked_convert_ceil::<N7>().map(|o| o.bits),
    8 => amount.checked_convert_ceil::<N8>().map(|o| o.bits),
    9 => Some(amount.bits),
    10 => amount.checked_convert_ceil::<N10>().map(|o| o.bits),
    _ => None,
  }
  .ok_or(ExoAmountNormalization)
}

#[macro_export]
macro_rules! eq_tolerance {
  ($l:expr, $r:expr, $place:ty, $tol:expr) => {{
    let diff = $l.convert::<$place>().abs_diff(&$r.convert::<$place>());
    diff <= $tol
  }};
}

#[cfg(test)]
pub mod proptest {
  use fix::prelude::*;
  use proptest::prelude::*;

  use crate::exchange_math::collateral_ratio;
  use crate::pyth::PriceRange;

  /// Represents a possible state of the protocol, collateral, and tokens.
  /// Always holds the Hylo invariant: `ns * ps = nx * px + nh * ph`.
  #[derive(Debug)]
  pub struct ProtocolState {
    pub usd_sol_price: UFix64<N9>,
    pub stablecoin_amount: UFix64<N6>,
    pub stablecoin_nav: UFix64<N9>,
    pub levercoin_amount: UFix64<N6>,
    pub levercoin_nav: UFix64<N9>,
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
    pub fn next_target_collateral_ratio(&self) -> Option<UFix64<N9>> {
      let current = self.collateral_ratio()?;
      let one = UFix64::<N9>::one();
      let next = current.mul_div_ceil(UFix64::new(900_000_000), one)?;
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

  pub fn usd_sol_price() -> BoxedStrategy<UFix64<N9>> {
    (10_000_000_000u64..2_500_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  pub fn token_amount() -> BoxedStrategy<UFix64<N6>> {
    (1_0000u64..5_000_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  pub fn stablecoin_nav() -> BoxedStrategy<UFix64<N9>> {
    (800_000_000u64..1_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  pub fn levercoin_nav() -> BoxedStrategy<UFix64<N9>> {
    (100_000u64..1_000_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  /// Realistic LST/SOL price between 1.0 and 5.0.
  pub fn lst_sol_price() -> BoxedStrategy<UFix64<N9>> {
    (1_000_000_000u64..5_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  /// `PriceRange` centered at $1 with a tight symbolic confidence interval.
  pub fn dollar_centered_price_range() -> BoxedStrategy<PriceRange<N9>> {
    (1u64..5_000_000u64)
      .prop_filter_map("from_conf within range", |conf| {
        PriceRange::from_conf(UFix64::<N9>::one(), UFix64::new(conf)).ok()
      })
      .boxed()
  }

  /// Extreme LST/SOL price including depeg.
  pub fn lst_sol_price_extreme() -> BoxedStrategy<UFix64<N9>> {
    (1u64..u64::MAX / 2).prop_map(UFix64::new).boxed()
  }

  /// Realistic LST amount: dust to 100K tokens.
  /// Upper bound prevents overflow in conversions with high prices.
  pub fn lst_amount() -> BoxedStrategy<UFix64<N9>> {
    (1_000u64..100_000_000_000_000u64)
      .prop_map(UFix64::new)
      .boxed()
  }

  /// Extreme LST amount from 1 unit to half of max.
  pub fn lst_amount_extreme() -> BoxedStrategy<UFix64<N9>> {
    (1u64..u64::MAX / 2).prop_map(UFix64::new).boxed()
  }
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::program_option::COption;
  use anchor_lang::AccountDeserialize;
  use anchor_spl::token::spl_token::solana_program::program_pack::Pack;
  use anchor_spl::token::spl_token::state::Mint as SplMint;
  use anchor_spl::token::Mint;
  use anyhow::Result;
  use fix::aliases::si::{Micro, Nano};
  use fix::prelude::*;
  use proptest::prelude::*;

  use super::{
    denormalize_mint_exp, denormalize_mint_exp_ceil, normalize_mint_exp,
  };
  use crate::asset_swap_config::AssetSwapConfig;
  use crate::error::CoreError::SlippageExceeded;
  use crate::fees::controller::FeeExtract;
  use crate::slippage_config::SlippageConfig;

  fn test_mint(decimals: u8) -> Result<Mint> {
    let spl_mint = SplMint {
      mint_authority: COption::None,
      supply: 0,
      decimals,
      is_initialized: true,
      freeze_authority: COption::None,
    };
    let mut buf = [0u8; SplMint::LEN];
    SplMint::pack(spl_mint, &mut buf)?;
    Ok(Mint::try_deserialize(&mut buf.as_slice())?)
  }

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
    assert_eq!(out, Err(SlippageExceeded));
  }

  #[test]
  fn slippage_pos() {
    let config =
      SlippageConfig::new(UFix64::<N6>::new(99_411_501), UFix64::new(10));
    let amount = UFix64::<N6>::new(99_312_089);
    let out = config.validate_token_out(amount);
    assert!(out.is_ok());
  }

  proptest! {
    #[test]
    fn fee_split_denorm_conserves(
      amount in 1u64..1_000_000_000_000,
      decimals in 2u8..=9,
      fee_bps in 1u64..=100,
    ) {
      let mint = test_mint(decimals).expect("mint");

      // Normalize
      let norm = normalize_mint_exp(&mint, amount).expect("normalize");

      // Extract fees
      let config = AssetSwapConfig::new(UFixValue64::new(fee_bps, -4))
        .expect("config");
      let FeeExtract {
        fees_extracted,
        amount_remaining,
      } = config.apply_fee(norm).expect("apply_fee");

      // Denormalize, rounding the fee up and the remainder down
      let fee_out =
        denormalize_mint_exp_ceil(&mint, fees_extracted).expect("ceil");
      let amount_out =
        denormalize_mint_exp(&mint, amount_remaining).expect("floor");

      // Operation is lossless
      prop_assert_eq!(fee_out + amount_out, amount);
    }
  }
}
