use fix::prelude::*;

use super::interp::{FixInterp, Point};
use crate::error::CoreError;

macro_rules! generate_curve {
    ($name:ident, $res:expr, $prec:ty, $(($x:expr, $y:expr)),* $(,)?) => {
      pub const $name: &[Point<$prec>; $res] = &[
        $(
          Point::from_ints($x, $y),
        )*
      ];
    };
}

/// Loads the mint fee curve into an interpolator.
///
/// # Errors
/// * Curve validation
pub fn mint_fee_curve() -> Result<FixInterp<21, N5>, CoreError> {
  FixInterp::from_points(*MINT_FEE_INV)
}

/// Loads the redeem fee curve into an interpolator.
///
/// # Errors
/// * Curve validation
pub fn redeem_fee_curve() -> Result<FixInterp<11, N5>, CoreError> {
  FixInterp::from_points(*REDEEM_FEE_LN)
}

/// Highest collateral ratio quoting a stablecoin redeem fee.
pub const REDEEM_MAX_CR: IFix64<N5> =
  FixInterp::from_points_unchecked(*REDEEM_FEE_LN).x_max();

generate_curve!(
  MINT_FEE_INV,
  21,
  N5,
  (150_000, 800),
  (151_000, 780),
  (152_000, 750),
  (153_000, 740),
  (154_000, 720),
  (155_000, 710),
  (156_000, 700),
  (157_000, 690),
  (158_000, 680),
  (159_000, 670),
  (160_000, 660),
  (161_000, 650),
  (162_000, 650),
  (163_000, 640),
  (164_000, 630),
  (165_000, 630),
  (166_000, 620),
  (167_000, 610),
  (168_000, 610),
  (169_000, 605),
  (170_000, 605),
);

generate_curve!(
  REDEEM_FEE_LN,
  11,
  N5,
  (130_000, 0),
  (132_000, 45),
  (134_000, 77),
  (135_000, 91),
  (137_000, 113),
  (138_000, 123),
  (140_000, 140),
  (141_000, 148),
  (143_000, 162),
  (145_000, 174),
  (150_000, 200),
);
