use anchor_lang::prelude::{pubkey, Pubkey};
use anchor_spl::mint::USDC as USDC_MINT;
use fix::prelude::{N6, N8, N9};
use fix::typenum::Integer;
use hylo_oracle_types::{BTC_USD_FEED_ID, HYPE_USD_FEED_ID};

use crate::{earn_pool, exchange, pda};

/// Calls `$cb` "shaper" macro once for every exo pair.
///
/// ```ignore
/// macro_rules! shaper {
///   ($(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?) => { ... };
/// }
///
/// with_exo_pairs!(shaper);
/// ```
#[macro_export]
macro_rules! with_exo_pairs {
  ($cb:ident) => {
    $cb! {
      (CBBTC, XBTC, N8),
      (HYPE, XHYPE, N9),
      (ONYC, XONYC, N9),
      (PST, XPST, N6),
      (WETH, XETH, N8),
      (ZEC, XZEC, N8),
    }
  };
}

pub trait TokenMint {
  type Exp: Integer;
  const MINT: Pubkey;
}

pub struct HYUSD;
pub struct SHYUSD;
pub struct XSOL;
pub struct JITOSOL;
pub struct HYLOSOL;
pub struct USDC;
pub struct CBBTC;
pub struct XBTC;
pub struct ZEC;
pub struct XZEC;
pub struct ONYC;
pub struct XONYC;
pub struct HYPE;
pub struct XHYPE;
pub struct PST;
pub struct XPST;
pub struct WETH;
pub struct XETH;

impl TokenMint for HYUSD {
  type Exp = N6;
  const MINT: Pubkey = pda::mint(exchange::ID, exchange::constants::HYUSD);
}

impl TokenMint for SHYUSD {
  type Exp = N6;
  const MINT: Pubkey =
    pda::mint(earn_pool::ID, earn_pool::constants::STAKED_HYUSD);
}

impl TokenMint for XSOL {
  type Exp = N6;
  const MINT: Pubkey = pda::mint(exchange::ID, exchange::constants::XSOL);
}

impl TokenMint for JITOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
}

impl TokenMint for HYLOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("hy1oXYgrBW6PVcJ4s6s2FKavRdwgWTXdfE69AxT7kPT");
}

impl TokenMint for USDC {
  type Exp = N6;
  const MINT: Pubkey = USDC_MINT;
}

impl TokenMint for CBBTC {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("cbbtcf3aa214zXHbiAZQwf4122FBYbraNdFqgw4iMij");
}

impl TokenMint for XBTC {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(CBBTC::MINT);
}

impl TokenMint for ZEC {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("A7bdiYdS5GjqGFtxf17ppRHtDKPkkRqbKtR27dxvQXaS");
}

impl TokenMint for XZEC {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(ZEC::MINT);
}

impl TokenMint for ONYC {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5");
}

impl TokenMint for XONYC {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(ONYC::MINT);
}

impl TokenMint for HYPE {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("98sMhvDwXj1RQi5c5Mndm3vPe9cBqPrbLaufMXFNMh5g");
}

impl TokenMint for XHYPE {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(HYPE::MINT);
}

impl TokenMint for PST {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("59obFNBzyTBGowrkif5uK7ojS58vsuWz3ZCvg6tfZAGw");
}

impl TokenMint for XPST {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(PST::MINT);
}

impl TokenMint for WETH {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs");
}

impl TokenMint for XETH {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(WETH::MINT);
}

pub trait StakePool: TokenMint<Exp = N9> {
  const POOL_STATE: Pubkey;
}

impl StakePool for JITOSOL {
  const POOL_STATE: Pubkey =
    pubkey!("Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb");
}

impl StakePool for HYLOSOL {
  const POOL_STATE: Pubkey =
    pubkey!("hy1oDeVCVRDGkxS26qLVDvRhDpZGfWJ6w9AMvwMegwL");
}

/// Exogenous collateral backing an `ExoPair`.
pub trait Exo: TokenMint {}

macro_rules! impl_exo {
  ($(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?) => {
    $(impl Exo for $exo {})+
  };
}

with_exo_pairs!(impl_exo);

/// Canonical collateral-mint to oracle feed-id binding — the one place the
/// exchange learns which observation prices an EXO collateral. A mint with no
/// entry is not priceable (its feed has not launched), so it cannot register.
const EXO_FEEDS: &[(Pubkey, u16)] = &[
  (CBBTC::MINT, BTC_USD_FEED_ID),
  (HYPE::MINT, HYPE_USD_FEED_ID),
];

/// Oracle feed id that prices `collateral_mint`, or `None` if unbound.
#[must_use]
pub fn feed_id_for(collateral_mint: &Pubkey) -> Option<u16> {
  EXO_FEEDS
    .iter()
    .position(|(mint, _)| mint == collateral_mint)
    .map(|index| EXO_FEEDS[index].1)
}

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use hex_literal::hex;
  use hylo_oracle_types::FEEDS;

  use super::*;

  // Pyth-published BTC/USD Pyth-Core feed id — an independent second copy of
  // `FEEDS[1].pyth_core`. Reordering `FEEDS` or remapping `feed_id_for` trips
  // the binding assertion below.
  const BTC_USD_PYTH_CORE: [u8; 32] =
    hex!("e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43");

  // Pyth-published HYPE/USD Pyth-Core feed id — an independent second copy of
  // `FEEDS[3].pyth_core`.
  const HYPE_USD_PYTH_CORE: [u8; 32] =
    hex!("4279e31cc369bbcc2faf022b382b080e32a8e689ff20fbc530d2a603eb6cd98b");

  #[test]
  fn cbbtc_maps_to_feed_id_one() {
    assert_eq!(feed_id_for(&CBBTC::MINT), Some(1));
  }

  #[test]
  fn hype_maps_to_feed_id_three() {
    assert_eq!(feed_id_for(&HYPE::MINT), Some(3));
  }

  #[test]
  fn cbbtc_feed_binds_to_btc_usd_pyth_core() {
    let feed = FEEDS
      .iter()
      .find(|feed| Some(feed.id) == feed_id_for(&CBBTC::MINT));
    assert!(
      feed.is_some_and(|feed| feed.pyth_core.0 == BTC_USD_PYTH_CORE),
      "cbBTC must bind to the canonical BTC/USD Pyth Core feed id"
    );
  }

  #[test]
  fn hype_feed_binds_to_hype_usd_pyth_core() {
    let feed = FEEDS
      .iter()
      .find(|feed| Some(feed.id) == feed_id_for(&HYPE::MINT));
    assert!(
      feed.is_some_and(|feed| feed.pyth_core.0 == HYPE_USD_PYTH_CORE),
      "HYPE must bind to the canonical HYPE/USD Pyth Core feed id"
    );
  }

  #[test]
  fn exo_feeds_have_unique_mints() {
    let unique = EXO_FEEDS
      .iter()
      .map(|(mint, _)| mint)
      .collect::<HashSet<_>>()
      .len();
    assert_eq!(unique, EXO_FEEDS.len(), "duplicate mint in EXO_FEEDS");
  }

  #[test]
  fn exo_feeds_have_unique_ids() {
    let unique = EXO_FEEDS
      .iter()
      .map(|(_, id)| id)
      .collect::<HashSet<_>>()
      .len();
    assert_eq!(unique, EXO_FEEDS.len(), "duplicate feed id in EXO_FEEDS");
  }

  #[test]
  fn every_exo_feed_id_exists_in_feeds() {
    assert!(
      EXO_FEEDS
        .iter()
        .all(|(_, id)| FEEDS.iter().any(|feed| feed.id == *id)),
      "EXO_FEEDS references a feed id absent from FEEDS"
    );
  }
}
