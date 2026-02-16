use anchor_lang::prelude::*;
use fix::prelude::*;
use fix::typenum::Integer;
use serde::{Deserialize, Serialize};

use crate::error::CoreError::{SlippageArithmetic, SlippageExceeded};

/// Client specified slippage tolerance paired with expected token amount.
#[derive(Debug, AnchorSerialize, AnchorDeserialize, Serialize, Deserialize)]
pub struct SlippageConfig {
  pub expected_token_out: UFixValue64,
  pub slippage_tolerance: UFixValue64,
}

impl SlippageConfig {
  #[must_use]
  pub fn new<Exp: Integer>(
    expected_token_out: UFix64<Exp>,
    slippage_tolerance: UFix64<N4>,
  ) -> SlippageConfig {
    SlippageConfig {
      expected_token_out: expected_token_out.into(),
      slippage_tolerance: slippage_tolerance.into(),
    }
  }

  pub fn expected_token_out<Exp: Integer>(&self) -> Result<UFix64<Exp>> {
    self.expected_token_out.try_into()
  }

  pub fn slippage_tolerance(&self) -> Result<UFix64<N4>> {
    self.slippage_tolerance.try_into()
  }

  /// Checks token amount against the configured lowest tolerable amount
  pub fn validate_token_out<Exp: Integer>(
    &self,
    token_out: UFix64<Exp>,
  ) -> Result<()> {
    let expected = self.expected_token_out()?;
    let tolerance = self.slippage_tolerance()?;
    // Invert slippage and multiply with expected amount
    let tolerable_amount = UFix64::<N4>::one()
      .checked_sub(&tolerance)
      .and_then(|factor| expected.mul_div_floor(factor, UFix64::one()))
      .ok_or(SlippageArithmetic)?;
    if token_out >= tolerable_amount {
      Ok(())
    } else {
      Err(SlippageExceeded.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use fix::prelude::*;

  use crate::error::CoreError::SlippageExceeded;
  use crate::slippage_config::SlippageConfig;

  const ONE_PERCENT: UFix64<N4> = UFix64::constant(100);

  #[test]
  fn slippage_exact_boundary_pos() {
    // 1% on 1.0 where minimum acceptable is 0.99
    let config = SlippageConfig::new(UFix64::<N6>::one(), ONE_PERCENT);
    let amount = UFix64::<N6>::new(990_000);
    assert!(config.validate_token_out(amount).is_ok());
  }

  #[test]
  fn slippage_one_below_boundary_neg() {
    // One unit below the boundary
    let config = SlippageConfig::new(UFix64::<N6>::one(), ONE_PERCENT);
    let amount = UFix64::<N6>::new(989_999);
    assert_eq!(
      config.validate_token_out(amount),
      Err(SlippageExceeded.into())
    );
  }

  #[test]
  fn slippage_zero_tolerance_exact_pos() {
    let config = SlippageConfig::new(UFix64::<N6>::one(), UFix64::zero());
    assert!(config.validate_token_out(UFix64::<N6>::one()).is_ok());
  }

  #[test]
  fn slippage_zero_tolerance_below_neg() {
    let config = SlippageConfig::new(UFix64::<N6>::one(), UFix64::zero());
    let amount = UFix64::<N6>::new(999_999);
    assert_eq!(
      config.validate_token_out(amount),
      Err(SlippageExceeded.into())
    );
  }

  #[test]
  fn slippage_favorable_execution_pos() {
    // Positive slippage (more than expected)
    let config = SlippageConfig::new(UFix64::<N6>::one(), UFix64::new(50));
    let amount = UFix64::<N6>::new(1_500_000);
    assert!(config.validate_token_out(amount).is_ok());
  }

  #[test]
  fn slippage_full_tolerance_accepts_zero_pos() {
    // 100% tolerance accepts any output including zero
    let config = SlippageConfig::new(UFix64::<N6>::one(), UFix64::<N4>::one());
    assert!(config.validate_token_out(UFix64::<N6>::zero()).is_ok());
  }
}
