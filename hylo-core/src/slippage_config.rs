use crate::error::CoreError::{SlippageArithmetic, SlippageExceeded};

use anchor_lang::prelude::*;
use fix::prelude::*;
use fix::typenum::Integer;

/// Client specified slippage tolerance paired with expected token amount.
#[derive(Debug, AnchorSerialize, AnchorDeserialize)]
pub struct SlippageConfig {
  expected_token_out: UFixValue64,
  slippage_tolerance: UFixValue64,
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
