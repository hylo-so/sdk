use anchor_lang::prelude::Pubkey;
use const_crypto::ed25519;
#[cfg(feature = "offchain")]
use hylo_idl::tokens::{TokenMint, CBBTC, HYPE, ONYC, PST, ZEC};
use pyth_solana_receiver_sdk::price_update::FeedId;
use pyth_solana_receiver_sdk::PYTH_PUSH_ORACLE_ID;

pub struct PythFeed {
  pub feed_id: FeedId,
  pub address: Pubkey,
}

/// Shard 0 holds the sponsored price account for every feed.
const PYTH_SHARD_0: [u8; 2] = 0u16.to_le_bytes();

/// Price update account published for `feed_id` under the push oracle.
#[must_use]
pub const fn pyth_feed_address(feed_id: FeedId) -> Pubkey {
  let (key, _bump) = ed25519::derive_program_address(
    &[&PYTH_SHARD_0, &feed_id],
    PYTH_PUSH_ORACLE_ID.as_array(),
  );
  Pubkey::new_from_array(key)
}

impl PythFeed {
  #[must_use]
  pub const fn new(feed_id: FeedId) -> PythFeed {
    PythFeed {
      feed_id,
      address: pyth_feed_address(feed_id),
    }
  }
}

pub const SOL_USD: PythFeed = PythFeed::new([
  239, 13, 139, 111, 218, 44, 235, 164, 29, 161, 93, 64, 149, 209, 218, 57, 42,
  13, 47, 142, 208, 198, 199, 188, 15, 76, 250, 200, 194, 128, 181, 109,
]);

pub const BTC_USD: PythFeed = PythFeed::new([
  230, 45, 246, 200, 180, 168, 95, 225, 166, 125, 180, 77, 193, 45, 229, 219,
  51, 15, 122, 198, 107, 114, 220, 101, 138, 254, 223, 15, 74, 65, 91, 67,
]);

pub const USDC_USD: PythFeed = PythFeed::new([
  234, 160, 32, 198, 28, 196, 121, 113, 40, 19, 70, 28, 225, 83, 137, 74, 150,
  166, 192, 11, 33, 237, 12, 252, 39, 152, 209, 249, 169, 233, 201, 74,
]);

pub const HYPE_USD: PythFeed = PythFeed::new([
  66, 121, 227, 28, 195, 105, 187, 204, 47, 175, 2, 43, 56, 43, 8, 14, 50, 168,
  230, 137, 255, 32, 251, 197, 48, 210, 166, 3, 235, 108, 217, 139,
]);

pub const ZEC_USD: PythFeed = PythFeed::new([
  190, 155, 89, 209, 120, 240, 214, 169, 122, 180, 195, 67, 191, 242, 170, 105,
  202, 161, 234, 174, 62, 144, 72, 166, 87, 136, 197, 41, 177, 37, 187, 36,
]);

pub const ONYC_USD: PythFeed = PythFeed::new([
  186, 187, 252, 199, 244, 107, 110, 125, 247, 58, 220, 204, 236, 232, 182,
  120, 36, 8, 237, 39, 196, 231, 127, 53, 186, 57, 164, 73, 68, 1, 112, 171,
]);

pub const PST_USDC: PythFeed = PythFeed::new([
  103, 94, 54, 248, 74, 107, 231, 121, 237, 121, 60, 113, 235, 92, 3, 21, 30,
  24, 102, 193, 37, 118, 127, 70, 147, 54, 38, 224, 97, 10, 248, 77,
]);

/// Associates a [`TokenMint`] with the Pyth feed pricing it.
#[cfg(feature = "offchain")]
pub trait PythOracle: TokenMint {
  const FEED: PythFeed;
}

#[cfg(feature = "offchain")]
impl PythOracle for CBBTC {
  const FEED: PythFeed = BTC_USD;
}

#[cfg(feature = "offchain")]
impl PythOracle for HYPE {
  const FEED: PythFeed = HYPE_USD;
}

#[cfg(feature = "offchain")]
impl PythOracle for ZEC {
  const FEED: PythFeed = ZEC_USD;
}

#[cfg(feature = "offchain")]
impl PythOracle for ONYC {
  const FEED: PythFeed = ONYC_USD;
}

#[cfg(feature = "offchain")]
impl PythOracle for PST {
  const FEED: PythFeed = PST_USDC;
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::pubkey;

  use super::*;

  /// Addresses confirmed live onchain when the feed ids were recorded.
  #[test]
  fn feed_addresses_derive_known_accounts() {
    assert_eq!(
      SOL_USD.address,
      pubkey!("7AviUf9nL62mcxNbQGKm4nKDQnPjswo6c5MX4D57HmyE")
    );
    assert_eq!(
      BTC_USD.address,
      pubkey!("APgzQGGdv2qCgBkX6aHVkrGePtBVDDg68GiqaM7rmtf5")
    );
    assert_eq!(
      USDC_USD.address,
      pubkey!("6HAuqASbHEh4w4REJEUUUCginTLfj1kwCh215ZLtMkrT")
    );
    assert_eq!(
      HYPE_USD.address,
      pubkey!("9dAoWJ5ua81c43ntstN4xyg3Uo9Zt93x7BjCxSc27V6V")
    );
    assert_eq!(
      ZEC_USD.address,
      pubkey!("J6PtzWsBtLcuMM7cFS86XVRH4vQfDMcGmBUzSRCMrWMa")
    );
    assert_eq!(
      ONYC_USD.address,
      pubkey!("81yWm9JDhCGgzxWivVkz4tZAEJxYE1VB8tMKKEFbTx71")
    );
    assert_eq!(
      PST_USDC.address,
      pubkey!("2r3AysmX5q9Jo6vznSrkwZxV1wqs9FNniUd8tf2n66oq")
    );
  }
}
