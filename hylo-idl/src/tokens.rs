use anchor_lang::prelude::{pubkey, Pubkey};
use anchor_spl::mint::USDC as USDC_MINT;
use fix::prelude::{N6, N8, N9};
use fix::typenum::Integer;
use hylo_oracle_types::BTC_USD_FEED_ID;

use crate::{earn_pool, exchange, pda};

pub trait TokenMint {
  type Exp: Integer;
  const MINT: Pubkey;
}

pub trait StakePool: TokenMint<Exp = N9> {
  const POOL_STATE: Pubkey;
}

pub struct HYUSD;

impl TokenMint for HYUSD {
  type Exp = N6;
  const MINT: Pubkey = pda::mint(exchange::ID, exchange::constants::HYUSD);
}

pub struct SHYUSD;

impl TokenMint for SHYUSD {
  type Exp = N6;
  const MINT: Pubkey =
    pda::mint(earn_pool::ID, earn_pool::constants::STAKED_HYUSD);
}

pub struct XSOL;

impl TokenMint for XSOL {
  type Exp = N6;
  const MINT: Pubkey = pda::mint(exchange::ID, exchange::constants::XSOL);
}

pub struct JITOSOL;

impl TokenMint for JITOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
}

impl StakePool for JITOSOL {
  const POOL_STATE: Pubkey =
    pubkey!("Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb");
}

pub struct HYLOSOL;

impl TokenMint for HYLOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("hy1oXYgrBW6PVcJ4s6s2FKavRdwgWTXdfE69AxT7kPT");
}

impl StakePool for HYLOSOL {
  const POOL_STATE: Pubkey =
    pubkey!("hy1oDeVCVRDGkxS26qLVDvRhDpZGfWJ6w9AMvwMegwL");
}

pub struct USDC;

impl TokenMint for USDC {
  type Exp = N6;
  const MINT: Pubkey = USDC_MINT;
}

pub struct CBBTC;

impl TokenMint for CBBTC {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("cbbtcf3aa214zXHbiAZQwf4122FBYbraNdFqgw4iMij");
}

pub struct XBTC;

impl TokenMint for XBTC {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(CBBTC::MINT);
}

pub struct SPYX;

impl TokenMint for SPYX {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("XsoCS1TfEyfFhfvj8EtZ528L3CaKBDBRqRapnBbDF2W");
}

pub struct ZEC;

impl TokenMint for ZEC {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("A7bdiYdS5GjqGFtxf17ppRHtDKPkkRqbKtR27dxvQXaS");
}

pub struct XAUT0;

impl TokenMint for XAUT0 {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("AymATz4TCL9sWNEEV9Kvyz45CHVhDZ6kUgjTJPzLpU9P");
}

pub struct ONYC;

impl TokenMint for ONYC {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5");
}

pub struct JLP;

impl TokenMint for JLP {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4");
}

pub struct HYPE;

impl TokenMint for HYPE {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("98sMhvDwXj1RQi5c5Mndm3vPe9cBqPrbLaufMXFNMh5g");
}

/// Canonical collateral-mint to oracle feed-id binding — the one place the
/// exchange learns which observation prices an EXO collateral. A mint with no
/// entry is not priceable (its feed has not launched), so it cannot register.
const EXO_FEEDS: &[(Pubkey, u16)] = &[(CBBTC::MINT, BTC_USD_FEED_ID)];

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

  #[test]
  fn cbbtc_maps_to_feed_id_one() {
    assert_eq!(feed_id_for(&CBBTC::MINT), Some(1));
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
