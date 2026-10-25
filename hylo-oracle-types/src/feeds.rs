use static_assertions::const_assert;

use crate::OracleSourceConfig::PythLazer;
use crate::{FeedConfig, FeedSourceConfig, PythLazerFeedId};

pub const SOL_USD_FEED_ID: u16 = 0;
pub const BTC_USD_FEED_ID: u16 = 1;
pub const USDC_USD_FEED_ID: u16 = 2;
pub const HYPE_USD_FEED_ID: u16 = 3;

pub const FEEDS: &[FeedConfig] = &[
  // id 0 — SOL/USD (LST USD anchor).
  FeedConfig {
    id: SOL_USD_FEED_ID,
    feeds: &[FeedSourceConfig {
      source: PythLazer {
        feed_id: PythLazerFeedId(6),
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

// No cyclic or out-of-range reference feeds.
const_assert!(valid_feed_registry(FEEDS, 0));

#[cfg(test)]
mod tests {
  use static_assertions::const_assert;

  use super::{valid_reference_feeds, FEEDS};
  use crate::{FeedSourceConfig, OracleSourceConfig, PythLazerFeedId};

  const fn source(reference_feed: Option<u8>) -> FeedSourceConfig {
    FeedSourceConfig {
      source: OracleSourceConfig::PythLazer {
        feed_id: PythLazerFeedId(1),
      },
      reference_feed,
    }
  }

  const_assert!(valid_reference_feeds(&[source(Some(1)), source(None)], 0));
  const_assert!(!valid_reference_feeds(&[source(Some(1))], 0));
  const_assert!(!valid_reference_feeds(&[source(Some(0))], 0));
  const_assert!(!valid_reference_feeds(
    &[source(Some(1)), source(Some(0))],
    0
  ));
  const_assert!(!valid_reference_feeds(
    &[source(Some(1)), source(Some(2)), source(Some(0))],
    0
  ));

  #[test]
  fn pyth_lazer_feed_ids_are_unique() {
    let feed_ids = FEEDS
      .iter()
      .flat_map(|feed| feed.feeds)
      .map(|source| {
        let OracleSourceConfig::PythLazer { feed_id } = &source.source;
        feed_id.0
      })
      .collect::<Vec<_>>();
    let unique = feed_ids
      .iter()
      .collect::<std::collections::HashSet<_>>()
      .len();
    assert_eq!(
      feed_ids.len(),
      unique,
      "duplicate pyth lazer feed id in FEEDS"
    );
  }
}
