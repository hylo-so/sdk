use crate::OracleSourceConfig::PythLazer;
use crate::{FeedConfig, FeedSourceConfig, PythLazerFeedId, LAZER_CHANNEL};

pub const SOL_USD_FEED_ID: u16 = 0;
pub const BTC_USD_FEED_ID: u16 = 1;
pub const USDC_USD_FEED_ID: u16 = 2;
pub const HYPE_USD_FEED_ID: u16 = 3;

/// Index-keyed feed registry. Pyth Lazer ids match the mio publisher.
pub const FEEDS: &[FeedConfig] = &[
  // id 0 — SOL/USD (LST USD anchor).
  FeedConfig {
    id: SOL_USD_FEED_ID,
    feeds: &[FeedSourceConfig {
      source: PythLazer {
        feed_id: PythLazerFeedId(6),
        channel_id: LAZER_CHANNEL,
      },
      reference_feed: None,
    }],
  },
  // id 1 — BTC/USD (cbBTC collateral).
  FeedConfig {
    id: BTC_USD_FEED_ID,
    feeds: &[FeedSourceConfig {
      source: PythLazer {
        feed_id: PythLazerFeedId(1),
        channel_id: LAZER_CHANNEL,
      },
      reference_feed: None,
    }],
  },
  // id 2 — USDC/USD (USDC pair).
  FeedConfig {
    id: USDC_USD_FEED_ID,
    feeds: &[FeedSourceConfig {
      source: PythLazer {
        feed_id: PythLazerFeedId(7),
        channel_id: LAZER_CHANNEL,
      },
      reference_feed: None,
    }],
  },
  // id 3 — HYPE/USD (HYPE collateral).
  FeedConfig {
    id: HYPE_USD_FEED_ID,
    feeds: &[FeedSourceConfig {
      source: PythLazer {
        feed_id: PythLazerFeedId(110),
        channel_id: LAZER_CHANNEL,
      },
      reference_feed: None,
    }],
  },
];

const fn valid_reference_feeds(
  feeds: &[FeedSourceConfig],
  index: usize,
) -> bool {
  if index == feeds.len() {
    true
  } else {
    let valid = match feeds[index].reference_feed {
      None => true,
      Some(reference) => {
        let reference = reference as usize;
        reference < feeds.len() && feeds[reference].reference_feed.is_none()
      }
    };
    valid && valid_reference_feeds(feeds, index + 1)
  }
}

const fn valid_feed_registry(feeds: &[FeedConfig], index: usize) -> bool {
  if index == feeds.len() {
    true
  } else {
    feeds[index].id as usize == index
      && valid_reference_feeds(feeds[index].feeds, 0)
      && valid_feed_registry(feeds, index + 1)
  }
}

/// Compile-time validation of feed registry: ensures no cyclic or out-of-bound
/// references.
const _: () = assert!(valid_feed_registry(FEEDS, 0), "invalid reference feed");

#[cfg(test)]
mod tests {
  use super::{valid_reference_feeds, FEEDS, LAZER_CHANNEL};
  use crate::{FeedSourceConfig, OracleSourceConfig, PythLazerFeedId};

  const fn source(reference_feed: Option<u8>) -> FeedSourceConfig {
    FeedSourceConfig {
      source: OracleSourceConfig::PythLazer {
        feed_id: PythLazerFeedId(1),
        channel_id: LAZER_CHANNEL,
      },
      reference_feed,
    }
  }

  const _: () = {
    assert!(valid_reference_feeds(&[source(Some(1)), source(None)], 0));
    assert!(!valid_reference_feeds(&[source(Some(1))], 0));
    assert!(!valid_reference_feeds(&[source(Some(0))], 0));
    assert!(!valid_reference_feeds(
      &[source(Some(1)), source(Some(0))],
      0
    ));
    assert!(!valid_reference_feeds(
      &[source(Some(1)), source(Some(2)), source(Some(0))],
      0
    ));
  };

  #[test]
  fn pyth_lazer_feed_channel_pairs_are_unique() {
    let pairs = FEEDS
      .iter()
      .flat_map(|feed| feed.feeds)
      .map(|source| {
        let OracleSourceConfig::PythLazer {
          feed_id,
          channel_id,
        } = &source.source;
        (feed_id.0, channel_id.0)
      })
      .collect::<Vec<_>>();
    let unique = pairs.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(
      pairs.len(),
      unique,
      "duplicate (pyth_lazer, channel) in FEEDS"
    );
  }

  #[test]
  fn lazer_channel_matches_pyth_protocol() {
    assert_eq!(
      LAZER_CHANNEL.0,
      pyth_lazer_protocol::ChannelId::FIXED_RATE_1000.0,
      "LAZER_CHANNEL must track pyth_lazer_protocol FIXED_RATE_1000"
    );
  }
}
