use fix::prelude::*;

const U_BUY_ZONE_2: UFix64<N9> = UFix64::constant(1_750_000_000);
const U_BUY_ZONE_1: UFix64<N9> = UFix64::constant(1_650_000_000);
const U_NEUTRAL: UFix64<N9> = UFix64::constant(1_500_000_000);
const U_SELL_ZONE_1: UFix64<N9> = UFix64::constant(1_350_000_000);
const U_SELL_ZONE_2: UFix64<N9> = UFix64::constant(1_200_000_000);
const U_DEPEG: UFix64<N9> = UFix64::constant(1_000_000_000);

const I_BUY_ZONE_2: IFix64<N9> = IFix64::constant(1_750_000_000);
const I_BUY_ZONE_1: IFix64<N9> = IFix64::constant(1_650_000_000);
const I_NEUTRAL: IFix64<N9> = IFix64::constant(1_500_000_000);
const I_SELL_ZONE_1: IFix64<N9> = IFix64::constant(1_350_000_000);
const I_SELL_ZONE_2: IFix64<N9> = IFix64::constant(1_200_000_000);
const I_DEPEG: IFix64<N9> = IFix64::constant(1_000_000_000);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RebalanceMode {
  BuyZone2,
  BuyZone1,
  Neutral,
  SellZone1,
  SellZone2,
  Depeg,
}

impl RebalanceMode {
  #[must_use]
  pub fn from_cr(cr: UFix64<N9>) -> RebalanceMode {
    if (UFix64::zero()..U_DEPEG).contains(&cr) {
      RebalanceMode::Depeg
    } else if (U_DEPEG..U_SELL_ZONE_2).contains(&cr) {
      RebalanceMode::SellZone2
    } else if (U_SELL_ZONE_2..U_SELL_ZONE_1).contains(&cr) {
      RebalanceMode::SellZone1
    } else if (U_SELL_ZONE_1..U_BUY_ZONE_1).contains(&cr) {
      RebalanceMode::Neutral
    } else if (U_BUY_ZONE_1..U_BUY_ZONE_2).contains(&cr) {
      RebalanceMode::BuyZone1
    } else {
      RebalanceMode::BuyZone2
    }
  }

  #[must_use]
  pub const fn threshold(&self) -> UFix64<N9> {
    match self {
      RebalanceMode::BuyZone2 => U_BUY_ZONE_2,
      RebalanceMode::BuyZone1 => U_BUY_ZONE_1,
      RebalanceMode::Neutral => U_NEUTRAL,
      RebalanceMode::SellZone1 => U_SELL_ZONE_1,
      RebalanceMode::SellZone2 => U_SELL_ZONE_2,
      RebalanceMode::Depeg => U_DEPEG,
    }
  }

  #[must_use]
  pub const fn threshold_signed(&self) -> IFix64<N9> {
    match self {
      RebalanceMode::BuyZone2 => I_BUY_ZONE_2,
      RebalanceMode::BuyZone1 => I_BUY_ZONE_1,
      RebalanceMode::Neutral => I_NEUTRAL,
      RebalanceMode::SellZone1 => I_SELL_ZONE_1,
      RebalanceMode::SellZone2 => I_SELL_ZONE_2,
      RebalanceMode::Depeg => I_DEPEG,
    }
  }

  #[must_use]
  pub fn mint_enabled(cr: UFix64<N9>) -> bool {
    cr >= RebalanceMode::Neutral.threshold()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use RebalanceMode::*;

  #[test]
  fn mode_ordering() {
    assert!(BuyZone2 < BuyZone1);
    assert!(BuyZone1 < Neutral);
    assert!(Neutral < SellZone1);
    assert!(SellZone1 < SellZone2);
    assert!(SellZone2 < Depeg);
  }

  #[test]
  fn from_cr_exact_boundaries() {
    assert_eq!(RebalanceMode::from_cr(U_DEPEG), SellZone2);
    assert_eq!(RebalanceMode::from_cr(U_SELL_ZONE_2), SellZone1);
    assert_eq!(RebalanceMode::from_cr(U_SELL_ZONE_1), Neutral);
    assert_eq!(RebalanceMode::from_cr(U_BUY_ZONE_1), BuyZone1);
    assert_eq!(RebalanceMode::from_cr(U_BUY_ZONE_2), BuyZone2);
  }

  #[test]
  fn from_cr_just_below_boundaries() {
    assert_eq!(RebalanceMode::from_cr(UFix64::new(999_999_999)), Depeg);
    assert_eq!(
      RebalanceMode::from_cr(UFix64::new(1_199_999_999)),
      SellZone2
    );
    assert_eq!(
      RebalanceMode::from_cr(UFix64::new(1_349_999_999)),
      SellZone1
    );
    assert_eq!(RebalanceMode::from_cr(UFix64::new(1_649_999_999)), Neutral);
    assert_eq!(RebalanceMode::from_cr(UFix64::new(1_749_999_999)), BuyZone1);
  }

  #[test]
  fn from_cr_extremes() {
    assert_eq!(RebalanceMode::from_cr(UFix64::zero()), Depeg);
    assert_eq!(RebalanceMode::from_cr(UFix64::new(5_000_000_000)), BuyZone2);
  }

  #[test]
  fn mint_enabled_check() {
    assert!(!RebalanceMode::mint_enabled(UFix64::new(1_499_999_999)));
    assert!(RebalanceMode::mint_enabled(UFix64::new(1_500_000_000)));
    assert!(RebalanceMode::mint_enabled(UFix64::new(2_000_000_000)));
  }
}
