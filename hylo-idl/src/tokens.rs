use anchor_lang::prelude::{pubkey, Pubkey};
use anchor_spl::mint::USDC as USDC_MINT;
use fix::prelude::{N6, N8, N9};
use fix::typenum::Integer;

use crate::{earn_pool, exchange, pda};

macro_rules! token {
  ($name:ident, $exp:ty, $mint:expr) => {
    pub struct $name;

    impl TokenMint for $name {
      type Exp = $exp;
      const MINT: Pubkey = $mint;
    }
  };
}

pub trait TokenMint {
  type Exp: Integer;
  const MINT: Pubkey;
}

token!(
  HYUSD,
  N6,
  pda::mint(exchange::ID, exchange::constants::HYUSD)
);
token!(
  SHYUSD,
  N6,
  pda::mint(earn_pool::ID, earn_pool::constants::STAKED_HYUSD)
);
token!(XSOL, N6, pda::mint(exchange::ID, exchange::constants::XSOL));
token!(
  JITOSOL,
  N9,
  pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn")
);
token!(
  HYLOSOL,
  N9,
  pubkey!("hy1oXYgrBW6PVcJ4s6s2FKavRdwgWTXdfE69AxT7kPT")
);
token!(USDC, N6, USDC_MINT);
token!(
  CBBTC,
  N8,
  pubkey!("cbbtcf3aa214zXHbiAZQwf4122FBYbraNdFqgw4iMij")
);
token!(XBTC, N6, pda::exo_levercoin_mint(CBBTC::MINT));
token!(
  ZEC,
  N8,
  pubkey!("A7bdiYdS5GjqGFtxf17ppRHtDKPkkRqbKtR27dxvQXaS")
);
token!(XZEC, N6, pda::exo_levercoin_mint(ZEC::MINT));
token!(
  ONYC,
  N9,
  pubkey!("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5")
);
token!(XONYC, N6, pda::exo_levercoin_mint(ONYC::MINT));
token!(
  HYPE,
  N9,
  pubkey!("98sMhvDwXj1RQi5c5Mndm3vPe9cBqPrbLaufMXFNMh5g")
);
token!(XHYPE, N6, pda::exo_levercoin_mint(HYPE::MINT));
token!(
  PST,
  N6,
  pubkey!("59obFNBzyTBGowrkif5uK7ojS58vsuWz3ZCvg6tfZAGw")
);
token!(XPST, N6, pda::exo_levercoin_mint(PST::MINT));

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

impl Exo for CBBTC {}
impl Exo for ZEC {}
impl Exo for ONYC {}
impl Exo for HYPE {}
impl Exo for PST {}
