use anchor_lang::prelude::{bpf_loader_upgradeable, pubkey, Pubkey};
use anchor_spl::associated_token::spl_associated_token_account;
use anchor_spl::token;
use const_crypto::ed25519;
use solana_address_lookup_table_interface::program as address_lookup_table;

use crate::exchange::types::AddressField;
use crate::tokens::{TokenMint, HYUSD, SHYUSD, USDC, XSOL};
use crate::{earn_pool, exchange};

macro_rules! pda {
  ($program_id:expr, $base:expr) => {{
    let (key, _bump) = ed25519::derive_program_address(
      &[$base.as_slice()],
      $program_id.as_array(),
    );
    Pubkey::new_from_array(key)
  }};
  ($program_id:expr, $base:expr, $key:expr) => {{
    let (key, _bump) = ed25519::derive_program_address(
      &[$base.as_slice(), $key.as_array()],
      $program_id.as_array(),
    );
    Pubkey::new_from_array(key)
  }};
}

macro_rules! pda_with_bump {
  ($program_id:expr, $base:expr) => {{
    let (key, bump) = ed25519::derive_program_address(
      &[$base.as_slice()],
      $program_id.as_array(),
    );
    (Pubkey::new_from_array(key), bump)
  }};
}

#[must_use]
pub const fn mint<const N: usize>(program_id: Pubkey, seed: [u8; N]) -> Pubkey {
  let (key, _bump) =
    ed25519::derive_program_address(&[&seed], program_id.as_array());
  Pubkey::new_from_array(key)
}

#[must_use]
pub const fn ata(auth: Pubkey, mint: Pubkey) -> Pubkey {
  let (key, _bump) = ed25519::derive_program_address(
    &[auth.as_array(), token::ID.as_array(), mint.as_array()],
    spl_associated_token_account::ID.as_array(),
  );
  Pubkey::new_from_array(key)
}

#[must_use]
pub const fn progdata(program_id: Pubkey) -> Pubkey {
  let (key, _bump) = ed25519::derive_program_address(
    &[program_id.as_array()],
    bpf_loader_upgradeable::ID.as_array(),
  );
  Pubkey::new_from_array(key)
}

#[must_use]
pub const fn metadata(mint: Pubkey) -> Pubkey {
  let (key, _bump) = ed25519::derive_program_address(
    &[
      b"metadata",
      mpl_token_metadata::ID.as_array(),
      mint.as_array(),
    ],
    mpl_token_metadata::ID.as_array(),
  );
  Pubkey::new_from_array(key)
}

#[must_use]
pub const fn hyusd_ata(auth: Pubkey) -> Pubkey {
  ata(auth, HYUSD::MINT)
}

#[must_use]
pub const fn xsol_ata(auth: Pubkey) -> Pubkey {
  ata(auth, XSOL::MINT)
}

#[must_use]
pub const fn shyusd_ata(auth: Pubkey) -> Pubkey {
  ata(auth, SHYUSD::MINT)
}

#[must_use]
pub const fn usdc_ata(auth: Pubkey) -> Pubkey {
  ata(auth, USDC::MINT)
}

#[must_use]
pub const fn lst_vault(mint: Pubkey) -> Pubkey {
  ata(lst_vault_auth(mint), mint)
}

#[must_use]
pub const fn exo_vault(mint: Pubkey) -> Pubkey {
  ata(exo_vault_auth(mint), mint)
}

#[must_use]
pub const fn usdc_vault(mint: Pubkey) -> Pubkey {
  ata(usdc_vault_auth(mint), mint)
}

#[must_use]
pub const fn lst_vault_auth(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::VAULT_AUTH, mint)
}

#[must_use]
pub const fn exo_vault_auth(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::EXO_VAULT_AUTH, mint)
}

#[must_use]
pub const fn usdc_vault_auth(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::USDC_VAULT_AUTH, mint)
}

#[must_use]
pub const fn new_lst_registry(slot: u64) -> Pubkey {
  let (key, _bump) = ed25519::derive_program_address(
    &[LST_REGISTRY_AUTH.as_array(), &slot.to_le_bytes()],
    address_lookup_table::ID.as_array(),
  );
  Pubkey::new_from_array(key)
}

#[must_use]
pub const fn lst_header(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::LST_HEADER, mint)
}

#[must_use]
pub const fn fee_vault(mint: Pubkey) -> Pubkey {
  ata(fee_auth(mint), mint)
}

#[must_use]
pub const fn fee_auth(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::FEE_AUTH, mint)
}

#[must_use]
pub const fn mint_auth(mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::MINT_AUTH, mint)
}

#[must_use]
pub const fn event_auth(program_id: Pubkey) -> Pubkey {
  let (key, _bump) = ed25519::derive_program_address(
    &[b"__event_authority"],
    program_id.as_array(),
  );
  Pubkey::new_from_array(key)
}

#[must_use]
pub const fn exo_pair(collateral_mint: Pubkey) -> Pubkey {
  pda!(exchange::ID, exchange::constants::EXO_PAIR, collateral_mint)
}

#[must_use]
pub fn address_update_proposal(field: AddressField) -> Pubkey {
  let (key, _bump) = ed25519::derive_program_address(
    &[
      exchange::constants::ADDRESS_UPDATE_PROPOSAL.as_slice(),
      &[field as u8],
    ],
    exchange::ID.as_array(),
  );
  Pubkey::new_from_array(key)
}

#[must_use]
pub const fn exo_levercoin_mint(collateral_mint: Pubkey) -> Pubkey {
  pda!(
    exchange::ID,
    exchange::constants::EXO_LEVERCOIN,
    collateral_mint
  )
}

pub const HYLO: Pubkey = pda!(exchange::ID, exchange::constants::HYLO);

pub const HYUSD_AUTH: Pubkey = mint_auth(HYUSD::MINT);

pub const XSOL_AUTH: Pubkey = mint_auth(XSOL::MINT);

pub const LST_REGISTRY_AUTH: Pubkey =
  pda!(exchange::ID, exchange::constants::LST_REGISTRY_AUTH);

pub const POOL_CONFIG: Pubkey =
  pda!(earn_pool::ID, earn_pool::constants::POOL_CONFIG);

pub const SHYUSD_AUTH: Pubkey =
  pda!(earn_pool::ID, exchange::constants::MINT_AUTH, SHYUSD::MINT);

pub const POOL_AUTH: Pubkey =
  pda!(earn_pool::ID, earn_pool::constants::POOL_AUTH);

const SETTLEMENT_AUTH_DERIVED: (Pubkey, u8) =
  pda_with_bump!(exchange::ID, exchange::constants::SETTLEMENT_AUTH);

pub const SETTLEMENT_AUTH: Pubkey = SETTLEMENT_AUTH_DERIVED.0;

pub const SETTLEMENT_AUTH_BUMP: u8 = SETTLEMENT_AUTH_DERIVED.1;

pub const HYUSD_POOL: Pubkey = ata(POOL_AUTH, HYUSD::MINT);

pub const XSOL_POOL: Pubkey = ata(POOL_AUTH, XSOL::MINT);

pub const EARN_POOL_PROGRAM_DATA: Pubkey = progdata(earn_pool::ID);

pub const EXCHANGE_PROGRAM_DATA: Pubkey = progdata(exchange::ID);

pub const SOL_USD_PYTH_FEED: Pubkey =
  pubkey!("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");

pub const USDC_USD_PYTH_FEED: Pubkey =
  pubkey!("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX");

pub const BTC_USD_PYTH_FEED: Pubkey =
  pubkey!("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo");

pub const EXCHANGE_EVENT_AUTHORITY: Pubkey = event_auth(exchange::ID);

pub const EARN_POOL_EVENT_AUTHORITY: Pubkey = event_auth(earn_pool::ID);

pub const USDC_PAIR: Pubkey =
  pda!(exchange::ID, exchange::constants::USDC_PAIR);

use crate::trigger_orders::types::{ConvertDirection, PairTarget};

/// Seed for `TriggerOrder` PDAs — byte-equal to `hylo_trigger_orders::ORDER`.
pub const TRIGGER_ORDER: &[u8; 5] = b"order";

/// PDA for an LST trigger order (s2l or l2s).
#[must_use]
pub fn trigger_order_lst(
  owner: Pubkey,
  direction: ConvertDirection,
  nonce: u64,
) -> (Pubkey, u8) {
  Pubkey::find_program_address(
    &[
      TRIGGER_ORDER,
      owner.as_ref(),
      &[direction.tag_byte()],
      &[PairTarget::LST_TAG],
      &nonce.to_le_bytes(),
    ],
    &crate::trigger_orders::ID,
  )
}

/// PDA for an EXO trigger order (s2l or l2s).
#[must_use]
pub fn trigger_order_exo(
  owner: Pubkey,
  direction: ConvertDirection,
  collateral_mint: Pubkey,
  nonce: u64,
) -> (Pubkey, u8) {
  Pubkey::find_program_address(
    &[
      TRIGGER_ORDER,
      owner.as_ref(),
      &[direction.tag_byte()],
      &[PairTarget::EXO_TAG],
      collateral_mint.as_ref(),
      &nonce.to_le_bytes(),
    ],
    &crate::trigger_orders::ID,
  )
}

/// `__event_authority` PDA for the trigger-orders program.
#[must_use]
pub fn trigger_orders_event_authority() -> Pubkey {
  Pubkey::find_program_address(
    &[b"__event_authority"],
    &crate::trigger_orders::ID,
  )
  .0
}

#[cfg(test)]
mod trigger_orders_pda_tests {
  use anchor_lang::prelude::Pubkey;

  use super::*;
  use crate::trigger_orders::types::{ConvertDirection, PairTarget};

  #[test]
  fn lst_pda_seeds_round_trip() {
    let owner = Pubkey::new_unique();
    let (pda, _bump) =
      trigger_order_lst(owner, ConvertDirection::StableToLever, 42);
    let (expected, _) = Pubkey::find_program_address(
      &[
        TRIGGER_ORDER,
        owner.as_ref(),
        &[ConvertDirection::S2L_TAG],
        &[PairTarget::LST_TAG],
        &42u64.to_le_bytes(),
      ],
      &crate::trigger_orders::ID,
    );
    assert_eq!(pda, expected);
  }

  #[test]
  fn exo_pda_seeds_include_collateral_mint() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let (pda, _bump) =
      trigger_order_exo(owner, ConvertDirection::LeverToStable, mint, 7);
    let (expected, _) = Pubkey::find_program_address(
      &[
        TRIGGER_ORDER,
        owner.as_ref(),
        &[ConvertDirection::L2S_TAG],
        &[PairTarget::EXO_TAG],
        mint.as_ref(),
        &7u64.to_le_bytes(),
      ],
      &crate::trigger_orders::ID,
    );
    assert_eq!(pda, expected);
  }

  #[test]
  fn lst_and_exo_pdas_disjoint_at_same_owner_nonce() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let (lst, _) = trigger_order_lst(owner, ConvertDirection::StableToLever, 1);
    let (exo, _) =
      trigger_order_exo(owner, ConvertDirection::StableToLever, mint, 1);
    assert_ne!(lst, exo, "PDA-tag separation broken");
  }

  #[test]
  fn s2l_and_l2s_pdas_disjoint_at_same_owner_pair_nonce() {
    let owner = Pubkey::new_unique();
    let (s2l, _) = trigger_order_lst(owner, ConvertDirection::StableToLever, 1);
    let (l2s, _) = trigger_order_lst(owner, ConvertDirection::LeverToStable, 1);
    assert_ne!(s2l, l2s, "direction-tag separation broken");
  }
}
