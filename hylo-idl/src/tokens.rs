use anchor_lang::prelude::{pubkey, Pubkey};
use anchor_spl::mint::USDC as USDC_MINT;
use fix::prelude::{N6, N8, N9};
use fix::typenum::Integer;

use crate::{exchange, pda, stability_pool};

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
    pda::mint(stability_pool::ID, stability_pool::constants::STAKED_HYUSD);
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
