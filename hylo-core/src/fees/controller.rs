use anchor_lang::prelude::*;
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError::{
  FeeExtraction, InvalidFees, NoValidLevercoinMintFee,
  NoValidLevercoinRedeemFee, NoValidSwapFee,
};
use crate::rebalance::mode::RebalanceMode::{
  self, BuyZone1, BuyZone2, Depeg, Neutral, SellZone1, SellZone2,
};

const MAX_FEE: UFix64<N4> = UFix64::constant(1000);

/// Represents the spread of fees between mint and redeem for protocol tokens.
/// All fees must be in basis points to represent a fractional percentage
/// directly applicable to a token amount e.g. `0.XXXX` or `bips x 10^-4`.
#[derive(
  Copy,
  Clone,
  PartialEq,
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Serialize,
  Deserialize,
)]
pub struct FeePair {
  pub mint: UFixValue64,
  pub redeem: UFixValue64,
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

  pub fn validate(&self) -> Result<()> {
    (self.mint()? <= MAX_FEE && self.redeem()? <= MAX_FEE)
      .then_some(())
      .ok_or(InvalidFees.into())
  }
}

/// Fee configuration table reacts to different rebalance modes.
pub trait FeeController: Sized {
  fn mint_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>>;
  fn redeem_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>>;
  fn validate(self) -> Result<Self>;
}

/// Combines fee multiplication for a token amount with the remaining token
/// amount by subtraction.
pub struct FeeExtract<Exp> {
  pub fees_extracted: UFix64<Exp>,
  pub amount_remaining: UFix64<Exp>,
}

impl<Exp> FeeExtract<Exp> {
  pub fn new<FeeExp>(
    fee: UFix64<FeeExp>,
    amount_in: UFix64<Exp>,
  ) -> Result<FeeExtract<Exp>>
  where
    UFix64<FeeExp>: FixExt,
  {
    FeeExtract::split(fee, amount_in).ok_or(FeeExtraction.into())
  }

  fn split<FeeExp>(
    fee: UFix64<FeeExp>,
    amount_in: UFix64<Exp>,
  ) -> Option<FeeExtract<Exp>>
  where
    UFix64<FeeExp>: FixExt,
  {
    let fees_extracted =
      amount_in.mul_div_ceil(fee, UFix64::<FeeExp>::one())?;
    let amount_remaining = amount_in.checked_sub(&fees_extracted)?;
    Some(FeeExtract {
      fees_extracted,
      amount_remaining,
    })
  }
}

/// **Deprecated** — retained only for `Hylo` account deserialization.
#[derive(
  Copy,
  Clone,
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Serialize,
  Deserialize,
)]
pub struct StablecoinFees {
  pub normal: FeePair,
  pub mode_1: FeePair,
}

impl StablecoinFees {
  #[must_use]
  pub fn new(normal: FeePair, mode_1: FeePair) -> StablecoinFees {
    StablecoinFees { normal, mode_1 }
  }
}

#[derive(
  Copy,
  Clone,
  PartialEq,
  InitSpace,
  AnchorDeserialize,
  AnchorSerialize,
  Serialize,
  Deserialize,
)]
pub struct LevercoinFees {
  pub normal: FeePair,
  pub sell_zone_1: FeePair,
  pub sell_zone_2: FeePair,
}

impl FeeController for LevercoinFees {
  /// Determines minting fee based on
  fn mint_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>> {
    match mode {
      Neutral | BuyZone1 | BuyZone2 => self.normal.mint(),
      SellZone1 => self.sell_zone_1.mint(),
      SellZone2 => self.sell_zone_2.mint(),
      Depeg => Err(NoValidLevercoinMintFee.into()),
    }
  }

  /// Determines fee to charge when redeeming `xSOL`.
  fn redeem_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>> {
    match mode {
      Neutral | BuyZone1 | BuyZone2 => self.normal.redeem(),
      SellZone1 => self.sell_zone_1.redeem(),
      SellZone2 => self.sell_zone_2.redeem(),
      Depeg => Err(NoValidLevercoinRedeemFee.into()),
    }
  }

  /// Run validations
  fn validate(self) -> Result<LevercoinFees> {
    self.normal.validate()?;
    self.sell_zone_1.validate()?;
    self.sell_zone_2.validate()?;
    Ok(self)
  }
}

impl LevercoinFees {
  #[must_use]
  pub fn new(
    normal: FeePair,
    sell_zone_1: FeePair,
    sell_zone_2: FeePair,
  ) -> LevercoinFees {
    LevercoinFees {
      normal,
      sell_zone_1,
      sell_zone_2,
    }
  }

  /// Fees to charge in the levercoin to stablecoin conversion.
  pub fn convert_to_stablecoin_fee(
    &self,
    mode: RebalanceMode,
  ) -> Result<UFix64<N4>> {
    match mode {
      Neutral | BuyZone1 | BuyZone2 => self.normal.redeem.try_into(),
      SellZone1 => self.sell_zone_1.redeem.try_into(),
      SellZone2 | Depeg => Err(NoValidSwapFee.into()),
    }
  }

  /// Fees to charge in the stablecoin to levercoin conversion.
  pub fn convert_from_stablecoin_fee(
    &self,
    mode: RebalanceMode,
  ) -> Result<UFix64<N4>> {
    match mode {
      Neutral | BuyZone1 | BuyZone2 => self.normal.mint(),
      SellZone1 => self.sell_zone_1.mint(),
      SellZone2 => self.sell_zone_2.mint(),
      Depeg => Err(NoValidSwapFee.into()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fee_extraction() -> Result<()> {
    let fee = UFix64::<N4>::new(50);
    let amount = UFix64::<N9>::new(69_618_816_010);
    let out = FeeExtract::new(fee, amount)?;
    assert_eq!(out.fees_extracted, UFix64::new(348_094_081));
    assert_eq!(out.amount_remaining, UFix64::new(69_270_721_929));
    Ok(())
  }

  #[test]
  fn fee_extraction_underflow() {
    let fee = UFix64::<N4>::new(10001);
    let amount = UFix64::<N9>::new(69_618_816_010);
    let out = FeeExtract::new(fee, amount);
    assert_eq!(out.err(), Some(FeeExtraction.into()));
  }
}

#[cfg(kani)]
mod proofs {
  use fix::prelude::*;

  use crate::fees::controller::FeeExtract;
  use crate::kani_generators::{narrow_ufix64, tolerance};

  /// `fees_extracted + amount_remaining == amount_in` (fee in [0, 1.0]).
  #[kani::proof]
  fn fee_extract_conservation() {
    let fee = tolerance();
    let amount_in: UFix64<N6> = narrow_ufix64();
    let extract = FeeExtract::split(fee, amount_in);
    assert!(extract.is_none_or(|e| {
      e.fees_extracted.checked_add(&e.amount_remaining) == Some(amount_in)
    }));
  }
}
