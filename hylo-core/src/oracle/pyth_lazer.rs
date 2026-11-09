use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};

use super::PriceObservation;
use crate::error::CoreError;

/// Pyth Lazer feed id (Lazer's numeric identifier).
#[derive(
  AnchorSerialize,
  AnchorDeserialize,
  Clone,
  Copy,
  Debug,
  PartialEq,
  Eq,
  InitSpace,
)]
pub struct PythLazerFeedId(pub u32);

/// Pyth Lazer feed and the latest observation posted from it.
#[derive(
  Debug,
  Clone,
  Copy,
  PartialEq,
  Eq,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
)]
pub struct PythLazerSlot {
  pub feed_id: PythLazerFeedId,
  pub observation: PriceObservation,
}

/// Three slots per price aggregate.
pub const PYTH_LAZER_SLOT_COUNT: usize = 3;

/// Fixed Pyth Lazer slots stored for one price aggregate.
#[derive(
  Debug,
  Default,
  Clone,
  Copy,
  PartialEq,
  Eq,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
)]
pub struct PythLazerSlots(pub [Option<PythLazerSlot>; PYTH_LAZER_SLOT_COUNT]);

impl PythLazerSlots {
  /// Returns occupied slots in slot order.
  pub fn occupied(&self) -> impl Iterator<Item = &PythLazerSlot> {
    self.0.iter().flatten()
  }

  /// Returns the slot at an index mutably.
  ///
  /// # Errors
  /// * The index is out of range.
  fn slot_mut(
    &mut self,
    index: u8,
  ) -> Result<&mut Option<PythLazerSlot>, CoreError> {
    self
      .0
      .get_mut(usize::from(index))
      .ok_or(CoreError::OraclePythLazerSlotOutOfRange)
  }

  /// Returns the slot holding a feed ID.
  #[must_use]
  pub fn slot_for_feed(
    &self,
    feed_id: PythLazerFeedId,
  ) -> Option<&PythLazerSlot> {
    self.occupied().find(|slot| slot.feed_id == feed_id)
  }

  /// Returns the slot holding a feed ID mutably.
  pub fn slot_for_feed_mut(
    &mut self,
    feed_id: PythLazerFeedId,
  ) -> Option<&mut PythLazerSlot> {
    self
      .0
      .iter_mut()
      .flatten()
      .find(|slot| slot.feed_id == feed_id)
  }

  /// Stores a feed with a default observation in an empty slot.
  ///
  /// # Errors
  /// * The feed ID has already been configured to a slot.
  /// * The given index is out of range.
  /// * The slot is already occupied.
  pub fn set(
    &mut self,
    index: u8,
    feed_id: PythLazerFeedId,
  ) -> Result<(), CoreError> {
    self
      .slot_for_feed(feed_id)
      .is_none()
      .then_some(())
      .ok_or(CoreError::OracleDuplicatePythLazerFeedId)?;
    let slot = self.slot_mut(index)?;
    slot
      .is_none()
      .then_some(())
      .ok_or(CoreError::OraclePythLazerSlotOccupied)?;
    *slot = Some(PythLazerSlot {
      feed_id,
      observation: PriceObservation::default(),
    });
    Ok(())
  }

  /// Clears an occupied slot and returns its contents.
  ///
  /// # Errors
  /// * The index is out of range.
  /// * The slot is empty.
  pub fn clear(&mut self, index: u8) -> Result<PythLazerSlot, CoreError> {
    self
      .slot_mut(index)?
      .take()
      .ok_or(CoreError::OraclePythLazerSlotEmpty)
  }
}

#[cfg(test)]
mod tests {
  use super::{
    PriceObservation, PythLazerFeedId, PythLazerSlot, PythLazerSlots,
  };
  use crate::error::CoreError;

  fn slot(feed_id: u32, price: u64) -> PythLazerSlot {
    PythLazerSlot {
      feed_id: PythLazerFeedId(feed_id),
      observation: PriceObservation {
        price,
        ..PriceObservation::default()
      },
    }
  }

  fn full() -> PythLazerSlots {
    PythLazerSlots([Some(slot(1, 0)), Some(slot(2, 0)), Some(slot(3, 0))])
  }

  #[test]
  fn set_fills_the_indexed_slot() -> Result<(), CoreError> {
    let mut slots = PythLazerSlots::default();
    slots.set(2, PythLazerFeedId(1))?;
    assert_eq!(slots, PythLazerSlots([None, None, Some(slot(1, 0))]));
    Ok(())
  }

  #[test]
  fn set_rejects_out_of_range_index() {
    let mut slots = PythLazerSlots::default();
    assert_eq!(
      slots.set(3, PythLazerFeedId(1)),
      Err(CoreError::OraclePythLazerSlotOutOfRange)
    );
    assert_eq!(slots, PythLazerSlots::default());
  }

  #[test]
  fn set_rejects_occupied_slot() {
    let mut slots = full();
    assert_eq!(
      slots.set(1, PythLazerFeedId(9)),
      Err(CoreError::OraclePythLazerSlotOccupied)
    );
    assert_eq!(slots, full());
  }

  #[test]
  fn set_rejects_duplicate_feed_id() {
    let mut slots = PythLazerSlots([Some(slot(1, 0)), None, None]);
    assert_eq!(
      slots.set(1, PythLazerFeedId(1)),
      Err(CoreError::OracleDuplicatePythLazerFeedId)
    );
    assert_eq!(slots, PythLazerSlots([Some(slot(1, 0)), None, None]));
  }

  #[test]
  fn clear_empties_the_slot_and_set_refills_it() -> Result<(), CoreError> {
    let mut slots =
      PythLazerSlots([Some(slot(1, 0)), Some(slot(2, 7)), Some(slot(3, 0))]);
    assert_eq!(slots.clear(1), Ok(slot(2, 7)));
    assert_eq!(
      slots,
      PythLazerSlots([Some(slot(1, 0)), None, Some(slot(3, 0))])
    );
    slots.set(1, PythLazerFeedId(2))?;
    assert_eq!(slots, full());
    Ok(())
  }

  #[test]
  fn clear_rejects_empty_or_out_of_range() {
    let mut slots = PythLazerSlots([Some(slot(1, 0)), None, None]);
    assert_eq!(slots.clear(1), Err(CoreError::OraclePythLazerSlotEmpty));
    assert_eq!(
      slots.clear(3),
      Err(CoreError::OraclePythLazerSlotOutOfRange)
    );
    assert_eq!(slots, PythLazerSlots([Some(slot(1, 0)), None, None]));
  }

  #[test]
  fn slot_for_feed_finds_the_slot_past_an_empty_one() {
    let mut slots = PythLazerSlots([Some(slot(1, 0)), None, Some(slot(3, 0))]);
    if let Some(found) = slots.slot_for_feed_mut(PythLazerFeedId(3)) {
      found.observation.price = 7;
    }
    assert_eq!(
      slots,
      PythLazerSlots([Some(slot(1, 0)), None, Some(slot(3, 7))])
    );
    assert_eq!(slots.slot_for_feed(PythLazerFeedId(3)), Some(&slot(3, 7)));
    assert_eq!(slots.slot_for_feed(PythLazerFeedId(2)), None);
  }
}
