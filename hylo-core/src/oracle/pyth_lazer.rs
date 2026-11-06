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

  /// Returns the observations of occupied slots in slot order.
  pub fn observations(&self) -> impl Iterator<Item = PriceObservation> + '_ {
    self.occupied().map(|slot| slot.observation)
  }

  /// Returns the slot at an index.
  #[must_use]
  pub fn slot(&self, index: u8) -> Option<&PythLazerSlot> {
    self.0.get(usize::from(index)).and_then(Option::as_ref)
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
  /// * The index is out of range.
  /// * The slot is occupied.
  /// * The feed ID is already configured.
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
    let slot = self
      .0
      .get_mut(usize::from(index))
      .ok_or(CoreError::OraclePythLazerSlotOutOfRange)?;
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
      .0
      .get_mut(usize::from(index))
      .ok_or(CoreError::OraclePythLazerSlotOutOfRange)?
      .take()
      .ok_or(CoreError::OraclePythLazerSlotEmpty)
  }
}

#[cfg(test)]
mod tests {
  use super::{PythLazerFeedId, PythLazerSlots, PYTH_LAZER_SLOT_COUNT};
  use crate::error::CoreError;

  fn full() -> Result<PythLazerSlots, CoreError> {
    let mut slots = PythLazerSlots::default();
    slots.set(0, PythLazerFeedId(1))?;
    slots.set(1, PythLazerFeedId(2))?;
    slots.set(2, PythLazerFeedId(3))?;
    Ok(slots)
  }

  #[test]
  fn set_fills_the_indexed_slot() -> Result<(), CoreError> {
    let mut slots = PythLazerSlots::default();
    slots.set(2, PythLazerFeedId(1))?;
    assert_eq!(slots.slot(0), None);
    assert_eq!(
      slots.slot(2).map(|slot| slot.feed_id),
      Some(PythLazerFeedId(1))
    );
    assert_eq!(slots.occupied().count(), 1);
    assert_eq!(
      slots
        .slot_for_feed(PythLazerFeedId(1))
        .map(|slot| slot.feed_id),
      Some(PythLazerFeedId(1))
    );
    Ok(())
  }

  #[test]
  fn set_rejects_out_of_range_index() {
    let mut slots = PythLazerSlots::default();
    assert_eq!(
      slots.set(3, PythLazerFeedId(1)),
      Err(CoreError::OraclePythLazerSlotOutOfRange)
    );
  }

  #[test]
  fn set_rejects_occupied_slot() -> Result<(), CoreError> {
    let mut slots = full()?;
    assert_eq!(
      slots.set(1, PythLazerFeedId(9)),
      Err(CoreError::OraclePythLazerSlotOccupied)
    );
    Ok(())
  }

  #[test]
  fn set_rejects_duplicate_feed_id() -> Result<(), CoreError> {
    let mut slots = PythLazerSlots::default();
    slots.set(0, PythLazerFeedId(1))?;
    assert_eq!(
      slots.set(1, PythLazerFeedId(1)),
      Err(CoreError::OracleDuplicatePythLazerFeedId)
    );
    Ok(())
  }

  #[test]
  fn clear_empties_the_slot_and_set_refills_it() -> Result<(), CoreError> {
    let mut slots = full()?;
    let cleared = slots.clear(1)?;
    assert_eq!(cleared.feed_id, PythLazerFeedId(2));
    assert_eq!(slots.slot(1), None);
    assert_eq!(slots.occupied().count(), PYTH_LAZER_SLOT_COUNT - 1);
    slots.set(1, PythLazerFeedId(9))?;
    assert_eq!(
      slots.slot(1).map(|slot| slot.feed_id),
      Some(PythLazerFeedId(9))
    );
    Ok(())
  }

  #[test]
  fn clear_rejects_empty_or_out_of_range() {
    let mut slots = PythLazerSlots::default();
    assert_eq!(slots.clear(0), Err(CoreError::OraclePythLazerSlotEmpty));
    assert_eq!(
      slots.clear(3),
      Err(CoreError::OraclePythLazerSlotOutOfRange)
    );
  }

  #[test]
  fn slot_for_feed_mut_writes_the_slot_observation() -> Result<(), CoreError> {
    let mut slots = PythLazerSlots::default();
    slots.set(0, PythLazerFeedId(1))?;
    let slot = slots
      .slot_for_feed_mut(PythLazerFeedId(1))
      .ok_or(CoreError::OraclePythLazerSlotEmpty)?;
    slot.observation.price = 7;
    assert_eq!(
      slots
        .observations()
        .map(|observation| observation.price)
        .next(),
      Some(7)
    );
    assert!(slots.slot_for_feed_mut(PythLazerFeedId(2)).is_none());
    Ok(())
  }
}
