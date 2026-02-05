use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::{FundingRateApply, FundingRateValidation};

/// Per-epoch funding rate for exogenous collateral without native yield.
#[derive(
  Copy, Clone, Debug, PartialEq, InitSpace, AnchorSerialize, AnchorDeserialize,
)]
pub struct FundingRateConfig {
  rate: UFixValue64,
}

/// Maximum per-epoch rate (~10% annualized at 182 epochs/year)
const MAX_RATE: UFix64<N8> = UFix64::constant(60_000);

impl FundingRateConfig {
  #[must_use]
  pub fn new(rate: UFixValue64) -> FundingRateConfig {
    FundingRateConfig { rate }
  }

  /// Per-epoch funding rate.
  ///
  /// # Errors
  /// * Invalid rate data
  pub fn rate(&self) -> Result<UFix64<N8>> {
    self.rate.try_into()
  }

  /// Applies the funding rate to collateral value in USD.
  ///
  /// # Errors
  /// * Arithmetic overflow
  pub fn apply<Exp>(
    &self,
    collateral_value_usd: UFix64<Exp>,
  ) -> Result<UFix64<Exp>>
  where
    UFix64<Exp>: FixExt,
  {
    let rate = self.rate()?;
    collateral_value_usd
      .mul_div_floor(rate, UFix64::<N8>::one())
      .ok_or(FundingRateApply.into())
  }

  /// Validates rate is positive and at most ~10% annualized.
  ///
  /// # Errors
  /// * Rate is zero or exceeds maximum
  pub fn validate(&self) -> Result<FundingRateConfig> {
    let rate = self.rate()?;
    if rate > UFix64::zero() && rate <= MAX_RATE {
      Ok(*self)
    } else {
      Err(FundingRateValidation.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn apply_7_percent_annual() -> Result<()> {
    let rate = UFix64::<N8>::new(38462);
    let config = FundingRateConfig::new(rate.into());
    let collateral = UFix64::<N6>::new(1_000_000_000_000);
    let funding = config.apply(collateral)?;
    assert_eq!(funding, UFix64::new(384_620_000));
    Ok(())
  }

  #[test]
  fn validate_rate_pos() -> Result<()> {
    let rate = UFix64::<N8>::new(38462);
    let config = FundingRateConfig::new(rate.into());
    config.validate()?;
    Ok(())
  }

  #[test]
  fn validate_neg_zero() {
    let config = FundingRateConfig::new(UFix64::<N8>::zero().into());
    assert_eq!(config.validate(), Err(FundingRateValidation.into()));
  }

  #[test]
  fn validate_neg_high() {
    let config = FundingRateConfig::new((MAX_RATE + UFix64::new(1)).into());
    assert_eq!(config.validate(), Err(FundingRateValidation.into()));
  }
}
