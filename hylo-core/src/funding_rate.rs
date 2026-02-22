use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::{FundingRateApply, FundingRateValidation};
use crate::fee_controller::FeeExtract;

/// Per-epoch funding rate for exogenous collateral without native yield.
#[derive(
  Copy, Clone, Debug, PartialEq, InitSpace, AnchorSerialize, AnchorDeserialize,
)]
pub struct FundingRateConfig {
  rate: UFixValue64,
  fee: UFixValue64,
}

/// Maximum per-epoch rate (~10% annualized at 182 epochs/year)
const MAX_RATE: UFix64<N9> = UFix64::constant(600_000);

/// Maximum fee exacted against funding rate
const MAX_FEE: UFix64<N4> = UFix64::constant(10_000);

impl FundingRateConfig {
  #[must_use]
  pub fn new(rate: UFixValue64, fee: UFixValue64) -> FundingRateConfig {
    FundingRateConfig { rate, fee }
  }

  /// Per-epoch funding rate.
  ///
  /// # Errors
  /// * Invalid rate data
  pub fn rate(&self) -> Result<UFix64<N9>> {
    self.rate.try_into()
  }

  /// Percentage of funding rate harvest to divert to treasury.
  ///
  /// # Errors
  /// * Invalid fee data
  pub fn fee(&self) -> Result<UFix64<N4>> {
    self.fee.try_into()
  }

  /// Applies the funding rate to a collateral amount.
  ///
  /// # Errors
  /// * Arithmetic overflow
  pub fn apply_funding_rate(&self, amount: UFix64<N9>) -> Result<UFix64<N9>> {
    let rate = self.rate()?;
    amount
      .mul_div_floor(rate, UFix64::<N9>::one())
      .ok_or(FundingRateApply.into())
  }

  /// Extracts treasury fee from the harvested funding amount.
  ///
  /// # Errors
  /// * Fee extraction arithmetic
  pub fn apply_fee(&self, amount: UFix64<N6>) -> Result<FeeExtract<N6>> {
    let fee = self.fee()?;
    FeeExtract::new(fee, amount)
  }

  /// # Errors
  /// * Rate is zero or exceeds maximum
  /// * Fee is zero or exceeds 100%
  pub fn validate(&self) -> Result<FundingRateConfig> {
    let rate = self.rate()?;
    let fee = self.fee()?;
    if rate > UFix64::zero()
      && rate <= MAX_RATE
      && fee > UFix64::zero()
      && fee <= MAX_FEE
    {
      Ok(*self)
    } else {
      Err(FundingRateValidation.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_config() -> FundingRateConfig {
    let rate = UFix64::<N9>::new(384_620);
    let fee = UFix64::<N4>::new(500);
    FundingRateConfig::new(rate.into(), fee.into())
  }

  #[test]
  fn apply_funding_rate_7_percent_annual() -> Result<()> {
    let config = test_config();
    let collateral = UFix64::<N9>::new(1_000_000_000_000_000);
    let funding = config.apply_funding_rate(collateral)?;
    assert_eq!(funding, UFix64::new(384_620_000_000));
    Ok(())
  }

  #[test]
  fn apply_fee_5_percent() -> Result<()> {
    let config = test_config();
    let amount = UFix64::<N6>::new(384_620_000);
    let extract = config.apply_fee(amount)?;
    assert_eq!(extract.fees_extracted, UFix64::new(19_231_000));
    assert_eq!(extract.amount_remaining, UFix64::new(365_389_000));
    Ok(())
  }

  #[test]
  fn validate_pos() -> Result<()> {
    test_config().validate()?;
    Ok(())
  }

  #[test]
  fn validate_neg_zero_rate() {
    let config = FundingRateConfig::new(
      UFix64::<N9>::zero().into(),
      UFix64::<N4>::new(500).into(),
    );
    assert_eq!(config.validate(), Err(FundingRateValidation.into()));
  }

  #[test]
  fn validate_neg_high_rate() {
    let config = FundingRateConfig::new(
      UFix64::<N9>::new(600_001).into(),
      UFix64::<N4>::new(500).into(),
    );
    assert_eq!(config.validate(), Err(FundingRateValidation.into()));
  }

  #[test]
  fn validate_neg_zero_fee() {
    let config = FundingRateConfig::new(
      UFix64::<N9>::new(384_620).into(),
      UFix64::<N4>::zero().into(),
    );
    assert_eq!(config.validate(), Err(FundingRateValidation.into()));
  }

  #[test]
  fn validate_neg_high_fee() {
    let config = FundingRateConfig::new(
      UFix64::<N9>::new(384_620).into(),
      UFix64::<N4>::new(10_001).into(),
    );
    assert_eq!(config.validate(), Err(FundingRateValidation.into()));
  }
}
