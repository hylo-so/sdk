use anchor_lang::prelude::{pubkey, Pubkey};
use fix::prelude::{N6, N9};
use fix::typenum::Integer;

pub trait TokenMint {
  type Exp: Integer;
  const MINT: Pubkey;
}

pub struct HYUSD;

impl TokenMint for HYUSD {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("5YMkXAYccHSGnHn9nob9xEvv6Pvka9DZWH7nTbotTu9E");
}

pub struct SHYUSD;

impl TokenMint for SHYUSD {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("HnnGv3HrSqjRpgdFmx7vQGjntNEoex1SU4e9Lxcxuihz");
}

pub struct XSOL;

impl TokenMint for XSOL {
  type Exp = N6;
  const MINT: Pubkey = pubkey!("4sWNB8zGWHkh6UnmwiEtzNxL4XrN7uK9tosbESbJFfVs");
}

pub struct JITOSOL;

impl TokenMint for JITOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
}

pub struct HYLOSOL;

impl TokenMint for HYLOSOL {
  type Exp = N9;
  const MINT: Pubkey = pubkey!("hy1oXYgrBW6PVcJ4s6s2FKavRdwgWTXdfE69AxT7kPT");
}
