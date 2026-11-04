use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};

use super::{OracleObservationConfig, MAX_PRICE_AGGREGATE_OBSERVATIONS};
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

#[derive(
  Debug,
  Clone,
  Copy,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  PartialEq,
  Eq,
)]
/// Pyth Lazer feed and its observation mapping.
pub struct PythLazerConfig {
  pub feed_id: PythLazerFeedId,
  pub observation: OracleObservationConfig,
}

/// Three feeds per price aggregate.
pub const MAX_PYTH_LAZER_FEEDS: usize = 3;

/// Fixed Pyth Lazer feed configuration slots stored for one price aggregate.
#[derive(
  Debug, Default, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace,
)]
pub struct PythLazerFeeds([Option<PythLazerConfig>; MAX_PYTH_LAZER_FEEDS]);

impl PythLazerFeeds {
  /// Creates empty Pyth Lazer feed configuration slots.
  #[must_use]
  pub fn new() -> PythLazerFeeds {
    PythLazerFeeds([None; MAX_PYTH_LAZER_FEEDS])
  }

  /// Returns configured feeds in slot order.
  pub fn configs(&self) -> impl Iterator<Item = &PythLazerConfig> {
    self.0.iter().flatten()
  }

  /// Returns the configuration for a feed ID.
  #[must_use]
  pub fn get_config(
    &self,
    feed_id: PythLazerFeedId,
  ) -> Option<&PythLazerConfig> {
    self.configs().find(|config| config.feed_id == feed_id)
  }

  /// Stores a feed in the first free slot and returns the slot index.
  ///
  /// # Errors
  ///
  /// * The observation or reference index exceeds observation capacity, or the
  ///   reference index matches the observation index.
  /// * The feed ID or observation index is already configured.
  /// * Every slot is occupied.
  pub fn add(&mut self, config: PythLazerConfig) -> Result<usize, CoreError> {
    Self::validate_observation_index(config.observation.index)?;
    Self::validate_reference_index(config)?;
    self.validate_unique_feed_id(config.feed_id)?;
    self.validate_unique_observation_index(config.observation.index)?;
    let slot = self
      .0
      .iter()
      .position(Option::is_none)
      .ok_or(CoreError::OraclePythLazerFeedsFull)?;
    self.0[slot] = Some(config);
    Ok(slot)
  }

  fn validate_observation_index(
    observation_index: u8,
  ) -> Result<(), CoreError> {
    (usize::from(observation_index) < MAX_PRICE_AGGREGATE_OBSERVATIONS)
      .then_some(())
      .ok_or(CoreError::OracleObservationCapacityExceeded)
  }

  fn validate_reference_index(
    config: PythLazerConfig,
  ) -> Result<(), CoreError> {
    config
      .observation
      .reference_index
      .is_none_or(|reference_index| {
        usize::from(reference_index) < MAX_PRICE_AGGREGATE_OBSERVATIONS
          && reference_index != config.observation.index
      })
      .then_some(())
      .ok_or(CoreError::OracleInvalidReferenceFeed)
  }

  fn validate_unique_feed_id(
    &self,
    feed_id: PythLazerFeedId,
  ) -> Result<(), CoreError> {
    self
      .configs()
      .all(|config| config.feed_id != feed_id)
      .then_some(())
      .ok_or(CoreError::OracleDuplicatePythLazerFeedId)
  }

  fn validate_unique_observation_index(
    &self,
    observation_index: u8,
  ) -> Result<(), CoreError> {
    self
      .configs()
      .all(|config| config.observation.index != observation_index)
      .then_some(())
      .ok_or(CoreError::OracleDuplicateObservationIndex)
  }
}

#[cfg(test)]
mod tests {
  use super::{
    OracleObservationConfig, PythLazerConfig, PythLazerFeedId, PythLazerFeeds,
    MAX_PYTH_LAZER_FEEDS,
  };
  use crate::error::CoreError;

  fn config(feed_id: u32, index: u8) -> PythLazerConfig {
    PythLazerConfig {
      feed_id: PythLazerFeedId(feed_id),
      observation: OracleObservationConfig {
        index,
        reference_index: None,
      },
    }
  }

  #[test]
  fn add_fills_slots_in_order() {
    let mut feeds = PythLazerFeeds::new();
    assert_eq!(feeds.add(config(1, 0)), Ok(0));
    assert_eq!(feeds.add(config(2, 1)), Ok(1));
    assert_eq!(feeds.add(config(3, 2)), Ok(2));
    assert_eq!(feeds.configs().count(), MAX_PYTH_LAZER_FEEDS);
    assert_eq!(feeds.get_config(PythLazerFeedId(2)), Some(&config(2, 1)));
  }

  #[test]
  fn add_rejects_when_full() {
    let mut feeds = PythLazerFeeds::new();
    (0..MAX_PYTH_LAZER_FEEDS).for_each(|slot| {
      let slot = u8::try_from(slot).unwrap();
      feeds.add(config(u32::from(slot), slot)).unwrap();
    });
    assert_eq!(
      feeds.add(config(9, 7)),
      Err(CoreError::OraclePythLazerFeedsFull)
    );
  }

  #[test]
  fn add_rejects_duplicate_feed_id() {
    let mut feeds = PythLazerFeeds::new();
    feeds.add(config(1, 0)).unwrap();
    assert_eq!(
      feeds.add(config(1, 1)),
      Err(CoreError::OracleDuplicatePythLazerFeedId)
    );
  }

  #[test]
  fn add_rejects_duplicate_observation_index() {
    let mut feeds = PythLazerFeeds::new();
    feeds.add(config(1, 0)).unwrap();
    assert_eq!(
      feeds.add(config(2, 0)),
      Err(CoreError::OracleDuplicateObservationIndex)
    );
  }

  #[test]
  fn add_rejects_out_of_range_indexes() {
    let mut feeds = PythLazerFeeds::new();
    assert_eq!(
      feeds.add(config(1, 8)),
      Err(CoreError::OracleObservationCapacityExceeded)
    );
    let self_reference = PythLazerConfig {
      feed_id: PythLazerFeedId(1),
      observation: OracleObservationConfig {
        index: 0,
        reference_index: Some(0),
      },
    };
    assert_eq!(
      feeds.add(self_reference),
      Err(CoreError::OracleInvalidReferenceFeed)
    );
    let out_of_range_reference = PythLazerConfig {
      feed_id: PythLazerFeedId(1),
      observation: OracleObservationConfig {
        index: 0,
        reference_index: Some(8),
      },
    };
    assert_eq!(
      feeds.add(out_of_range_reference),
      Err(CoreError::OracleInvalidReferenceFeed)
    );
  }
}
