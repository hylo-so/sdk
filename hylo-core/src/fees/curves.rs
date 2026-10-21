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
  (150_000, 500),
  (151_000, 480),
  (152_000, 450),
  (153_000, 440),
  (154_000, 420),
  (155_000, 410),
  (156_000, 400),
  (157_000, 390),
  (158_000, 380),
  (159_000, 370),
  (160_000, 360),
  (161_000, 350),
  (162_000, 350),
  (163_000, 340),
  (164_000, 330),
  (165_000, 330),
  (166_000, 320),
  (167_000, 310),
  (168_000, 310),
  (169_000, 305),
  (170_000, 305),
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
