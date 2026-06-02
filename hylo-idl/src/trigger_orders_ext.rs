//! Hand-written extensions on `declare_program!`-generated trigger-orders
//! types. Lives in `hylo-idl` because Rust's orphan rule forbids `impl`s
//! on generated types from a foreign crate.

use crate::exchange::accounts::{ExoPair, Hylo};
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
  /// `hylo_trigger_orders::state::ConvertDirection::STABLE_TO_LEVER_TAG`.
  pub const STABLE_TO_LEVER_TAG: u8 = 0;
  /// PDA seed-slice tag for `LeverToStable` orders. Must match
  /// `hylo_trigger_orders::state::ConvertDirection::LEVER_TO_STABLE_TAG`.
  pub const LEVER_TO_STABLE_TAG: u8 = 1;

  #[must_use]
  pub const fn tag_byte(&self) -> u8 {
    match self {
      Self::StableToLever => Self::STABLE_TO_LEVER_TAG,
      Self::LeverToStable => Self::LEVER_TO_STABLE_TAG,
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

/// Why an otherwise-triggered order would still fail right now. The
/// variants `StableToLeverDisabled` / `LeverToStableDisabled` are
/// returned ONLY by `TriggerOrdersClient::simulate_execute_order_*` —
/// not by [`TriggerOrder::can_execute`], which doesn't replicate the
/// on-chain CR-gate predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutabilityBlocker {
  /// Pyth snapshot doesn't satisfy the trigger.
  TriggerNotMet,
  /// Pyth expo on the snapshot doesn't match the order's stored expo.
  ExpoMismatch,
  /// `Hylo.protocol_paused` is true.
  ProtocolPaused,
  /// `Hylo.lst_pair_paused` is true (LST orders only).
  LstPairPaused,
  /// `ExoPair.paused` is true (EXO orders only).
  ExoPairPaused,
  /// `pool_drawdown.ledger.supply.bits > 0` on the relevant pair state.
  DrawdownNotRepaid,
  /// `levercoin_mint_enabled()` is false (Depeg). Returned by simulation only.
  StableToLeverDisabled,
  /// `stablecoin_mint_enabled()` is false (CR below threshold, ~1.35).
  /// Returned by simulation only. The asymmetric regime.
  LeverToStableDisabled,
  /// `yield_harvest_cache.epoch < current_epoch` (LST orders).
  YieldHarvestStale,
  /// `borrow_rate_harvest_cache.epoch < current_epoch` (EXO orders).
  BorrowRateHarvestStale,
  /// Caller passed `exo_pair: Some` for an LST order, or `None` for an EXO.
  PairStateMismatch,
}

impl TriggerOrder {
  /// Returns `Ok(())` if the order would execute against the supplied
  /// chain state for the gates this method can evaluate cheaply.
  ///
  /// **Does NOT replicate the on-chain CR-gate.** The asymmetric
  /// `levercoin_mint_enabled` / `stablecoin_mint_enabled` predicates
  /// live on `LstExchangeContext` / `ExoExchangeContext` and require a
  /// full exchange-context load with Pyth, fees, and rebalance configs.
  /// Keepers should call `TriggerOrdersClient::simulate_execute_order_*`
  /// after `can_execute` returns `Ok` to discover CR-gate blockers via
  /// the actual revert. As a result, this method NEVER returns
  /// [`ExecutabilityBlocker::StableToLeverDisabled`] or
  /// [`ExecutabilityBlocker::LeverToStableDisabled`].
  ///
  /// # Errors
  /// Returns the first blocker found, in on-chain check order.
  pub fn can_execute(
    &self,
    hylo: &Hylo,
    exo_pair: Option<&ExoPair>,
    pyth_price: i64,
    pyth_expo: i32,
    current_epoch: u64,
  ) -> Result<(), ExecutabilityBlocker> {
    // 1. Pair-state shape.
    let is_exo = matches!(self.pair_target, PairTarget::Exo { .. });
    if is_exo != exo_pair.is_some() {
      return Err(ExecutabilityBlocker::PairStateMismatch);
    }

    // 2. Trigger.
    match self.evaluate_trigger(pyth_price, pyth_expo) {
      TriggerOutcome::Met => {}
      TriggerOutcome::NotMet => {
        return Err(ExecutabilityBlocker::TriggerNotMet)
      }
      TriggerOutcome::ExpoMismatch => {
        return Err(ExecutabilityBlocker::ExpoMismatch)
      }
    }

    // 3. Pauses.
    if hylo.protocol_paused {
      return Err(ExecutabilityBlocker::ProtocolPaused);
    }
    if let Some(pair) = exo_pair {
      if pair.paused {
        return Err(ExecutabilityBlocker::ExoPairPaused);
      }
    } else if hylo.lst_pair_paused {
      return Err(ExecutabilityBlocker::LstPairPaused);
    }

    // 4. Drawdown repaid?
    let drawdown_outstanding = exo_pair
      .map_or(hylo.pool_drawdown.ledger.supply.bits != 0, |p| {
        p.pool_drawdown.ledger.supply.bits != 0
      });
    if drawdown_outstanding {
      return Err(ExecutabilityBlocker::DrawdownNotRepaid);
    }

    // 5. Harvest staleness.
    if let Some(pair) = exo_pair {
      if pair.borrow_rate_harvest_cache.epoch < current_epoch {
        return Err(ExecutabilityBlocker::BorrowRateHarvestStale);
      }
    } else if hylo.yield_harvest_cache.epoch < current_epoch {
      return Err(ExecutabilityBlocker::YieldHarvestStale);
    }

    // CR-gate intentionally omitted — see docstring.
    Ok(())
  }
}

#[cfg(test)]
mod can_execute_tests {
  use anchor_lang::prelude::Pubkey;

  use super::*;
  use crate::exchange::accounts::Hylo;
  use crate::exchange::types::{
    BorrowRateConfig, HarvestCache, LevercoinFees, PoolDrawdown,
    RebalanceCurveConfig, UFixValue64, VirtualStablecoin,
  };
  use crate::trigger_orders::types::TriggerDirection;

  // `TriggerOrder` intentionally does NOT derive `Default`, so this fixture
  // lists every field explicitly — adding a field forces it to be updated.
  fn lst_order_stable_to_lever_at_above_trigger() -> TriggerOrder {
    TriggerOrder {
      owner: Pubkey::default(),
      pair_target: PairTarget::Lst,
      convert_direction: ConvertDirection::StableToLever,
      nonce: 0,
      escrow_amount: 0,
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
      created_at: 0,
      bump: 0,
    }
  }

  // An `Exo`-variant order at-or-above its trigger. Clones the LST fixture
  // and swaps the pair target so the EXO code paths in `can_execute` are
  // exercised. (`TriggerOrder` is `Copy`, so the clone is a plain reassign.)
  fn exo_order_stable_to_lever_at_above_trigger() -> TriggerOrder {
    let mut order = lst_order_stable_to_lever_at_above_trigger();
    order.pair_target = PairTarget::Exo {
      collateral_mint: Pubkey::new_unique(),
    };
    order
  }

  // NOTE: unlike `Hylo`, `ExoPair` does NOT derive `Default` — its
  // `_reserved: [u8; 100]` field exceeds the 32-element array limit for which
  // `Default` is auto-implemented, so the macro can't derive it. We therefore
  // list every field explicitly, using `Default::default()` for the embedded
  // sub-structs the tests don't exercise (those are all small enough to derive
  // `Default`) and overriding only `paused`, `pool_drawdown`, and
  // `borrow_rate_harvest_cache`.
  fn healthy_exo_pair(current_epoch: u64) -> ExoPair {
    ExoPair {
      collateral_mint: Pubkey::new_unique(),
      levercoin_mint_bump: 0,
      levercoin_auth_bump: 0,
      vault_auth_bump: 0,
      fee_auth_bump: 0,
      oracle: Pubkey::new_unique(),
      oracle_feed_id: [0u8; 32],
      oracle_interval_secs: 30,
      oracle_conf_tolerance: UFixValue64::default(),
      stablecoin_mint_threshold: UFixValue64::default(),
      virtual_stablecoin: VirtualStablecoin::default(),
      borrow_rate_config: BorrowRateConfig::default(),
      borrow_rate_harvest_cache: HarvestCache {
        epoch: current_epoch,
        ..Default::default()
      },
      levercoin_fees: LevercoinFees::default(),
      sell_curve_config: RebalanceCurveConfig::default(),
      buy_curve_config: RebalanceCurveConfig::default(),
      rebalance_deviation_tolerance: UFixValue64::default(),
      paused: false,
      levercoin_market_cap_limit: UFixValue64::default(),
      pool_drawdown: PoolDrawdown {
        ledger: VirtualStablecoin {
          supply: UFixValue64 { bits: 0, exp: 0 },
        },
      },
      _reserved: [0u8; 100],
    }
  }

  // `Hylo` derives `Default` (every embedded type does too), so use
  // `..Default::default()` and override only the fields the test exercises.
  fn healthy_hylo(current_epoch: u64) -> Hylo {
    Hylo {
      oracle_interval_secs: 30,
      yield_harvest_cache: HarvestCache {
        epoch: current_epoch,
        ..Default::default()
      },
      protocol_paused: false,
      lst_pair_paused: false,
      pool_drawdown: PoolDrawdown {
        ledger: VirtualStablecoin {
          supply: UFixValue64 { bits: 0, exp: 0 },
        },
      },
      ..Default::default()
    }
  }

  #[test]
  fn met_and_healthy_returns_ok() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    assert!(order.can_execute(&hylo, None, 150, -8, 978).is_ok());
  }

  #[test]
  fn trigger_not_met_returns_trigger_not_met() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    assert_eq!(
      order.can_execute(&hylo, None, 50, -8, 978),
      Err(ExecutabilityBlocker::TriggerNotMet),
    );
  }

  #[test]
  fn expo_mismatch_returns_expo_mismatch() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    assert_eq!(
      order.can_execute(&hylo, None, 150, -6, 978),
      Err(ExecutabilityBlocker::ExpoMismatch),
    );
  }

  #[test]
  fn protocol_paused_returns_protocol_paused() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let mut hylo = healthy_hylo(978);
    hylo.protocol_paused = true;
    assert_eq!(
      order.can_execute(&hylo, None, 150, -8, 978),
      Err(ExecutabilityBlocker::ProtocolPaused),
    );
  }

  #[test]
  fn lst_pair_paused_returns_lst_pair_paused() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let mut hylo = healthy_hylo(978);
    hylo.lst_pair_paused = true;
    assert_eq!(
      order.can_execute(&hylo, None, 150, -8, 978),
      Err(ExecutabilityBlocker::LstPairPaused),
    );
  }

  #[test]
  fn drawdown_outstanding_returns_drawdown_not_repaid() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let mut hylo = healthy_hylo(978);
    hylo.pool_drawdown.ledger.supply.bits = 1;
    assert_eq!(
      order.can_execute(&hylo, None, 150, -8, 978),
      Err(ExecutabilityBlocker::DrawdownNotRepaid),
    );
  }

  #[test]
  fn yield_harvest_stale_returns_stale() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(977); // one epoch behind
    assert_eq!(
      order.can_execute(&hylo, None, 150, -8, 978),
      Err(ExecutabilityBlocker::YieldHarvestStale),
    );
  }

  #[test]
  fn pair_state_shape_mismatch() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    let mut exo_order = order;
    exo_order.pair_target = PairTarget::Exo {
      collateral_mint: Pubkey::new_unique(),
    };
    assert_eq!(
      exo_order.can_execute(&hylo, None, 150, -8, 978),
      Err(ExecutabilityBlocker::PairStateMismatch),
    );
  }

  #[test]
  fn exo_met_and_healthy_returns_ok() {
    let order = exo_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    let exo_pair = healthy_exo_pair(978);
    assert!(order
      .can_execute(&hylo, Some(&exo_pair), 150, -8, 978)
      .is_ok());
  }

  #[test]
  fn exo_pair_paused_returns_exo_pair_paused() {
    let order = exo_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    let mut exo_pair = healthy_exo_pair(978);
    exo_pair.paused = true;
    assert_eq!(
      order.can_execute(&hylo, Some(&exo_pair), 150, -8, 978),
      Err(ExecutabilityBlocker::ExoPairPaused),
    );
  }

  #[test]
  fn exo_drawdown_outstanding_returns_drawdown_not_repaid() {
    let order = exo_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    let mut exo_pair = healthy_exo_pair(978);
    exo_pair.pool_drawdown.ledger.supply.bits = 1;
    assert_eq!(
      order.can_execute(&hylo, Some(&exo_pair), 150, -8, 978),
      Err(ExecutabilityBlocker::DrawdownNotRepaid),
    );
  }

  #[test]
  fn exo_borrow_rate_harvest_stale_returns_stale() {
    let order = exo_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    let exo_pair = healthy_exo_pair(977); // one epoch behind
    assert_eq!(
      order.can_execute(&hylo, Some(&exo_pair), 150, -8, 978),
      Err(ExecutabilityBlocker::BorrowRateHarvestStale),
    );
  }

  #[test]
  fn lst_order_with_some_exo_pair_returns_pair_state_mismatch() {
    let order = lst_order_stable_to_lever_at_above_trigger();
    let hylo = healthy_hylo(978);
    let exo_pair = healthy_exo_pair(978);
    assert_eq!(
      order.can_execute(&hylo, Some(&exo_pair), 150, -8, 978),
      Err(ExecutabilityBlocker::PairStateMismatch),
    );
  }
}

#[cfg(test)]
mod tag_tests {
  use super::*;

  #[test]
  fn convert_direction_tags() {
    assert_eq!(ConvertDirection::STABLE_TO_LEVER_TAG, 0);
    assert_eq!(ConvertDirection::LEVER_TO_STABLE_TAG, 1);
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
