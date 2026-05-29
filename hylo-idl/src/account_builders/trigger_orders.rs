//! Account-context builders for `hylo-trigger-orders`.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar::rent;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};

use crate::exchange::accounts::{ExoPair, Hylo};
use crate::tokens::{TokenMint, HYUSD, XSOL};
use crate::trigger_orders::client::accounts::{
  CreateOrderL2sExo, CreateOrderL2sLst, CreateOrderS2lExo, CreateOrderS2lLst,
  ExecuteOrderL2sExo, ExecuteOrderL2sLst, ExecuteOrderS2lExo,
  ExecuteOrderS2lLst,
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

/// Builds account context for executing a stable-to-lever order (LST).
#[must_use]
pub fn execute_order_s2l_lst(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  hylo: &Hylo,
) -> ExecuteOrderS2lLst {
  ExecuteOrderS2lLst {
    executor,
    owner,
    hylo: pda::HYLO,
    sol_usd_pyth_feed: hylo.sol_usd_oracle,
    hyusd_mint: HYUSD::MINT,
    stablecoin_auth: pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    xsol_mint: XSOL::MINT,
    levercoin_auth: pda::XSOL_AUTH,
    order,
    order_hyusd_vault: pda::ata(order, HYUSD::MINT),
    order_xsol_vault: pda::ata(order, XSOL::MINT),
    owner_xsol_ta: pda::ata(owner, XSOL::MINT),
    hylo_exchange_event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    hylo_exchange_program: crate::exchange::ID,
    event_authority: pda::trigger_orders_event_authority(),
    program: trigger_orders::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
  }
}

/// Builds account context for executing a lever-to-stable order (LST).
#[must_use]
pub fn execute_order_l2s_lst(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  hylo: &Hylo,
) -> ExecuteOrderL2sLst {
  ExecuteOrderL2sLst {
    executor,
    owner,
    hylo: pda::HYLO,
    sol_usd_pyth_feed: hylo.sol_usd_oracle,
    hyusd_mint: HYUSD::MINT,
    stablecoin_auth: pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    xsol_mint: XSOL::MINT,
    levercoin_auth: pda::XSOL_AUTH,
    order,
    order_xsol_vault: pda::ata(order, XSOL::MINT),
    order_hyusd_vault: pda::ata(order, HYUSD::MINT),
    owner_hyusd_ta: pda::ata(owner, HYUSD::MINT),
    hylo_exchange_event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    hylo_exchange_program: crate::exchange::ID,
    event_authority: pda::trigger_orders_event_authority(),
    program: trigger_orders::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
  }
}

/// Builds account context for executing a stable-to-lever order (EXO).
///
/// `_hylo` is accepted for signature symmetry with the LST builders; the EXO
/// CPI uses the per-pair `exo_pair.oracle` rather than the Hylo SOL/USD oracle.
#[must_use]
pub fn execute_order_s2l_exo(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  _hylo: &Hylo,
  exo_pair: &ExoPair,
) -> ExecuteOrderS2lExo {
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  let vault_auth = pda::exo_vault_auth(collateral_mint);
  ExecuteOrderS2lExo {
    executor,
    owner,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    vault_auth,
    collateral_vault: pda::ata(vault_auth, collateral_mint),
    collateral_usd_pyth_feed: exo_pair.oracle,
    hyusd_mint: HYUSD::MINT,
    stablecoin_auth: pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    levercoin_mint,
    levercoin_auth: pda::mint_auth(levercoin_mint),
    order,
    order_hyusd_vault: pda::ata(order, HYUSD::MINT),
    order_levercoin_vault: pda::ata(order, levercoin_mint),
    owner_levercoin_ta: pda::ata(owner, levercoin_mint),
    hylo_exchange_event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    hylo_exchange_program: crate::exchange::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    event_authority: pda::trigger_orders_event_authority(),
    program: trigger_orders::ID,
  }
}

/// Builds account context for executing a lever-to-stable order (EXO).
///
/// `_hylo` is accepted for signature symmetry with the LST builders; the EXO
/// CPI uses the per-pair `exo_pair.oracle` rather than the Hylo SOL/USD oracle.
#[must_use]
pub fn execute_order_l2s_exo(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  _hylo: &Hylo,
  exo_pair: &ExoPair,
) -> ExecuteOrderL2sExo {
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  let vault_auth = pda::exo_vault_auth(collateral_mint);
  ExecuteOrderL2sExo {
    executor,
    owner,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    vault_auth,
    collateral_vault: pda::ata(vault_auth, collateral_mint),
    collateral_usd_pyth_feed: exo_pair.oracle,
    hyusd_mint: HYUSD::MINT,
    stablecoin_auth: pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    levercoin_mint,
    levercoin_auth: pda::mint_auth(levercoin_mint),
    order,
    order_levercoin_vault: pda::ata(order, levercoin_mint),
    order_hyusd_vault: pda::ata(order, HYUSD::MINT),
    owner_hyusd_ta: pda::ata(owner, HYUSD::MINT),
    hylo_exchange_event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    hylo_exchange_program: crate::exchange::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    event_authority: pda::trigger_orders_event_authority(),
    program: trigger_orders::ID,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::exchange::types::{
    BorrowRateConfig, HarvestCache, LevercoinFees, PoolDrawdown,
    RebalanceCurveConfig, UFixValue64, VirtualStablecoin,
  };

  fn healthy_hylo_min() -> crate::exchange::accounts::Hylo {
    crate::exchange::accounts::Hylo::default()
  }

  // `ExoPair` does NOT derive `Default` (its `_reserved: [u8; 100]` exceeds
  // the 32-element auto-Default array limit), so list every field explicitly.
  // The builder only reads `exo_pair.oracle`, which the test sets explicitly.
  fn healthy_exo_pair_min() -> ExoPair {
    ExoPair {
      collateral_mint: Pubkey::new_unique(),
      levercoin_mint_bump: 0,
      levercoin_auth_bump: 0,
      vault_auth_bump: 0,
      fee_auth_bump: 0,
      oracle: Pubkey::new_unique(),
      oracle_feed_id: [0u8; 32],
      oracle_interval_secs: 30,
      oracle_conf_tolerance: UFixValue64::default(),
      stablecoin_mint_threshold: UFixValue64::default(),
      virtual_stablecoin: VirtualStablecoin::default(),
      borrow_rate_config: BorrowRateConfig::default(),
      borrow_rate_harvest_cache: HarvestCache::default(),
      levercoin_fees: LevercoinFees::default(),
      sell_curve_config: RebalanceCurveConfig::default(),
      buy_curve_config: RebalanceCurveConfig::default(),
      rebalance_deviation_tolerance: UFixValue64::default(),
      paused: false,
      levercoin_market_cap_limit: UFixValue64::default(),
      pool_drawdown: PoolDrawdown::default(),
      _reserved: [0u8; 100],
    }
  }

  #[test]
  fn execute_order_s2l_lst_threads_hylo_oracle() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let mut hylo = healthy_hylo_min();
    let oracle = Pubkey::new_unique();
    hylo.sol_usd_oracle = oracle;

    let a = execute_order_s2l_lst(executor, owner, order, &hylo);

    assert_eq!(a.executor, executor);
    assert_eq!(a.owner, owner);
    assert_eq!(a.hylo, pda::HYLO);
    assert_eq!(a.sol_usd_pyth_feed, oracle);
    assert_eq!(a.hyusd_mint, HYUSD::MINT);
    assert_eq!(a.stablecoin_auth, pda::HYUSD_AUTH);
    assert_eq!(a.fee_auth, pda::fee_auth(HYUSD::MINT));
    assert_eq!(a.fee_vault, pda::fee_vault(HYUSD::MINT));
    assert_eq!(a.xsol_mint, XSOL::MINT);
    assert_eq!(a.levercoin_auth, pda::XSOL_AUTH);
    assert_eq!(a.order, order);
    assert_eq!(a.order_hyusd_vault, pda::ata(order, HYUSD::MINT));
    assert_eq!(a.order_xsol_vault, pda::ata(order, XSOL::MINT));
    assert_eq!(a.owner_xsol_ta, pda::ata(owner, XSOL::MINT));
    assert_eq!(
      a.hylo_exchange_event_authority,
      pda::EXCHANGE_EVENT_AUTHORITY
    );
    assert_eq!(a.hylo_exchange_program, crate::exchange::ID);
    assert_eq!(a.event_authority, pda::trigger_orders_event_authority());
    assert_eq!(a.program, trigger_orders::ID);
    assert_eq!(a.token_program, token::ID);
    assert_eq!(a.associated_token_program, associated_token::ID);
    assert_eq!(a.system_program, system_program::ID);
  }

  #[test]
  fn execute_order_l2s_lst_threads_hylo_oracle() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let mut hylo = healthy_hylo_min();
    let oracle = Pubkey::new_unique();
    hylo.sol_usd_oracle = oracle;

    let a = execute_order_l2s_lst(executor, owner, order, &hylo);

    assert_eq!(a.executor, executor);
    assert_eq!(a.owner, owner);
    assert_eq!(a.hylo, pda::HYLO);
    assert_eq!(a.sol_usd_pyth_feed, oracle);
    assert_eq!(a.hyusd_mint, HYUSD::MINT);
    assert_eq!(a.stablecoin_auth, pda::HYUSD_AUTH);
    assert_eq!(a.fee_auth, pda::fee_auth(HYUSD::MINT));
    assert_eq!(a.fee_vault, pda::fee_vault(HYUSD::MINT));
    assert_eq!(a.xsol_mint, XSOL::MINT);
    assert_eq!(a.levercoin_auth, pda::XSOL_AUTH);
    assert_eq!(a.order, order);
    assert_eq!(a.order_xsol_vault, pda::ata(order, XSOL::MINT));
    assert_eq!(a.order_hyusd_vault, pda::ata(order, HYUSD::MINT));
    assert_eq!(a.owner_hyusd_ta, pda::ata(owner, HYUSD::MINT));
    assert_eq!(
      a.hylo_exchange_event_authority,
      pda::EXCHANGE_EVENT_AUTHORITY
    );
    assert_eq!(a.hylo_exchange_program, crate::exchange::ID);
    assert_eq!(a.event_authority, pda::trigger_orders_event_authority());
    assert_eq!(a.program, trigger_orders::ID);
    assert_eq!(a.token_program, token::ID);
    assert_eq!(a.associated_token_program, associated_token::ID);
    assert_eq!(a.system_program, system_program::ID);
  }

  #[test]
  fn execute_order_s2l_exo_threads_exo_oracle() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let mut exo_pair = healthy_exo_pair_min();
    let oracle = Pubkey::new_unique();
    exo_pair.oracle = oracle;
    let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
    let vault_auth = pda::exo_vault_auth(collateral_mint);

    let a = execute_order_s2l_exo(
      executor,
      owner,
      order,
      collateral_mint,
      &hylo,
      &exo_pair,
    );

    assert_eq!(a.executor, executor);
    assert_eq!(a.owner, owner);
    assert_eq!(a.hylo, pda::HYLO);
    assert_eq!(a.exo_pair, pda::exo_pair(collateral_mint));
    assert_eq!(a.collateral_mint, collateral_mint);
    assert_eq!(a.vault_auth, vault_auth);
    assert_eq!(a.collateral_vault, pda::ata(vault_auth, collateral_mint));
    assert_eq!(a.collateral_usd_pyth_feed, oracle);
    assert_eq!(a.hyusd_mint, HYUSD::MINT);
    assert_eq!(a.stablecoin_auth, pda::HYUSD_AUTH);
    assert_eq!(a.fee_auth, pda::fee_auth(HYUSD::MINT));
    assert_eq!(a.fee_vault, pda::fee_vault(HYUSD::MINT));
    assert_eq!(a.levercoin_mint, levercoin_mint);
    assert_eq!(a.levercoin_auth, pda::mint_auth(levercoin_mint));
    assert_eq!(a.order, order);
    assert_eq!(a.order_hyusd_vault, pda::ata(order, HYUSD::MINT));
    assert_eq!(a.order_levercoin_vault, pda::ata(order, levercoin_mint));
    assert_eq!(a.owner_levercoin_ta, pda::ata(owner, levercoin_mint));
    assert_eq!(
      a.hylo_exchange_event_authority,
      pda::EXCHANGE_EVENT_AUTHORITY
    );
    assert_eq!(a.hylo_exchange_program, crate::exchange::ID);
    assert_eq!(a.token_program, token::ID);
    assert_eq!(a.associated_token_program, associated_token::ID);
    assert_eq!(a.system_program, system_program::ID);
    assert_eq!(a.event_authority, pda::trigger_orders_event_authority());
    assert_eq!(a.program, trigger_orders::ID);
  }

  #[test]
  fn execute_order_l2s_exo_threads_exo_oracle() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let mut exo_pair = healthy_exo_pair_min();
    let oracle = Pubkey::new_unique();
    exo_pair.oracle = oracle;
    let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
    let vault_auth = pda::exo_vault_auth(collateral_mint);

    let a = execute_order_l2s_exo(
      executor,
      owner,
      order,
      collateral_mint,
      &hylo,
      &exo_pair,
    );

    assert_eq!(a.executor, executor);
    assert_eq!(a.owner, owner);
    assert_eq!(a.hylo, pda::HYLO);
    assert_eq!(a.exo_pair, pda::exo_pair(collateral_mint));
    assert_eq!(a.collateral_mint, collateral_mint);
    assert_eq!(a.vault_auth, vault_auth);
    assert_eq!(a.collateral_vault, pda::ata(vault_auth, collateral_mint));
    assert_eq!(a.collateral_usd_pyth_feed, oracle);
    assert_eq!(a.hyusd_mint, HYUSD::MINT);
    assert_eq!(a.stablecoin_auth, pda::HYUSD_AUTH);
    assert_eq!(a.fee_auth, pda::fee_auth(HYUSD::MINT));
    assert_eq!(a.fee_vault, pda::fee_vault(HYUSD::MINT));
    assert_eq!(a.levercoin_mint, levercoin_mint);
    assert_eq!(a.levercoin_auth, pda::mint_auth(levercoin_mint));
    assert_eq!(a.order, order);
    assert_eq!(a.order_levercoin_vault, pda::ata(order, levercoin_mint));
    assert_eq!(a.order_hyusd_vault, pda::ata(order, HYUSD::MINT));
    assert_eq!(a.owner_hyusd_ta, pda::ata(owner, HYUSD::MINT));
    assert_eq!(
      a.hylo_exchange_event_authority,
      pda::EXCHANGE_EVENT_AUTHORITY
    );
    assert_eq!(a.hylo_exchange_program, crate::exchange::ID);
    assert_eq!(a.token_program, token::ID);
    assert_eq!(a.associated_token_program, associated_token::ID);
    assert_eq!(a.system_program, system_program::ID);
    assert_eq!(a.event_authority, pda::trigger_orders_event_authority());
    assert_eq!(a.program, trigger_orders::ID);
  }

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
