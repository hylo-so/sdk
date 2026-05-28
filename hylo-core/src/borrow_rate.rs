use anchor_lang::prelude::*;
use fix::prelude::*;
use fix::typenum::Z0;
use serde::{Deserialize, Serialize};

use crate::error::CoreError::{BorrowRateApply, BorrowRateValidation};
use crate::fees::controller::FeeExtract;

/// Per-epoch borrow rate for exogenous collateral without native yield.
#[derive(
  Copy,
  Clone,
  Debug,
  PartialEq,
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Serialize,
  Deserialize,
)]
pub struct BorrowRateConfig {
  pub rate: UFixValue64,
  pub fee: UFixValue64,
}

/// Maximum per-epoch rate (~10% annualized at 182 epochs/year)
const MAX_RATE: UFix64<N9> = UFix64::constant(600_000);

/// Maximum fee exacted against borrow rate
const MAX_FEE: UFix64<N4> = UFix64::constant(10_000);

impl BorrowRateConfig {
  #[must_use]
  pub fn new(rate: UFixValue64, fee: UFixValue64) -> BorrowRateConfig {
    BorrowRateConfig { rate, fee }
  }

  /// Per-epoch borrow rate.
  ///
  /// # Errors
  /// * Invalid rate data
  pub fn rate(&self) -> Result<UFix64<N9>> {
    self.rate.try_into()
  }

  /// Percentage of borrow rate harvest to divert to treasury.
  ///
  /// # Errors
  /// * Invalid fee data
  pub fn fee(&self) -> Result<UFix64<N4>> {
    self.fee.try_into()
  }

  /// Applies the borrow rate to a USD amount.
  /// Multiplies by elapsed epochs to cover missed harvests.
  ///
  /// # Errors
  /// * Arithmetic overflow
  pub fn apply_borrow_rate(
    &self,
    amount: UFix64<N9>,
    elapsed_epochs: UFix64<Z0>,
  ) -> Result<UFix64<N9>> {
    let rate = self.rate()?;
    amount
      .mul_div_floor(rate, UFix64::one())
      .and_then(|base| base.checked_mul(&elapsed_epochs))
      .ok_or(BorrowRateApply.into())
  }

  /// Extracts treasury fee from the harvested borrow amount.
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
  pub fn validate(&self) -> Result<BorrowRateConfig> {
    let rate = self.rate()?;
    let fee = self.fee()?;
    if rate > UFix64::zero()
      && rate <= MAX_RATE
      && fee > UFix64::zero()
      && fee <= MAX_FEE
    {
      Ok(*self)
    } else {
      Err(BorrowRateValidation.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_config() -> BorrowRateConfig {
    let rate = UFix64::<N9>::new(384_620);
    let fee = UFix64::<N4>::new(500);
    BorrowRateConfig::new(rate.into(), fee.into())
  }

  #[test]
  fn apply_borrow_rate_7_percent_annual() -> Result<()> {
    let config = test_config();
    let collateral = UFix64::<N9>::new(1_000_000_000_000_000);
    let borrow = config.apply_borrow_rate(collateral, UFix64::constant(1))?;
    assert_eq!(borrow, UFix64::new(384_620_000_000));
    Ok(())
  }

  #[test]
  fn apply_borrow_rate_multiple_epochs() -> Result<()> {
    let config = test_config();
    let collateral = UFix64::<N9>::new(1_234_567_890_123_456);
    let borrow = config.apply_borrow_rate(collateral, UFix64::constant(5))?;
    assert_eq!(borrow, UFix64::new(2_374_197_509_495));
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
    let config = BorrowRateConfig::new(
      UFix64::<N9>::zero().into(),
      UFix64::<N4>::new(500).into(),
    );
    assert_eq!(config.validate(), Err(BorrowRateValidation.into()));
  }

  #[test]
  fn validate_neg_high_rate() {
    let config = BorrowRateConfig::new(
      UFix64::<N9>::new(600_001).into(),
      UFix64::<N4>::new(500).into(),
    );
    assert_eq!(config.validate(), Err(BorrowRateValidation.into()));
  }

  #[test]
  fn validate_neg_zero_fee() {
    let config = BorrowRateConfig::new(
      UFix64::<N9>::new(384_620).into(),
      UFix64::<N4>::zero().into(),
    );
    assert_eq!(config.validate(), Err(BorrowRateValidation.into()));
  }

  #[test]
  fn validate_neg_high_fee() {
    let config = BorrowRateConfig::new(
      UFix64::<N9>::new(384_620).into(),
      UFix64::<N4>::new(10_001).into(),
    );
    assert_eq!(config.validate(), Err(BorrowRateValidation.into()));
  }
}
