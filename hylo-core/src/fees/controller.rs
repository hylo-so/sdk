use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
#[cfg(feature = "offchain")]
use fix::typenum::Integer;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::error::CoreError::{
  FeeExtraction, InvalidFees, NoValidLevercoinMintFee,
  NoValidLevercoinRedeemFee, NoValidStablecoinMintFee,
  NoValidStablecoinRedeemFee, NoValidSwapFee,
};
use crate::rebalance::mode::RebalanceMode::{
  self, BuyZone1, BuyZone2, Depeg, Neutral, SellZone1, SellZone2,
};
#[cfg(feature = "offchain")]
use crate::util::max_scaled_input;

const MAX_FEE: UFix64<N4> = UFix64::constant(1000);

/// 10 bps (0.1%)
const MIN_STABLECOIN_FEE: UFix64<N4> = UFix64::constant(10);

/// Represents the spread of fees between mint and redeem for protocol tokens.
/// All fees must be in basis points to represent a fractional percentage
/// directly applicable to a token amount e.g. `0.XXXX` or `bips x 10^-4`.
#[derive(
  Clone,
  Copy,
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

  pub fn mint(&self) -> Result<UFix64<N4>, CoreError> {
    Ok(self.mint.try_into()?)
  }

  pub fn redeem(&self) -> Result<UFix64<N4>, CoreError> {
    Ok(self.redeem.try_into()?)
  }

  pub fn validate(&self) -> Result<(), CoreError> {
    (self.mint()? <= MAX_FEE && self.redeem()? <= MAX_FEE)
      .then_some(())
      .ok_or(InvalidFees)
  }
}

/// Fee configuration table reacts to different rebalance modes.
pub trait FeeController: Sized {
  fn mint_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>, CoreError>;
  fn redeem_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>, CoreError>;
  fn validate(self) -> Result<Self, CoreError>;
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
  ) -> Result<FeeExtract<Exp>, CoreError>
  where
    UFix64<FeeExp>: FixExt,
  {
    FeeExtract::split(fee, amount_in).ok_or(FeeExtraction)
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

  /// Inverse of fee extraction under a remainder cap.
  ///
  /// ```txt
  /// x - ceil(x * fee) = floor(x * (1 - fee)) <= cap
  /// ```
  ///
  /// # Errors
  /// * Fee at or above 100%
  #[cfg(feature = "offchain")]
  pub fn max_input<FeeExp>(
    fee: UFix64<FeeExp>,
    cap: UFix64<Exp>,
  ) -> Result<UFix64<Exp>, CoreError>
  where
    UFix64<FeeExp>: FixExt,
    Exp: Integer,
    FeeExp: Integer,
  {
    let kept = UFix64::one()
      .checked_sub(&fee)
      .filter(|kept| *kept != UFix64::zero())
      .ok_or(FeeExtraction)?;
    max_scaled_input(cap, kept, UFix64::one()).ok_or(FeeExtraction)
  }
}

/// Stablecoin mint and redeem fees per rebalance zone.
#[derive(
  Clone,
  Copy,
  PartialEq,
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Serialize,
  Deserialize,
)]
pub struct StablecoinFees {
  pub zone_1: FeePair,
  pub zone_2: FeePair,
}

impl FeeController for StablecoinFees {
  /// Mint fee in the buy zones.
  fn mint_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>, CoreError> {
    match mode {
      BuyZone1 => self.zone_1.mint(),
      BuyZone2 => self.zone_2.mint(),
      Depeg | SellZone2 | SellZone1 | Neutral => Err(NoValidStablecoinMintFee),
    }
  }

  /// Redeem fee in the sell zones, zero in Depeg.
  fn redeem_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>, CoreError> {
    match mode {
      SellZone1 => self.zone_1.redeem(),
      SellZone2 => self.zone_2.redeem(),
      Depeg => Ok(UFix64::zero()),
      Neutral | BuyZone1 | BuyZone2 => Err(NoValidStablecoinRedeemFee),
    }
  }

  fn validate(self) -> Result<StablecoinFees, CoreError> {
    self.zone_1.validate()?;
    self.zone_2.validate()?;
    self.validate_min_fee()?;
    Ok(self)
  }
}

impl StablecoinFees {
  #[must_use]
  pub fn new(zone_1: FeePair, zone_2: FeePair) -> StablecoinFees {
    StablecoinFees { zone_1, zone_2 }
  }

  fn validate_min_fee(&self) -> Result<(), CoreError> {
    [self.zone_1, self.zone_2].iter().try_for_each(|zone| {
      (zone.mint()? >= MIN_STABLECOIN_FEE
        && zone.redeem()? >= MIN_STABLECOIN_FEE)
        .then_some(())
        .ok_or(InvalidFees)
    })
  }

  /// Redeem fee for `mode`. Returns the larger of the two zone fees in
  /// modes with no redeem fee.
  ///
  /// # Errors
  /// * Fee conversion
  #[cfg(feature = "offchain")]
  pub fn saturating_redeem_fee(
    &self,
    mode: RebalanceMode,
  ) -> Result<UFix64<N4>, CoreError> {
    match mode {
      Depeg | SellZone2 | SellZone1 => self.redeem_fee(mode),
      Neutral | BuyZone1 | BuyZone2 => {
        Ok(self.zone_1.redeem()?.max(self.zone_2.redeem()?))
      }
    }
  }
}

#[derive(
  Clone,
  Copy,
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
  fn mint_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>, CoreError> {
    match mode {
      Neutral | BuyZone1 | BuyZone2 => self.normal.mint(),
      SellZone1 => self.sell_zone_1.mint(),
      SellZone2 => self.sell_zone_2.mint(),
      Depeg => Err(NoValidLevercoinMintFee),
    }
  }

  /// Determines fee to charge when redeeming `xSOL`.
  fn redeem_fee(&self, mode: RebalanceMode) -> Result<UFix64<N4>, CoreError> {
    match mode {
      Neutral | BuyZone1 | BuyZone2 => self.normal.redeem(),
      SellZone1 => self.sell_zone_1.redeem(),
      SellZone2 => self.sell_zone_2.redeem(),
      Depeg => Err(NoValidLevercoinRedeemFee),
    }
  }

  /// Run validations
  fn validate(self) -> Result<LevercoinFees, CoreError> {
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
  ) -> Result<UFix64<N4>, CoreError> {
    match mode {
      Neutral | BuyZone1 | BuyZone2 => self.normal.redeem(),
      SellZone1 => self.sell_zone_1.redeem(),
      SellZone2 | Depeg => Err(NoValidSwapFee),
    }
  }

  /// Fees to charge in the stablecoin to levercoin conversion.
  pub fn convert_from_stablecoin_fee(
    &self,
    mode: RebalanceMode,
  ) -> Result<UFix64<N4>, CoreError> {
    match mode {
      Neutral | BuyZone1 | BuyZone2 => self.normal.mint(),
      SellZone1 => self.sell_zone_1.mint(),
      SellZone2 => self.sell_zone_2.mint(),
      Depeg => Err(NoValidSwapFee),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fee_extraction() -> Result<(), CoreError> {
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
    assert_eq!(out.err(), Some(FeeExtraction));
  }

  fn bps(mint: u64, redeem: u64) -> FeePair {
    FeePair::new(
      UFix64::<N4>::new(mint).into(),
      UFix64::<N4>::new(redeem).into(),
    )
  }

  fn stablecoin_fees() -> StablecoinFees {
    StablecoinFees::new(bps(20, 15), bps(10, 10))
  }

  #[test]
  fn stablecoin_fees_accept_min_fee() {
    let at_min = StablecoinFees::new(bps(10, 10), bps(10, 10));
    assert!(stablecoin_fees().validate().is_ok());
    assert!(at_min.validate().is_ok());
  }

  #[test]
  fn stablecoin_fees_reject_rate_below_min_fee() {
    [
      StablecoinFees::new(bps(9, 20), bps(20, 20)),
      StablecoinFees::new(bps(20, 9), bps(20, 20)),
      StablecoinFees::new(bps(20, 20), bps(9, 20)),
      StablecoinFees::new(bps(20, 20), bps(20, 9)),
    ]
    .into_iter()
    .for_each(|fees| {
      assert_eq!(fees.validate().err(), Some(InvalidFees));
    });
  }

  #[test]
  fn stablecoin_mint_fee_only_in_buy_zones() -> Result<(), CoreError> {
    let fees = stablecoin_fees();
    assert_eq!(fees.mint_fee(BuyZone1)?, UFix64::new(20));
    assert_eq!(fees.mint_fee(BuyZone2)?, UFix64::new(10));
    [Depeg, SellZone2, SellZone1, Neutral]
      .iter()
      .for_each(|mode| {
        assert_eq!(fees.mint_fee(*mode).err(), Some(NoValidStablecoinMintFee));
      });
    Ok(())
  }

  #[test]
  fn stablecoin_redeem_fee_only_below_neutral() -> Result<(), CoreError> {
    let fees = stablecoin_fees();
    assert_eq!(fees.redeem_fee(SellZone1)?, UFix64::new(15));
    assert_eq!(fees.redeem_fee(SellZone2)?, UFix64::new(10));
    assert_eq!(fees.redeem_fee(Depeg)?, UFix64::zero());
    [Neutral, BuyZone1, BuyZone2].iter().for_each(|mode| {
      assert_eq!(
        fees.redeem_fee(*mode).err(),
        Some(NoValidStablecoinRedeemFee)
      );
    });
    Ok(())
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
