use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::InvalidFees;
use crate::fee_controller::FeeExtract;

#[derive(Copy, Clone, InitSpace, AnchorSerialize, AnchorDeserialize)]
pub struct LstSwapConfig {
  pub fee: UFixValue64,
}

impl LstSwapConfig {
  #[must_use]
  pub fn new(fee: UFixValue64) -> LstSwapConfig {
    LstSwapConfig { fee }
  }

  /// Gets the configured fee rate in basis points.
  pub fn fee(&self) -> Result<UFix64<N4>> {
    self.fee.try_into()
  }

  /// Updates fee rate.
  pub fn update(&mut self, new_fee: UFixValue64) -> Result<()> {
    Self::validate_fee(new_fee)?;
    self.fee = new_fee;
    Ok(())
  }

  /// Applies swap fee to a token amount.
  pub fn apply_swap_fee<Exp>(
    &self,
    amount: UFix64<Exp>,
  ) -> Result<FeeExtract<Exp>> {
    FeeExtract::new(self.fee()?, amount)
  }

  /// Fee must be greater than zero and less than 100%.
  fn validate_fee(fee: UFixValue64) -> Result<()> {
    let bps: UFix64<N4> = fee.try_into()?;
    if bps > UFix64::zero() && bps < UFix64::one() {
      Ok(())
    } else {
      Err(InvalidFees.into())
    }
  }

  /// Validate the current fee configuration.
  pub fn validate(&self) -> Result<()> {
    Self::validate_fee(self.fee)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn apply_fee() -> Result<()> {
    let fee = UFixValue64::new(50, -4);
    let config = LstSwapConfig::new(fee);
    let amount = UFix64::<N9>::new(1_000_000_000);

    let result = config.apply_swap_fee(amount)?;

    assert_eq!(result.fees_extracted, UFix64::new(5_000_000)); // 0.005 tokens
    assert_eq!(result.amount_remaining, UFix64::new(995_000_000)); // 0.995 tokens
    Ok(())
  }

  #[test]
  fn update_fee() -> Result<()> {
    let mut config = LstSwapConfig::new(UFixValue64::new(50));
    config.update(UFixValue64::new(100))?; // 1%
    assert_eq!(config.fee()?, UFix64::new(100));
    Ok(())
  }

  #[test]
  fn reject_zero_fee() {
    let result = LstSwapConfig::new(UFixValue64::new(0)).validate();
    assert!(result.is_err());
  }

  #[test]
  fn reject_100_percent_fee() {
    let result = LstSwapConfig::new(UFixValue64::new(10000)).validate();
    assert!(result.is_err());
  }
}
