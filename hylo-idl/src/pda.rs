use crate::{exchange, stability_pool};

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::bpf_loader;
use std::sync::LazyLock;

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
  ata!(&auth, &HYUSD)
}

#[must_use]
pub fn xsol_ata(auth: Pubkey) -> Pubkey {
  ata!(&auth, &XSOL)
}

#[must_use]
pub fn shyusd_ata(auth: Pubkey) -> Pubkey {
  ata!(&auth, &SHYUSD)
}

#[must_use]
pub fn vault(mint: Pubkey) -> Pubkey {
  ata!(&vault_auth(mint), &mint)
}

#[must_use]
pub fn vault_auth(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::VAULT_AUTH, mint)
}

#[must_use]
pub fn lst_header(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::LST_HEADER, mint)
}

#[must_use]
pub fn fee_vault(mint: Pubkey) -> Pubkey {
  ata!(&fee_auth(mint), &mint)
}

#[must_use]
pub fn fee_auth(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::FEE_AUTH, mint)
}

pub static HYLO: LazyLock<Pubkey> =
  lazy!(pda!(exchange::ID, exchange::constants::HYLO));

pub static HYUSD: LazyLock<Pubkey> =
  lazy!(pda!(exchange::ID, exchange::constants::HYUSD));

pub static XSOL: LazyLock<Pubkey> =
  lazy!(pda!(exchange::ID, exchange::constants::XSOL));

pub static HYUSD_AUTH: LazyLock<Pubkey> =
  lazy!(pda!(exchange::ID, exchange::constants::MINT_AUTH, *HYUSD));

pub static XSOL_AUTH: LazyLock<Pubkey> =
  lazy!(pda!(exchange::ID, exchange::constants::MINT_AUTH, *XSOL));

pub static LST_REGISTRY_AUTH: LazyLock<Pubkey> =
  lazy!(pda!(exchange::ID, exchange::constants::LST_REGISTRY_AUTH));

pub static EXCHANGE_EVENT_AUTH: LazyLock<Pubkey> =
  lazy!(pda!(exchange::ID, "__event_authority"));

pub static STABILITY_POOL_EVENT_AUTH: LazyLock<Pubkey> =
  lazy!(pda!(stability_pool::ID, "__event_authority"));

pub static POOL_CONFIG: LazyLock<Pubkey> = lazy!(pda!(
  stability_pool::ID,
  stability_pool::constants::POOL_CONFIG
));

pub static SHYUSD: LazyLock<Pubkey> = lazy!(pda!(
  stability_pool::ID,
  stability_pool::constants::STAKED_HYUSD
));

pub static POOL_AUTH: LazyLock<Pubkey> = lazy!(pda!(
  stability_pool::ID,
  stability_pool::constants::POOL_AUTH
));

pub static HYUSD_POOL: LazyLock<Pubkey> = lazy!(ata!(POOL_AUTH, HYUSD));

pub static XSOL_POOL: LazyLock<Pubkey> = lazy!(ata!(POOL_AUTH, XSOL));

pub static STABILITY_POOL_PROGRAM_DATA: LazyLock<Pubkey> =
  lazy!(pda!(bpf_loader::ID, stability_pool::ID));

pub static EXCHANGE_PROGRAM_DATA: LazyLock<Pubkey> =
  lazy!(pda!(bpf_loader::ID, exchange::ID));
