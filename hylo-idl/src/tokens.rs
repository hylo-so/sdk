use anchor_lang::prelude::{pubkey, Pubkey};
use anchor_spl::mint::USDC as USDC_MINT;
use anchor_spl::token;
use fix::prelude::{N6, N8, N9};
use fix::typenum::Integer;

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
  const TOKEN_PROGRAM: Pubkey;
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
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for SHYUSD {
  type Exp = N6;
  const MINT: Pubkey =
    pda::mint(earn_pool::ID, earn_pool::constants::STAKED_HYUSD);
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for XSOL {
  type Exp = N6;
  const MINT: Pubkey = pda::mint(exchange::ID, exchange::constants::XSOL);
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for JITOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for HYLOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("hy1oXYgrBW6PVcJ4s6s2FKavRdwgWTXdfE69AxT7kPT");
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for USDC {
  type Exp = N6;
  const MINT: Pubkey = USDC_MINT;
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for CBBTC {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("cbbtcf3aa214zXHbiAZQwf4122FBYbraNdFqgw4iMij");
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for XBTC {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(CBBTC::MINT);
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for ZEC {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("A7bdiYdS5GjqGFtxf17ppRHtDKPkkRqbKtR27dxvQXaS");
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for XZEC {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(ZEC::MINT);
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for ONYC {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5");
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for XONYC {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(ONYC::MINT);
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for HYPE {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("98sMhvDwXj1RQi5c5Mndm3vPe9cBqPrbLaufMXFNMh5g");
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for XHYPE {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(HYPE::MINT);
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for PST {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("59obFNBzyTBGowrkif5uK7ojS58vsuWz3ZCvg6tfZAGw");
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for XPST {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(PST::MINT);
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for WETH {
  type Exp = N8;
  const MINT: Pubkey = pubkey!("7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs");
  const TOKEN_PROGRAM: Pubkey = token::ID;
}

impl TokenMint for XETH {
  type Exp = N6;
  const MINT: Pubkey = pda::exo_levercoin_mint(WETH::MINT);
  const TOKEN_PROGRAM: Pubkey = token::ID;
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
