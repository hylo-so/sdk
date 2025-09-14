use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::pubkey;
use anyhow::{anyhow, Result};
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
  const MINT: Pubkey;
}

pub struct HYUSD;

impl TokenMint for HYUSD {
  const MINT: Pubkey = pubkey!("5YMkXAYccHSGnHn9nob9xEvv6Pvka9DZWH7nTbotTu9E");
}

try_from_pubkey!(HYUSD);

pub struct SHYUSD;

impl TokenMint for SHYUSD {
  const MINT: Pubkey = pubkey!("HnnGv3HrSqjRpgdFmx7vQGjntNEoex1SU4e9Lxcxuihz");
}

try_from_pubkey!(SHYUSD);

pub struct JITOSOL;

impl TokenMint for JITOSOL {
  const MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
}

try_from_pubkey!(JITOSOL);

pub struct XSOL;

impl TokenMint for XSOL {
  const MINT: Pubkey = pubkey!("4sWNB8zGWHkh6UnmwiEtzNxL4XrN7uK9tosbESbJFfVs");
}

try_from_pubkey!(XSOL);
