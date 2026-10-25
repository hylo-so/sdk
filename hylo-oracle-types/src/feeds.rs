use anchor_lang::prelude::{pubkey, Pubkey};
use static_assertions::const_assert;

use crate::OracleSourceConfig::PythLazer;
use crate::{FeedConfig, FeedSourceConfig, PythLazerFeedId};

pub const FEEDS: &[FeedConfig] = &[
  // SOL/USD
  FeedConfig {
    mint: pubkey!("So11111111111111111111111111111111111111112"),
    feeds: &[FeedSourceConfig {
      source: PythLazer {
        feed_id: PythLazerFeedId(6),
      },
      reference_feed: None,
    }],
  },
  // BTC/USD
  FeedConfig {
    mint: pubkey!("cbbtcf3aa214zXHbiAZQwf4122FBYbraNdFqgw4iMij"),
    feeds: &[FeedSourceConfig {
      source: PythLazer {
        feed_id: PythLazerFeedId(1),
      },
      reference_feed: None,
    }],
  },
  // USDC/USD
  FeedConfig {
    mint: pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
    feeds: &[FeedSourceConfig {
      source: PythLazer {
        feed_id: PythLazerFeedId(7),
      },
      reference_feed: None,
    }],
  },
  // HYPE/USD
  FeedConfig {
    mint: pubkey!("98sMhvDwXj1RQi5c5Mndm3vPe9cBqPrbLaufMXFNMh5g"),
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
    mint_absent_before(feeds, index, 0)
      && valid_reference_feeds(feeds[index].feeds, 0)
      && valid_feed_registry(feeds, index + 1)
  }
}

const fn get<T>(items: &[T], index: usize) -> Option<&T> {
  if index < items.len() {
    Some(&items[index])
  } else {
    None
  }
}

const fn mint_absent_before(
  feeds: &[FeedConfig],
  index: usize,
  earlier: usize,
) -> bool {
  if earlier == index {
    get(feeds, index).is_some()
  } else {
    match (get(feeds, earlier), get(feeds, index)) {
      (Some(earlier_feed), Some(index_feed)) => {
        !same_mint(earlier_feed.mint, index_feed.mint)
          && mint_absent_before(feeds, index, earlier + 1)
      }
      _ => false,
    }
  }
}

const fn same_mint(left: Pubkey, right: Pubkey) -> bool {
  same_bytes(&left.to_bytes(), &right.to_bytes(), 0)
}

const fn same_bytes(left: &[u8; 32], right: &[u8; 32], index: usize) -> bool {
  match (get(left, index), get(right, index)) {
    (Some(left_byte), Some(right_byte)) => {
      *left_byte == *right_byte && same_bytes(left, right, index + 1)
    }
    _ => index == left.len(),
  }
}

// No cyclic or out-of-range reference feeds, and no duplicate mints.
const_assert!(valid_feed_registry(FEEDS, 0));

#[cfg(test)]
mod tests {
  use static_assertions::const_assert;

  use super::{mint_absent_before, same_bytes, valid_reference_feeds, FEEDS};
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

  const_assert!(!mint_absent_before(&[], 0, 0));
  const_assert!(!mint_absent_before(FEEDS, 0, 1));
  const_assert!(!same_bytes(&[0; 32], &[0; 32], 33));

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
