use anchor_lang::Result;
use fix::prelude::*;

use crate::interp::{FixInterp, Point};

macro_rules! generate_curve {
    ($name:ident, $res:expr, $prec:ty, $(($x:expr, $y:expr)),* $(,)?) => {
      pub const $name: &[Point<$prec>; $res] = &[
        $(
          Point::from_ints($x, $y),
        )*
      ];
    };
}

generate_curve!(
  MINT_FEE_EXP_DECAY,
  10,
  N5,
  (150_000, 200),
  (155_000, 192),
  (160_000, 185),
  (166_000, 177),
  (172_000, 169),
  (187_000, 154),
  (207_000, 139),
  (232_000, 126),
  (263_000, 115),
  (300_000, 108),
);

/// Loads the mint fee curve into an interpolator.
///
/// # Errors
/// * Curve validation
pub fn mint_fee_curve() -> Result<FixInterp<10, N5>> {
  FixInterp::from_points(*MINT_FEE_EXP_DECAY)
}

/// Loads the redeem fee curve into an interpolator.
///
/// # Errors
/// * Curve validation
pub fn redeem_fee_curve() -> Result<FixInterp<20, N5>> {
  FixInterp::from_points(*REDEEM_FEE_LN)
}

generate_curve!(
  REDEEM_FEE_LN,
  20,
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
  (155_000, 212),
  (160_000, 221),
  (166_000, 230),
  (172_000, 238),
  (187_000, 252),
  (207_000, 265),
  (232_000, 278),
  (263_000, 289),
  (300_000, 300),
);
