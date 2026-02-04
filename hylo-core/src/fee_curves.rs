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
  20,
  N5,
  (130_000, 5000),
  (132_000, 3099),
  (134_000, 1945),
  (135_000, 1552),
  (137_000, 1007),
  (138_000, 822),
  (140_000, 564),
  (141_000, 476),
  (143_000, 355),
  (145_000, 281),
  (151_000, 198),
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
pub fn mint_fee_curve() -> Result<FixInterp<20, N5>> {
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
  (151_000, 203),
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
