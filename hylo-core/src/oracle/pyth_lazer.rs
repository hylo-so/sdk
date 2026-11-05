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

/// Three feeds per price aggregate.
pub const MAX_PYTH_LAZER_FEEDS: usize = 3;

/// Fixed Pyth Lazer feed slots stored for one price aggregate.
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
pub struct PythLazerFeeds(
  pub(crate) [Option<PythLazerSlot>; MAX_PYTH_LAZER_FEEDS],
);

impl PythLazerFeeds {
  /// Creates empty Pyth Lazer feed slots.
  #[must_use]
  pub fn new() -> PythLazerFeeds {
    PythLazerFeeds([None; MAX_PYTH_LAZER_FEEDS])
  }

  /// Returns occupied slots in slot order.
  pub fn slots(&self) -> impl Iterator<Item = &PythLazerSlot> {
    self.0.iter().flatten()
  }

  /// Returns the observations of occupied slots in slot order.
  pub fn observations(&self) -> impl Iterator<Item = PriceObservation> + '_ {
    self.slots().map(|slot| slot.observation)
  }

  /// Returns the slot for a feed ID.
  #[must_use]
  pub fn get(&self, feed_id: PythLazerFeedId) -> Option<&PythLazerSlot> {
    self.slots().find(|slot| slot.feed_id == feed_id)
  }

  /// Returns the slot for a feed ID mutably.
  pub fn get_mut(
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
  ///
  /// * The feed ID is already configured.
  /// * Every slot is occupied.
  pub fn add(&mut self, feed_id: PythLazerFeedId) -> Result<usize, CoreError> {
    self
      .get(feed_id)
      .is_none()
      .then_some(())
      .ok_or(CoreError::OracleDuplicatePythLazerFeedId)?;
    let index = self
      .0
      .iter()
      .position(Option::is_none)
      .ok_or(CoreError::OraclePythLazerFeedsFull)?;
    self.0[index] = Some(PythLazerSlot {
      feed_id,
      observation: PriceObservation::default(),
    });
    Ok(index)
  }
}

#[cfg(test)]
mod tests {
  use super::{PythLazerFeedId, PythLazerFeeds, MAX_PYTH_LAZER_FEEDS};
  use crate::error::CoreError;

  fn full() -> Result<PythLazerFeeds, CoreError> {
    let mut feeds = PythLazerFeeds::new();
    feeds.add(PythLazerFeedId(1))?;
    feeds.add(PythLazerFeedId(2))?;
    feeds.add(PythLazerFeedId(3))?;
    Ok(feeds)
  }

  #[test]
  fn add_fills_slots_in_order() {
    let mut feeds = PythLazerFeeds::new();
    assert_eq!(feeds.add(PythLazerFeedId(1)), Ok(0));
    assert_eq!(feeds.add(PythLazerFeedId(2)), Ok(1));
    assert_eq!(feeds.add(PythLazerFeedId(3)), Ok(2));
    assert_eq!(feeds.slots().count(), MAX_PYTH_LAZER_FEEDS);
    assert_eq!(
      feeds.get(PythLazerFeedId(2)).map(|slot| slot.feed_id),
      Some(PythLazerFeedId(2))
    );
  }

  #[test]
  fn add_rejects_when_full() -> Result<(), CoreError> {
    let mut feeds = full()?;
    assert_eq!(
      feeds.add(PythLazerFeedId(9)),
      Err(CoreError::OraclePythLazerFeedsFull)
    );
    Ok(())
  }

  #[test]
  fn add_rejects_duplicate_feed_id() -> Result<(), CoreError> {
    let mut feeds = PythLazerFeeds::new();
    feeds.add(PythLazerFeedId(1))?;
    assert_eq!(
      feeds.add(PythLazerFeedId(1)),
      Err(CoreError::OracleDuplicatePythLazerFeedId)
    );
    Ok(())
  }

  #[test]
  fn get_mut_writes_the_slot_observation() -> Result<(), CoreError> {
    let mut feeds = PythLazerFeeds::new();
    feeds.add(PythLazerFeedId(1))?;
    let slot = feeds
      .get_mut(PythLazerFeedId(1))
      .ok_or(CoreError::OracleNoFreshPriceObservations)?;
    slot.observation.price = 7;
    assert_eq!(
      feeds
        .observations()
        .map(|observation| observation.price)
        .next(),
      Some(7)
    );
    assert!(feeds.get_mut(PythLazerFeedId(2)).is_none());
    Ok(())
  }
}
