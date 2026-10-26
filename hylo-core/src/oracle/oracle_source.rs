use anchor_lang::prelude::*;

use super::PythLazerFeedId;

/// Price source backing an observation.
#[derive(
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  Clone,
  Copy,
  Debug,
  PartialEq,
  Eq,
)]
pub enum OracleSource {
  PythLazer,
}

/// Identifiers for one price source.
pub enum OracleSourceConfig {
  PythLazer { feed_id: PythLazerFeedId },
}
