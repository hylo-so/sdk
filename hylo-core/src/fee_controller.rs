use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::{
  FeeExtraction, InvalidFees, NoValidLevercoinMintFee,
  NoValidLevercoinRedeemFee, NoValidStablecoinMintFee, NoValidSwapFee,
};
use crate::stability_mode::StabilityMode::{self, Depeg, Mode1, Mode2, Normal};

/// Represents the spread of fees between mint and redeem for protocol tokens.
/// All fees must be in basis points to represent a fractional percentage
/// directly applicable to a token amount e.g. `0.XXXX` or `bips x 10^-4`.
#[derive(Copy, Clone, InitSpace, AnchorSerialize, AnchorDeserialize)]
pub struct FeePair {
  mint: UFixValue64,
  redeem: UFixValue64,
}

impl FeePair {
  #[must_use]
  pub fn new(mint: UFixValue64, redeem: UFixValue64) -> FeePair {
    FeePair { mint, redeem }
  }

  pub fn mint(&self) -> Result<UFix64<N4>> {
    self.mint.try_into()
  }

  pub fn redeem(&self) -> Result<UFix64<N4>> {
    self.redeem.try_into()
  }

  /// Fees must be less than 100%
  pub fn validate(&self) -> Result<()> {
    let one = UFix64::one();
    if self.mint()? < one && self.redeem()? < one {
      Ok(())
    } else {
      Err(InvalidFees.into())
    }
  }
}

/// Fee configuration table reacts to different stability modes.
pub trait FeeController {
  fn mint_fee(&self, mode: StabilityMode) -> Result<UFix64<N4>>;
  fn redeem_fee(&self, mode: StabilityMode) -> Result<UFix64<N4>>;
  fn validate(&self) -> Result<()>;
}

/// Combines fee multiplication for a token amount with the remaining token
/// amount by subtraction.
pub struct FeeExtract<Exp> {
  pub fees_extracted: UFix64<Exp>,
  pub amount_remaining: UFix64<Exp>,
}

impl<Exp> FeeExtract<Exp> {
  pub fn new(
    fee: UFix64<N4>,
    amount_in: UFix64<Exp>,
  ) -> Result<FeeExtract<Exp>> {
    let fees_extracted = amount_in
      .mul_div_ceil(fee, UFix64::<N4>::one())
      .ok_or(FeeExtraction)?;

    let amount_remaining = amount_in
      .checked_sub(&fees_extracted)
      .ok_or(FeeExtraction)?;

    Ok(FeeExtract {
      fees_extracted,
      amount_remaining,
    })
  }
}

#[derive(Copy, Clone, InitSpace, AnchorSerialize, AnchorDeserialize)]
pub struct StablecoinFees {
  normal: FeePair,
  mode_1: FeePair,
}

impl StablecoinFees {
  #[must_use]
  pub fn new(normal: FeePair, mode_1: FeePair) -> StablecoinFees {
    StablecoinFees { normal, mode_1 }
  }
}

impl FeeController for StablecoinFees {
  /// Determines fee to charge when minting `hyUSD`
  /// Fee increases in mode 1, and minting fails in mode 2.
  fn mint_fee(&self, mode: StabilityMode) -> Result<UFix64<N4>> {
    match mode {
      Normal => self.normal.mint.try_into(),
      Mode1 => self.mode_1.mint.try_into(),
      Mode2 | Depeg => Err(NoValidStablecoinMintFee.into()),
    }
  }

  /// Determines fee to charge when redeeming `hyUSD`.
  fn redeem_fee(&self, mode: StabilityMode) -> Result<UFix64<N4>> {
    match mode {
      Normal => self.normal.redeem.try_into(),
      Mode1 => self.mode_1.redeem.try_into(),
      Mode2 | Depeg => Ok(UFix64::zero()),
    }
  }

  /// Run validations
  fn validate(&self) -> Result<()> {
    self.normal.validate()?;
    self.mode_1.validate()
  }
}

#[derive(Copy, Clone, InitSpace, AnchorDeserialize, AnchorSerialize)]
pub struct LevercoinFees {
  normal: FeePair,
  mode_1: FeePair,
  mode_2: FeePair,
}

impl FeeController for LevercoinFees {
  /// Determines fee to charge when minting `xSOL`.
  /// Fees should become cheaper or zero as protocol goes into stability modes.
  fn mint_fee(&self, mode: StabilityMode) -> Result<UFix64<N4>> {
    match mode {
      Normal => self.normal.mint.try_into(),
      Mode1 => self.mode_1.mint.try_into(),
      Mode2 => self.mode_2.mint.try_into(),
      Depeg => Err(NoValidLevercoinMintFee.into()),
    }
  }

  /// Determines fee to charge when redeeming `xSOL`.
  /// Fees get increasingly more expensive in stability modes.
  fn redeem_fee(&self, mode: StabilityMode) -> Result<UFix64<N4>> {
    match mode {
      Normal => self.normal.redeem.try_into(),
      Mode1 => self.mode_1.redeem.try_into(),
      Mode2 => self.mode_2.redeem.try_into(),
      Depeg => Err(NoValidLevercoinRedeemFee.into()),
    }
  }

  /// Run validations
  fn validate(&self) -> Result<()> {
    self.normal.validate()?;
    self.mode_1.validate()?;
    self.mode_2.validate()
  }
}

impl LevercoinFees {
  #[must_use]
  pub fn new(
    normal: FeePair,
    mode_1: FeePair,
    mode_2: FeePair,
  ) -> LevercoinFees {
    LevercoinFees {
      normal,
      mode_1,
      mode_2,
    }
  }

  /// Fees to charge in the levercoin to stablecoin swap.
  pub fn swap_to_stablecoin_fee(
    &self,
    mode: StabilityMode,
  ) -> Result<UFix64<N4>> {
    match mode {
      Normal => self.normal.redeem.try_into(),
      Mode1 => self.mode_1.redeem.try_into(),
      Mode2 | Depeg => Err(NoValidSwapFee.into()),
    }
  }

  /// Fees to charge in the stablecoin to levercoin swap.
  pub fn swap_from_stablecoin_fee(
    &self,
    mode: StabilityMode,
  ) -> Result<UFix64<N4>> {
    match mode {
      Normal => self.normal.mint(),
      Mode1 => self.mode_1.mint(),
      Mode2 => self.mode_2.mint(),
      Depeg => Err(NoValidSwapFee.into()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fee_extraction() -> Result<()> {
    let fee = UFix64::new(50);
    let amount = UFix64::<N9>::new(69_618_816_010);
    let out = FeeExtract::new(fee, amount)?;
    assert_eq!(out.fees_extracted, UFix64::new(348_094_081));
    assert_eq!(out.amount_remaining, UFix64::new(69_270_721_929));
    Ok(())
  }

  #[test]
  fn fee_extraction_underflow() {
    let fee = UFix64::new(10001);
    let amount = UFix64::<N9>::new(69_618_816_010);
    let out = FeeExtract::new(fee, amount);
    assert_eq!(out.err(), Some(FeeExtraction.into()));
  }
}
