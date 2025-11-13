use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::pubkey;

pub trait TokenMint {
  const MINT: Pubkey;
}

pub struct HYUSD;

impl TokenMint for HYUSD {
  const MINT: Pubkey = pubkey!("5YMkXAYccHSGnHn9nob9xEvv6Pvka9DZWH7nTbotTu9E");
}

pub struct SHYUSD;

impl TokenMint for SHYUSD {
  const MINT: Pubkey = pubkey!("HnnGv3HrSqjRpgdFmx7vQGjntNEoex1SU4e9Lxcxuihz");
}

pub struct XSOL;

impl TokenMint for XSOL {
  const MINT: Pubkey = pubkey!("4sWNB8zGWHkh6UnmwiEtzNxL4XrN7uK9tosbESbJFfVs");
}

pub struct JITOSOL;

impl TokenMint for JITOSOL {
  const MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
}

pub struct HYLOSOL;

impl TokenMint for HYLOSOL {
  const MINT: Pubkey = pubkey!("hy1oXYgrBW6PVcJ4s6s2FKavRdwgWTXdfE69AxT7kPT");
}
