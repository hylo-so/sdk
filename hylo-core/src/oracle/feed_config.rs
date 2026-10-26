use anchor_lang::prelude::Pubkey;

use super::{OracleSourceConfig, PythLazerFeedId, FEEDS};

/// Static configuration for one asset feed.
pub struct FeedConfig {
  pub mint: Pubkey,
  pub feeds: &'static [FeedSourceConfig],
}

/// One oracle source in a feed, with an optional reference for non-regular
/// market sessions. `reference_feed` is an in-range index into that feed's
/// source list and points at a source with no reference.
pub struct FeedSourceConfig {
  pub source: OracleSourceConfig,
  pub reference_feed: Option<u8>,
}

impl FeedConfig {
  /// Returns the registry entry for `mint`.
  #[must_use]
  pub fn by_mint(mint: Pubkey) -> Option<&'static Self> {
    FEEDS.iter().find(|cfg| cfg.mint == mint)
  }

  /// Returns the registry entry whose Pyth Lazer feed id equals `feed`.
  #[must_use]
  pub fn by_pyth_lazer(feed: PythLazerFeedId) -> Option<&'static Self> {
    FEEDS.iter().find(|cfg| {
      cfg.feeds.iter().any(|source| {
        matches!(
          &source.source,
          OracleSourceConfig::PythLazer { feed_id } if *feed_id == feed
        )
      })
    })
  }
}
