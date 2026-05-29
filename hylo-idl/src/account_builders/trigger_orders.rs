//! Account-context builders for `hylo-trigger-orders`.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar::rent;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};

use crate::tokens::{TokenMint, HYUSD, XSOL};
use crate::trigger_orders::client::accounts::{
  CreateOrderL2sExo, CreateOrderL2sLst, CreateOrderS2lExo, CreateOrderS2lLst,
};
use crate::{pda, trigger_orders};

/// Builds account context for creating a stable-to-lever order (LST).
#[must_use]
pub fn create_order_s2l_lst(owner: Pubkey, order: Pubkey) -> CreateOrderS2lLst {
  CreateOrderS2lLst {
    owner,
    order,
    order_hyusd_vault: pda::ata(order, HYUSD::MINT),
    owner_hyusd_ta: pda::ata(owner, HYUSD::MINT),
    hyusd_mint: HYUSD::MINT,
    event_authority: pda::trigger_orders_event_authority(),
    program: trigger_orders::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    rent: rent::ID,
  }
}

/// Builds account context for creating a stable-to-lever order (EXO).
#[must_use]
pub fn create_order_s2l_exo(
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
) -> CreateOrderS2lExo {
  CreateOrderS2lExo {
    owner,
    order,
    order_hyusd_vault: pda::ata(order, HYUSD::MINT),
    owner_hyusd_ta: pda::ata(owner, HYUSD::MINT),
    hyusd_mint: HYUSD::MINT,
    collateral_mint,
    event_authority: pda::trigger_orders_event_authority(),
    program: trigger_orders::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    rent: rent::ID,
  }
}

/// Builds account context for creating a lever-to-stable order (LST).
#[must_use]
pub fn create_order_l2s_lst(owner: Pubkey, order: Pubkey) -> CreateOrderL2sLst {
  CreateOrderL2sLst {
    owner,
    order,
    order_xsol_vault: pda::ata(order, XSOL::MINT),
    owner_xsol_ta: pda::ata(owner, XSOL::MINT),
    xsol_mint: XSOL::MINT,
    event_authority: pda::trigger_orders_event_authority(),
    program: trigger_orders::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    rent: rent::ID,
  }
}

/// Builds account context for creating a lever-to-stable order (EXO).
#[must_use]
pub fn create_order_l2s_exo(
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
) -> CreateOrderL2sExo {
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  CreateOrderL2sExo {
    owner,
    order,
    order_levercoin_vault: pda::ata(order, levercoin_mint),
    owner_levercoin_ta: pda::ata(owner, levercoin_mint),
    levercoin_mint,
    collateral_mint,
    event_authority: pda::trigger_orders_event_authority(),
    program: trigger_orders::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    rent: rent::ID,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn create_order_s2l_lst_builds_correct_pdas() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let a = create_order_s2l_lst(owner, order);

    assert_eq!(a.owner, owner);
    assert_eq!(a.order, order);
    assert_eq!(a.hyusd_mint, HYUSD::MINT);
    assert_eq!(a.order_hyusd_vault, pda::ata(order, HYUSD::MINT));
    assert_eq!(a.owner_hyusd_ta, pda::ata(owner, HYUSD::MINT));
    assert_eq!(a.event_authority, pda::trigger_orders_event_authority());
    assert_eq!(a.program, trigger_orders::ID);
    assert_eq!(a.token_program, token::ID);
    assert_eq!(a.associated_token_program, associated_token::ID);
    assert_eq!(a.system_program, system_program::ID);
    assert_eq!(a.rent, rent::ID);
  }

  #[test]
  fn create_order_s2l_exo_builds_correct_pdas() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let a = create_order_s2l_exo(owner, order, collateral_mint);

    assert_eq!(a.owner, owner);
    assert_eq!(a.order, order);
    assert_eq!(a.hyusd_mint, HYUSD::MINT);
    assert_eq!(a.order_hyusd_vault, pda::ata(order, HYUSD::MINT));
    assert_eq!(a.owner_hyusd_ta, pda::ata(owner, HYUSD::MINT));
    assert_eq!(a.collateral_mint, collateral_mint);
    assert_eq!(a.event_authority, pda::trigger_orders_event_authority());
    assert_eq!(a.program, trigger_orders::ID);
    assert_eq!(a.token_program, token::ID);
    assert_eq!(a.associated_token_program, associated_token::ID);
    assert_eq!(a.system_program, system_program::ID);
    assert_eq!(a.rent, rent::ID);
  }

  #[test]
  fn create_order_l2s_lst_builds_correct_pdas() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let a = create_order_l2s_lst(owner, order);

    assert_eq!(a.owner, owner);
    assert_eq!(a.order, order);
    assert_eq!(a.xsol_mint, XSOL::MINT);
    assert_eq!(a.order_xsol_vault, pda::ata(order, XSOL::MINT));
    assert_eq!(a.owner_xsol_ta, pda::ata(owner, XSOL::MINT));
    assert_eq!(a.event_authority, pda::trigger_orders_event_authority());
    assert_eq!(a.program, trigger_orders::ID);
    assert_eq!(a.token_program, token::ID);
    assert_eq!(a.associated_token_program, associated_token::ID);
    assert_eq!(a.system_program, system_program::ID);
    assert_eq!(a.rent, rent::ID);
  }

  #[test]
  fn create_order_l2s_exo_builds_correct_pdas() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
    let a = create_order_l2s_exo(owner, order, collateral_mint);

    assert_eq!(a.owner, owner);
    assert_eq!(a.order, order);
    assert_eq!(a.levercoin_mint, levercoin_mint);
    assert_eq!(a.order_levercoin_vault, pda::ata(order, levercoin_mint));
    assert_eq!(a.owner_levercoin_ta, pda::ata(owner, levercoin_mint));
    assert_eq!(a.collateral_mint, collateral_mint);
    assert_eq!(a.event_authority, pda::trigger_orders_event_authority());
    assert_eq!(a.program, trigger_orders::ID);
    assert_eq!(a.token_program, token::ID);
    assert_eq!(a.associated_token_program, associated_token::ID);
    assert_eq!(a.system_program, system_program::ID);
    assert_eq!(a.rent, rent::ID);
  }
}
