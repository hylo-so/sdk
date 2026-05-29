//! Hand-written extensions on `declare_program!`-generated trigger-orders
//! types. Lives in `hylo-idl` because Rust's orphan rule forbids `impl`s
//! on generated types from a foreign crate.

use crate::trigger_orders::accounts::TriggerOrder;
use crate::trigger_orders::types::{
  ConvertDirection, PairTarget, TriggerDirection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerOutcome {
  /// Price condition satisfied. Execute may proceed (subject to chain
  /// gates surfaced by `TriggerOrder::can_execute` or
  /// `TriggerOrdersClient::simulate_execute_order_*`).
  Met,
  /// Price condition not satisfied at the snapshot.
  NotMet,
  /// Pyth expo on the snapshot doesn't match the order's stored expo.
  /// Order cannot execute until owner cancels and re-creates with the
  /// current expo. Mirrors on-chain `require_eq!(m.exponent,
  /// order.trigger_expo)`.
  ExpoMismatch,
}

impl TriggerOrder {
  /// Pure check against a Pyth price snapshot. No chain reads. Returns
  /// [`TriggerOutcome::ExpoMismatch`] if `pyth_expo` doesn't match the
  /// order's stored `trigger_expo`.
  #[must_use]
  pub fn evaluate_trigger(
    &self,
    pyth_price: i64,
    pyth_expo: i32,
  ) -> TriggerOutcome {
    if pyth_expo != self.trigger_expo {
      return TriggerOutcome::ExpoMismatch;
    }
    let ok = match self.direction {
      TriggerDirection::AtOrBelow => pyth_price <= self.trigger_price,
      TriggerDirection::AtOrAbove => pyth_price >= self.trigger_price,
    };
    if ok {
      TriggerOutcome::Met
    } else {
      TriggerOutcome::NotMet
    }
  }
}

impl TriggerOrder {
  /// Byte offset of the `owner` field within the serialized account
  /// (after the 8-byte Anchor discriminator). Used as the `memcmp` offset
  /// in `getProgramAccounts` filters keyed by owner. If a future migration
  /// inserts a field before `owner`, update this constant.
  pub const OWNER_OFFSET: usize = 8;
}

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

#[cfg(test)]
mod owner_offset_tests {
  use crate::trigger_orders::accounts::TriggerOrder;

  #[test]
  fn owner_offset_is_post_discriminator() {
    // Anchor 8-byte discriminator + owner as first struct field => offset 8.
    assert_eq!(TriggerOrder::OWNER_OFFSET, 8);
  }
}

#[cfg(test)]
mod evaluate_trigger_tests {
  use anchor_lang::prelude::Pubkey;

  use super::*;
  use crate::trigger_orders::types::TriggerDirection;

  fn order(
    trigger_price: i64,
    trigger_expo: i32,
    direction: TriggerDirection,
  ) -> TriggerOrder {
    TriggerOrder {
      owner: Pubkey::default(),
      pair_target: PairTarget::Lst,
      convert_direction: ConvertDirection::StableToLever,
      nonce: 0,
      escrow_amount: 0,
      trigger_price,
      trigger_expo,
      direction,
      created_at: 0,
      bump: 0,
    }
  }

  #[test]
  fn above_met_when_pyth_higher() {
    assert_eq!(
      order(100, -8, TriggerDirection::AtOrAbove).evaluate_trigger(150, -8),
      TriggerOutcome::Met,
    );
  }

  #[test]
  fn above_met_at_equality() {
    assert_eq!(
      order(100, -8, TriggerDirection::AtOrAbove).evaluate_trigger(100, -8),
      TriggerOutcome::Met,
    );
  }

  #[test]
  fn above_not_met_when_pyth_lower() {
    assert_eq!(
      order(100, -8, TriggerDirection::AtOrAbove).evaluate_trigger(50, -8),
      TriggerOutcome::NotMet,
    );
  }

  #[test]
  fn below_met_when_pyth_lower() {
    assert_eq!(
      order(100, -8, TriggerDirection::AtOrBelow).evaluate_trigger(50, -8),
      TriggerOutcome::Met,
    );
  }

  #[test]
  fn below_met_at_equality() {
    assert_eq!(
      order(100, -8, TriggerDirection::AtOrBelow).evaluate_trigger(100, -8),
      TriggerOutcome::Met,
    );
  }

  #[test]
  fn below_not_met_when_pyth_higher() {
    assert_eq!(
      order(100, -8, TriggerDirection::AtOrBelow).evaluate_trigger(150, -8),
      TriggerOutcome::NotMet,
    );
  }

  #[test]
  fn expo_mismatch_short_circuits_regardless_of_price() {
    let o = order(100, -8, TriggerDirection::AtOrAbove);
    assert_eq!(
      o.evaluate_trigger(1_000_000, -6),
      TriggerOutcome::ExpoMismatch
    );
    assert_eq!(o.evaluate_trigger(0, -10), TriggerOutcome::ExpoMismatch);
  }
}
