//! Hand-written extensions on `declare_program!`-generated trigger-orders
//! types. Lives in `hylo-idl` because Rust's orphan rule forbids `impl`s
//! on generated types from a foreign crate.

use crate::trigger_orders::types::{ConvertDirection, PairTarget};

impl ConvertDirection {
  /// PDA seed-slice tag for `StableToLever` orders. Must match
  /// `hylo_trigger_orders::state::ConvertDirection::S2L_TAG`.
  pub const S2L_TAG: u8 = 0;
  /// PDA seed-slice tag for `LeverToStable` orders. Must match
  /// `hylo_trigger_orders::state::ConvertDirection::L2S_TAG`.
  pub const L2S_TAG: u8 = 1;

  #[must_use]
  pub const fn tag_byte(&self) -> u8 {
    match self {
      Self::StableToLever => Self::S2L_TAG,
      Self::LeverToStable => Self::L2S_TAG,
    }
  }
}

impl PairTarget {
  /// PDA seed-slice tag for the `Lst` variant. Must match
  /// `hylo_trigger_orders::state::PairTarget::LST_TAG`.
  pub const LST_TAG: u8 = 0;
  /// PDA seed-slice tag for the `Exo` variant. Must match
  /// `hylo_trigger_orders::state::PairTarget::EXO_TAG`.
  pub const EXO_TAG: u8 = 1;

  #[must_use]
  pub const fn tag_byte(&self) -> u8 {
    match self {
      Self::Lst => Self::LST_TAG,
      Self::Exo { .. } => Self::EXO_TAG,
    }
  }
}

#[cfg(test)]
mod tag_tests {
  use super::*;

  #[test]
  fn convert_direction_tags() {
    assert_eq!(ConvertDirection::S2L_TAG, 0);
    assert_eq!(ConvertDirection::L2S_TAG, 1);
    assert_eq!(ConvertDirection::StableToLever.tag_byte(), 0);
    assert_eq!(ConvertDirection::LeverToStable.tag_byte(), 1);
  }

  #[test]
  fn pair_target_tags() {
    use anchor_lang::prelude::Pubkey;
    assert_eq!(PairTarget::LST_TAG, 0);
    assert_eq!(PairTarget::EXO_TAG, 1);
    assert_eq!(PairTarget::Lst.tag_byte(), 0);
    assert_eq!(
      PairTarget::Exo {
        collateral_mint: Pubkey::new_unique()
      }
      .tag_byte(),
      1
    );
  }
}
