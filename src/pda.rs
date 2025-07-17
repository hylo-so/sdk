use crate::{exchange, stability_pool};

use anchor_client::solana_sdk::bpf_loader;
use anchor_lang::prelude::Pubkey;
use anchor_spl::associated_token::get_associated_token_address;

macro_rules! pda {
  ($program_id:expr, $base:expr) => {
    Pubkey::find_program_address(&[$base.as_ref()], &$program_id).0
  };
  ($program_id:expr, $base:expr, $key:expr) => {
    Pubkey::find_program_address(&[$base.as_ref(), $key.as_ref()], &$program_id)
      .0
  };
}

#[must_use]
pub fn hylo() -> Pubkey {
  pda!(exchange::ID, exchange::constants::HYLO)
}

#[must_use]
pub fn vault(mint: Pubkey) -> Pubkey {
  get_associated_token_address(&vault_auth(mint), &mint)
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
  get_associated_token_address(&fee_auth(mint), &mint)
}

#[must_use]
pub fn fee_auth(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::FEE_AUTH, mint)
}

#[must_use]
pub fn ata(auth: Pubkey, mint: Pubkey) -> Pubkey {
  get_associated_token_address(&auth, &mint)
}

#[must_use]
pub fn hyusd() -> Pubkey {
  pda!(exchange::ID, exchange::constants::HYUSD)
}

#[must_use]
pub fn hyusd_ata(auth: Pubkey) -> Pubkey {
  ata(auth, hyusd())
}

#[must_use]
pub fn xsol() -> Pubkey {
  pda!(exchange::ID, exchange::constants::XSOL)
}

#[must_use]
pub fn xsol_ata(auth: Pubkey) -> Pubkey {
  ata(auth, xsol())
}

#[must_use]
pub fn hyusd_auth() -> Pubkey {
  pda!(exchange::ID, exchange::constants::MINT_AUTH, hyusd())
}

#[must_use]
pub fn xsol_auth() -> Pubkey {
  pda!(exchange::ID, exchange::constants::MINT_AUTH, xsol())
}

#[must_use]
pub fn lst_registry_auth() -> Pubkey {
  pda!(exchange::ID, exchange::constants::LST_REGISTRY_AUTH)
}

#[must_use]
pub fn event_auth(program: Pubkey) -> Pubkey {
  pda!(program, "__event_authority")
}

#[must_use]
pub fn pool_config() -> Pubkey {
  pda!(stability_pool::ID, stability_pool::constants::POOL_CONFIG)
}

#[must_use]
pub fn shyusd() -> Pubkey {
  pda!(stability_pool::ID, stability_pool::constants::STAKED_HYUSD)
}

#[must_use]
pub fn shyusd_ata(auth: Pubkey) -> Pubkey {
  ata(auth, shyusd())
}

#[must_use]
pub fn pool_auth() -> Pubkey {
  pda!(stability_pool::ID, stability_pool::constants::POOL_AUTH)
}

#[must_use]
pub fn hyusd_pool() -> Pubkey {
  ata(pool_auth(), hyusd())
}

#[must_use]
pub fn xsol_pool() -> Pubkey {
  ata(pool_auth(), xsol())
}

#[must_use]
pub fn stability_pool_program_data() -> Pubkey {
  pda!(bpf_loader::ID, stability_pool::ID)
}

#[must_use]
pub fn exchange_program_data() -> Pubkey {
  pda!(bpf_loader::ID, exchange::ID)
}
