use anchor_lang::prelude::*;
use hex_literal::hex;

/// On-chain Pyth Lazer channel id the mio publisher signs on
/// (`fixed_rate@1000ms`). Equals
/// `pyth_lazer_protocol::ChannelId::FIXED_RATE_1000`; kept a bare `u8` so
/// consumers don't pull the Lazer protocol crate.
pub const LAZER_CHANNEL: u8 = 4;

/// Price source backing an observation. New sources append here.
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
  PythCore,
  PythLazer,
  ChainlinkDataStreams,
}

/// Pyth Core feed id (Pyth's 32-byte identifier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythCoreFeedId(pub [u8; 32]);

/// Pyth Lazer feed id (Lazer's numeric identifier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythLazerFeedId(pub u32);

/// Chainlink Data Streams feed id (the verified report's 32-byte id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainlinkDataStreamsFeedId(pub [u8; 32]);

/// Registry feed ids; each equals the asset's index in `FEEDS`.
pub const SOL_USD_FEED_ID: u16 = 0;
pub const BTC_USD_FEED_ID: u16 = 1;
pub const USDC_USD_FEED_ID: u16 = 2;

/// Static per-asset feed configuration; the only place feed identity lives.
pub struct FeedConfig {
  pub id: u16,
  pub pyth_core: PythCoreFeedId,
  pub pyth_lazer: PythLazerFeedId,
  pub pyth_lazer_channel: u8,
  pub chainlink_data_streams: ChainlinkDataStreamsFeedId,
}

impl FeedConfig {
  /// Direct index; `feed_ids_match_index` guarantees `id` is the array index.
  #[must_use]
  pub fn by_id(id: u16) -> Option<&'static Self> {
    FEEDS.get(usize::from(id))
  }

  #[must_use]
  pub fn by_pyth_lazer(
    feed: PythLazerFeedId,
    channel: u8,
  ) -> Option<&'static Self> {
    FEEDS
      .iter()
      .find(|cfg| cfg.pyth_lazer == feed && cfg.pyth_lazer_channel == channel)
  }

  #[must_use]
  pub fn by_chainlink_data_streams(
    feed: ChainlinkDataStreamsFeedId,
  ) -> Option<&'static Self> {
    FEEDS.iter().find(|cfg| cfg.chainlink_data_streams == feed)
  }
}

/// Curated feed registry: the single source of truth for feed identity and the
/// `id` ⇄ asset mapping. Pyth Lazer ids and channel match the mio publisher.
pub const FEEDS: &[FeedConfig] = &[
  // id 0 — SOL/USD (LST USD anchor).
  FeedConfig {
    id: SOL_USD_FEED_ID,
    // Pyth-published SOL/USD feed id.
    pyth_core: PythCoreFeedId(hex!(
      "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"
    )),
    pyth_lazer: PythLazerFeedId(6),
    pyth_lazer_channel: LAZER_CHANNEL,
    // Chainlink Data Streams SOL/USD V3 feed id.
    chainlink_data_streams: ChainlinkDataStreamsFeedId(hex!(
      "0003b778d3f6b2ac4991302b89cb313f99a42467d6c9c5f96f57c29c0d2bc24f"
    )),
  },
  // id 1 — BTC/USD (cbBTC collateral).
  FeedConfig {
    id: BTC_USD_FEED_ID,
    // Pyth-published BTC/USD feed id.
    pyth_core: PythCoreFeedId(hex!(
      "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43"
    )),
    pyth_lazer: PythLazerFeedId(1),
    pyth_lazer_channel: LAZER_CHANNEL,
    // Chainlink Data Streams BTC/USD V3 feed id.
    chainlink_data_streams: ChainlinkDataStreamsFeedId(hex!(
      "00039d9e45394f473ab1f050a1b963e6b05351e52d71e507509ada0c95ed75b8"
    )),
  },
  // id 2 — USDC/USD (USDC pair).
  FeedConfig {
    id: USDC_USD_FEED_ID,
    // Pyth-published USDC/USD feed id.
    pyth_core: PythCoreFeedId(hex!(
      "eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a"
    )),
    pyth_lazer: PythLazerFeedId(7),
    pyth_lazer_channel: LAZER_CHANNEL,
    // Chainlink Data Streams USDC/USD V3 feed id.
    chainlink_data_streams: ChainlinkDataStreamsFeedId(hex!(
      "00038f83323b6b08116d1614cf33a9bd71ab5e0abf0c9f1b783a74a43e7bd992"
    )),
  },
];

#[cfg(test)]
mod tests {
  use super::{FEEDS, LAZER_CHANNEL};

  #[test]
  fn feed_ids_are_unique() {
    let count = FEEDS.len();
    let unique = FEEDS
      .iter()
      .map(|feed| feed.id)
      .collect::<std::collections::HashSet<_>>()
      .len();
    assert_eq!(count, unique, "duplicate feed id in FEEDS");
  }

  #[test]
  fn pyth_core_feed_ids_are_unique() {
    let count = FEEDS.len();
    let unique = FEEDS
      .iter()
      .map(|feed| feed.pyth_core.0)
      .collect::<std::collections::HashSet<_>>()
      .len();
    assert_eq!(count, unique, "duplicate Pyth Core feed id in FEEDS");
  }

  #[test]
  fn feed_ids_match_index() {
    FEEDS.iter().enumerate().for_each(|(index, feed)| {
      assert_eq!(
        usize::from(feed.id),
        index,
        "FEEDS[{index}].id must equal its index (by_id indexes directly)"
      );
    });
  }

  #[test]
  fn pyth_lazer_feed_channel_pairs_are_unique() {
    let count = FEEDS.len();
    let unique = FEEDS
      .iter()
      .map(|feed| (feed.pyth_lazer.0, feed.pyth_lazer_channel))
      .collect::<std::collections::HashSet<_>>()
      .len();
    assert_eq!(count, unique, "duplicate (pyth_lazer, channel) in FEEDS");
  }

  #[test]
  fn chainlink_data_streams_feed_ids_are_unique_and_bound() {
    let count = FEEDS.len();
    let unique = FEEDS
      .iter()
      .map(|feed| feed.chainlink_data_streams.0)
      .collect::<std::collections::HashSet<_>>()
      .len();
    assert_eq!(count, unique, "duplicate Chainlink Data Streams feed id");
  }

  #[test]
  fn lazer_channel_matches_pyth_protocol() {
    assert_eq!(
      LAZER_CHANNEL,
      pyth_lazer_protocol::ChannelId::FIXED_RATE_1000.0,
      "LAZER_CHANNEL must track pyth_lazer_protocol FIXED_RATE_1000"
    );
  }
}
