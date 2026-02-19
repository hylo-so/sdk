use anchor_lang::Result;
use fix::prelude::*;

use crate::error::CoreError::InvalidFees;
use crate::fee_controller::FeeExtract;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AssetSwapConfig {
  pub fee: UFix64<N4>,
}

impl AssetSwapConfig {
  pub fn new(serialized_fee: UFixValue64) -> Result<AssetSwapConfig> {
    let fee = serialized_fee.try_into()?;
    Self::validate_fee(fee)?;
    Ok(AssetSwapConfig { fee })
  }

  /// Applies swap fee to a token amount.
  pub fn apply_fee<Exp>(&self, amount: UFix64<Exp>) -> Result<FeeExtract<Exp>> {
    FeeExtract::new(self.fee, amount)
  }

  /// Fee must be greater than zero and less than 100%.
  pub fn validate_fee(fee: UFix64<N4>) -> Result<()> {
    if fee > UFix64::zero() && fee < UFix64::one() {
      Ok(())
    } else {
      Err(InvalidFees.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn apply_fee() -> Result<()> {
    let config = AssetSwapConfig::new(UFixValue64::new(50, -4))?;
    let amount = UFix64::<N9>::new(1_000_000_000);
    let result = config.apply_fee(amount)?;
    assert_eq!(result.fees_extracted, UFix64::new(5_000_000));
    assert_eq!(result.amount_remaining, UFix64::new(995_000_000));
    Ok(())
  }

  #[test]
  fn reject_out_of_range_fee() {
    let zero = AssetSwapConfig::new(UFixValue64::new(0, -4));
    let one = AssetSwapConfig::new(UFixValue64::new(10000, -4));
    assert_eq!(zero.err(), Some(InvalidFees.into()));
    assert_eq!(one.err(), Some(InvalidFees.into()));
  }

  #[test]
  fn reject_wrong_exp() {
    let result = AssetSwapConfig::new(UFixValue64::new(200, -2));
    assert!(result.is_err());
  }
}
