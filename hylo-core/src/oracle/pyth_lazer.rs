use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};

use super::OracleObservationConfig;
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

  /// Returns the configuration for a feed ID.
  #[must_use]
  pub fn get_config(
    &self,
    feed_id: PythLazerFeedId,
  ) -> Option<&PythLazerConfig> {
    self
      .0
      .iter()
      .flatten()
      .find(|config| config.feed_id == feed_id)
  }

  /// Validates configured feeds and references to a replaced feed.
  ///
  /// # Errors
  ///
  /// * A configured observation or reference index exceeds observation
  ///   capacity, or a reference index matches its own observation index.
  /// * A feed ID or observation index appears in multiple configurations.
  /// * A configuration still references the replaced observation index.
  pub fn validate(
    &self,
    replaced_config: Option<PythLazerConfig>,
  ) -> Result<(), CoreError> {
    self
      .0
      .iter()
      .enumerate()
      .filter_map(|(index, config)| config.map(|config| (index, config)))
      .try_for_each(|(index, config)| {
        Self::validate_observation_index(config.observation.index)?;
        Self::validate_reference_index(config)?;
        self.validate_unique_feed_id(index, config.feed_id)?;
        self.validate_unique_observation_index(index, config.observation.index)
      })?;

    replaced_config
      .map(|config| self.validate_unused_reference(config.observation.index))
      .transpose()
      .map(|_| ())
  }

  fn validate_observation_index(
    observation_index: u8,
  ) -> Result<(), CoreError> {
    (usize::from(observation_index) < super::MAX_PRICE_AGGREGATE_OBSERVATIONS)
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
        usize::from(reference_index) < super::MAX_PRICE_AGGREGATE_OBSERVATIONS
          && reference_index != config.observation.index
      })
      .then_some(())
      .ok_or(CoreError::OracleInvalidReferenceFeed)
  }

  fn validate_unique_feed_id(
    &self,
    config_index: usize,
    feed_id: PythLazerFeedId,
  ) -> Result<(), CoreError> {
    self
      .0
      .iter()
      .enumerate()
      .filter(|(index, _)| *index != config_index)
      .filter_map(|(_, config)| *config)
      .all(|config| config.feed_id != feed_id)
      .then_some(())
      .ok_or(CoreError::OracleDuplicatePythLazerFeedId)
  }

  fn validate_unique_observation_index(
    &self,
    config_index: usize,
    observation_index: u8,
  ) -> Result<(), CoreError> {
    self
      .0
      .iter()
      .enumerate()
      .filter(|(index, _)| *index != config_index)
      .filter_map(|(_, config)| *config)
      .all(|config| config.observation.index != observation_index)
      .then_some(())
      .ok_or(CoreError::OracleDuplicateObservationIndex)
  }

  fn validate_unused_reference(
    &self,
    observation_index: u8,
  ) -> Result<(), CoreError> {
    self
      .0
      .iter()
      .flatten()
      .all(|config| {
        config.observation.reference_index != Some(observation_index)
      })
      .then_some(())
      .ok_or(CoreError::OracleInvalidReferenceFeed)
  }
}

impl Deref for PythLazerFeeds {
  type Target = [Option<PythLazerConfig>; MAX_PYTH_LAZER_FEEDS];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for PythLazerFeeds {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}
