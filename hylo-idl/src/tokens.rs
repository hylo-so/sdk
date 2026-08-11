use anchor_lang::prelude::{pubkey, Pubkey};
use anchor_spl::mint::USDC as USDC_MINT;
use fix::prelude::{N6, N8, N9};
use fix::typenum::Integer;

use crate::{earn_pool, exchange, pda};

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
