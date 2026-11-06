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

  /// Stores a feed in the first free slot and returns the slot index.
  ///
  /// # Errors
  /// * The feed ID is already configured.
  /// * Every slot is occupied.
  pub fn add(&mut self, feed_id: PythLazerFeedId) -> Result<u8, CoreError> {
    self
      .slot_for_feed(feed_id)
      .is_none()
      .then_some(())
      .ok_or(CoreError::OracleDuplicatePythLazerFeedId)?;
    let (index, slot) = (0u8..)
      .zip(self.0.iter_mut())
      .find(|(_, slot)| slot.is_none())
      .ok_or(CoreError::OraclePythLazerSlotsFull)?;
    *slot = Some(PythLazerSlot {
      feed_id,
      observation: PriceObservation::default(),
    });
    Ok(index)
  }

  /// Clears the slot at an index and returns its contents.
  ///
  /// # Errors
  /// * The index is out of range or the slot is empty.
  pub fn remove(&mut self, index: u8) -> Result<PythLazerSlot, CoreError> {
    self
      .0
      .get_mut(usize::from(index))
      .and_then(Option::take)
      .ok_or(CoreError::OraclePythLazerSlotEmpty)
  }
}

#[cfg(test)]
mod tests {
  use super::{PythLazerFeedId, PythLazerSlots, PYTH_LAZER_SLOT_COUNT};
  use crate::error::CoreError;

  fn full() -> Result<PythLazerSlots, CoreError> {
    let mut slots = PythLazerSlots::default();
    slots.add(PythLazerFeedId(1))?;
    slots.add(PythLazerFeedId(2))?;
    slots.add(PythLazerFeedId(3))?;
    Ok(slots)
  }

  #[test]
  fn add_fills_slots_in_order() {
    let mut slots = PythLazerSlots::default();
    assert_eq!(slots.add(PythLazerFeedId(1)), Ok(0));
    assert_eq!(slots.add(PythLazerFeedId(2)), Ok(1));
    assert_eq!(slots.add(PythLazerFeedId(3)), Ok(2));
    assert_eq!(slots.occupied().count(), PYTH_LAZER_SLOT_COUNT);
    assert_eq!(
      slots.slot(1).map(|slot| slot.feed_id),
      Some(PythLazerFeedId(2))
    );
    assert_eq!(
      slots
        .slot_for_feed(PythLazerFeedId(2))
        .map(|slot| slot.feed_id),
      Some(PythLazerFeedId(2))
    );
  }

  #[test]
  fn add_rejects_when_full() -> Result<(), CoreError> {
    let mut slots = full()?;
    assert_eq!(
      slots.add(PythLazerFeedId(9)),
      Err(CoreError::OraclePythLazerSlotsFull)
    );
    Ok(())
  }

  #[test]
  fn add_rejects_duplicate_feed_id() -> Result<(), CoreError> {
    let mut slots = PythLazerSlots::default();
    slots.add(PythLazerFeedId(1))?;
    assert_eq!(
      slots.add(PythLazerFeedId(1)),
      Err(CoreError::OracleDuplicatePythLazerFeedId)
    );
    Ok(())
  }

  #[test]
  fn remove_clears_slot_and_add_refills_it() -> Result<(), CoreError> {
    let mut slots = full()?;
    let removed = slots.remove(1)?;
    assert_eq!(removed.feed_id, PythLazerFeedId(2));
    assert_eq!(slots.slot(1), None);
    assert_eq!(slots.occupied().count(), PYTH_LAZER_SLOT_COUNT - 1);
    assert_eq!(slots.add(PythLazerFeedId(9)), Ok(1));
    Ok(())
  }

  #[test]
  fn remove_rejects_empty_or_out_of_range() {
    let mut slots = PythLazerSlots::default();
    assert_eq!(slots.remove(0), Err(CoreError::OraclePythLazerSlotEmpty));
    assert_eq!(slots.remove(3), Err(CoreError::OraclePythLazerSlotEmpty));
  }

  #[test]
  fn slot_for_feed_mut_writes_the_slot_observation() -> Result<(), CoreError> {
    let mut slots = PythLazerSlots::default();
    slots.add(PythLazerFeedId(1))?;
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
