use std::sync::LazyLock;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::bpf_loader;

use crate::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use crate::{hylo_exchange, hylo_stability_pool};

macro_rules! lazy {
  ($x:expr) => {
    LazyLock::new(|| $x)
  };
}

macro_rules! pda {
  ($program_id:expr, $base:expr) => {
    Pubkey::find_program_address(&[$base.as_ref()], &$program_id).0
  };
  ($program_id:expr, $base:expr, $key:expr) => {
    Pubkey::find_program_address(&[$base.as_ref(), $key.as_ref()], &$program_id)
      .0
  };
}

#[macro_export]
macro_rules! ata {
  ($auth:expr, $mint:expr) => {
    anchor_spl::associated_token::get_associated_token_address(&$auth, &$mint)
  };
}

#[must_use]
pub fn hyusd_ata(auth: Pubkey) -> Pubkey {
  ata!(&auth, &HYUSD::MINT)
}

#[must_use]
pub fn xsol_ata(auth: Pubkey) -> Pubkey {
  ata!(&auth, &XSOL::MINT)
}

#[must_use]
pub fn shyusd_ata(auth: Pubkey) -> Pubkey {
  ata!(&auth, &SHYUSD::MINT)
}

#[must_use]
pub fn vault(mint: Pubkey) -> Pubkey {
  ata!(&vault_auth(mint), &mint)
}

#[must_use]
pub fn vault_auth(mint: Pubkey) -> Pubkey {
  pda!(
    hylo_exchange::ID,
    hylo_exchange::constants::VAULT_AUTH,
    mint
  )
}

#[must_use]
pub fn lst_header(mint: Pubkey) -> Pubkey {
  pda!(
    hylo_exchange::ID,
    hylo_exchange::constants::LST_HEADER,
    mint
  )
}

#[must_use]
pub fn fee_vault(mint: Pubkey) -> Pubkey {
  ata!(&fee_auth(mint), &mint)
}

#[must_use]
pub fn fee_auth(mint: Pubkey) -> Pubkey {
  pda!(hylo_exchange::ID, hylo_exchange::constants::FEE_AUTH, mint)
}

pub static HYLO: LazyLock<Pubkey> =
  lazy!(pda!(hylo_exchange::ID, hylo_exchange::constants::HYLO));

pub static HYUSD_AUTH: LazyLock<Pubkey> = lazy!(pda!(
  hylo_exchange::ID,
  hylo_exchange::constants::MINT_AUTH,
  HYUSD::MINT
));

pub static XSOL_AUTH: LazyLock<Pubkey> = lazy!(pda!(
  hylo_exchange::ID,
  hylo_exchange::constants::MINT_AUTH,
  XSOL::MINT
));

pub static LST_REGISTRY_AUTH: LazyLock<Pubkey> = lazy!(pda!(
  hylo_exchange::ID,
  hylo_exchange::constants::LST_REGISTRY_AUTH
));

pub static EXCHANGE_EVENT_AUTH: LazyLock<Pubkey> =
  lazy!(pda!(hylo_exchange::ID, "__event_authority"));

pub static STABILITY_POOL_EVENT_AUTH: LazyLock<Pubkey> =
  lazy!(pda!(hylo_stability_pool::ID, "__event_authority"));

pub static POOL_CONFIG: LazyLock<Pubkey> = lazy!(pda!(
  hylo_stability_pool::ID,
  hylo_stability_pool::constants::POOL_CONFIG
));

pub static SHYUSD_AUTH: LazyLock<Pubkey> = lazy!(pda!(
  hylo_stability_pool::ID,
  hylo_exchange::constants::MINT_AUTH,
  SHYUSD::MINT
));

pub static POOL_AUTH: LazyLock<Pubkey> = lazy!(pda!(
  hylo_stability_pool::ID,
  hylo_stability_pool::constants::POOL_AUTH
));

pub static HYUSD_POOL: LazyLock<Pubkey> = lazy!(ata!(POOL_AUTH, HYUSD::MINT));

pub static XSOL_POOL: LazyLock<Pubkey> = lazy!(ata!(POOL_AUTH, XSOL::MINT));

pub static STABILITY_POOL_PROGRAM_DATA: LazyLock<Pubkey> =
  lazy!(pda!(bpf_loader::ID, hylo_stability_pool::ID));

pub static EXCHANGE_PROGRAM_DATA: LazyLock<Pubkey> =
  lazy!(pda!(bpf_loader::ID, hylo_exchange::ID));
