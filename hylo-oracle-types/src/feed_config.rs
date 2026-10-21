use crate::{OracleSourceConfig, PythLazerChannelId, PythLazerFeedId, FEEDS};

/// Static per-asset feed configuration; the only place feed identity lives.
pub struct FeedConfig {
  pub id: u16,
  pub feeds: &'static [OracleSourceConfig],
}

impl FeedConfig {
  /// Direct index; `feed_ids_match_index` guarantees `id` is the array index.
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
          source,
          OracleSourceConfig::PythLazer {
            feed_id,
            channel_id,
          } if *feed_id == feed && *channel_id == channel
        )
      })
    })
  }
}
