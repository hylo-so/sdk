use std::sync::LazyLock;

use solana_loader_v3_interface::get_program_data_address;
use solana_pubkey::Pubkey;
use solana_sdk_ids::address_lookup_table;

use crate::tokens::{HYUSD, SHYUSD, XSOL};
use crate::{hylo_exchange, hylo_stability_pool, MPL_TOKEN_METADATA_ID};

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
    spl_associated_token_account_interface::address::get_associated_token_address(&$auth, &$mint)
  };
}

#[must_use]
pub fn metadata(mint: Pubkey) -> Pubkey {
  Pubkey::find_program_address(
    &[
      "metadata".as_ref(),
      MPL_TOKEN_METADATA_ID.as_ref(),
      mint.as_ref(),
    ],
    &MPL_TOKEN_METADATA_ID,
  )
  .0
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
  pda!(
    hylo_exchange::ID,
    hylo_exchange::constants::VAULT_AUTH,
    mint
  )
}

#[must_use]
pub fn new_lst_registry(slot: u64) -> Pubkey {
  Pubkey::find_program_address(
    &[LST_REGISTRY_AUTH.as_ref(), &slot.to_le_bytes()],
    &address_lookup_table::ID,
  )
  .0
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
  HYUSD
));

pub static XSOL_AUTH: LazyLock<Pubkey> = lazy!(pda!(
  hylo_exchange::ID,
  hylo_exchange::constants::MINT_AUTH,
  XSOL
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
  SHYUSD
));

pub static POOL_AUTH: LazyLock<Pubkey> = lazy!(pda!(
  hylo_stability_pool::ID,
  hylo_stability_pool::constants::POOL_AUTH
));

pub static HYUSD_POOL: LazyLock<Pubkey> = lazy!(ata!(POOL_AUTH, HYUSD));

pub static XSOL_POOL: LazyLock<Pubkey> = lazy!(ata!(POOL_AUTH, XSOL));

pub static STABILITY_POOL_PROGRAM_DATA: LazyLock<Pubkey> =
  lazy!(get_program_data_address(&hylo_stability_pool::ID));

pub static EXCHANGE_PROGRAM_DATA: LazyLock<Pubkey> =
  lazy!(get_program_data_address(&hylo_exchange::ID));

pub const SOL_USD_PYTH_FEED: Pubkey =
  Pubkey::from_str_const("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");
