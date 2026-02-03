use anchor_lang::prelude::{pubkey, Pubkey};
use anyhow::{anyhow, Result};
use fix::prelude::{N6, N9};
use fix::typenum::Integer;
use paste::paste;

macro_rules! try_from_pubkey {
  ($token:ty) => {
    paste! {
       impl TryFrom<Pubkey> for $token {
         type Error = anyhow::Error;
         fn try_from(k: Pubkey) -> Result<Self> {
           match k {
             $token::MINT => Ok($token),
             _ => Err(anyhow!("Hylo: {k} is not a supported token.")),
           }
         }
       }
    }
  };
}

pub trait TokenMint {
  type Exp: Integer;
  const MINT: Pubkey;
}

pub struct HYUSD;

impl TokenMint for HYUSD {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("5YMkXAYccHSGnHn9nob9xEvv6Pvka9DZWH7nTbotTu9E");
}

try_from_pubkey!(HYUSD);

pub struct SHYUSD;

impl TokenMint for SHYUSD {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("HnnGv3HrSqjRpgdFmx7vQGjntNEoex1SU4e9Lxcxuihz");
}

try_from_pubkey!(SHYUSD);

pub struct XSOL;

impl TokenMint for XSOL {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("4sWNB8zGWHkh6UnmwiEtzNxL4XrN7uK9tosbESbJFfVs");
}

try_from_pubkey!(XSOL);

pub struct JITOSOL;

impl TokenMint for JITOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
}

try_from_pubkey!(JITOSOL);

pub struct HYLOSOL;

impl TokenMint for HYLOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("hy1oXYgrBW6PVcJ4s6s2FKavRdwgWTXdfE69AxT7kPT");
}

try_from_pubkey!(HYLOSOL);
