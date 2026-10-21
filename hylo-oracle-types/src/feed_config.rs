use crate::{OracleSourceConfig, PythLazerChannelId, PythLazerFeedId, FEEDS};

/// Static configuration for one asset feed.
pub struct FeedConfig {
  pub id: u16,
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
  /// Returns the feed at `id`. `id` equals that feed's index in `FEEDS`.
  #[must_use]
  pub fn by_id(id: u16) -> Option<&'static Self> {
    FEEDS.get(usize::from(id))
  }

  /// Selects the registry entry whose Pyth Lazer source matches both ids.
  #[must_use]
  pub fn by_pyth_lazer(
    feed: PythLazerFeedId,
    channel: PythLazerChannelId,
  ) -> Option<&'static Self> {
    FEEDS.iter().find(|cfg| {
      cfg.feeds.iter().any(|source| {
        matches!(
          &source.source,
          OracleSourceConfig::PythLazer {
            feed_id,
            channel_id,
          } if *feed_id == feed && *channel_id == channel
        )
      })
    })
  }
}
