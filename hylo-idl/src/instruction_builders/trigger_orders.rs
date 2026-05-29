//! Instruction builders for `hylo-trigger-orders`.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};

use crate::exchange::accounts::{ExoPair, Hylo};
use crate::trigger_orders;
use crate::trigger_orders::account_builders;
use crate::trigger_orders::client::args;

#[must_use]
pub fn create_order_s2l_lst(
  owner: Pubkey,
  order: Pubkey,
  args: &args::CreateOrderS2lLst,
) -> Instruction {
  let accounts = account_builders::create_order_s2l_lst(owner, order);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn create_order_s2l_exo(
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  args: &args::CreateOrderS2lExo,
) -> Instruction {
  let accounts =
    account_builders::create_order_s2l_exo(owner, order, collateral_mint);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn create_order_l2s_lst(
  owner: Pubkey,
  order: Pubkey,
  args: &args::CreateOrderL2sLst,
) -> Instruction {
  let accounts = account_builders::create_order_l2s_lst(owner, order);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn create_order_l2s_exo(
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  args: &args::CreateOrderL2sExo,
) -> Instruction {
  let accounts =
    account_builders::create_order_l2s_exo(owner, order, collateral_mint);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn execute_order_s2l_lst(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  hylo: &Hylo,
) -> Instruction {
  let accounts =
    account_builders::execute_order_s2l_lst(executor, owner, order, hylo);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args::ExecuteOrderS2lLst {}.data(),
  }
}

#[must_use]
pub fn execute_order_l2s_lst(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  hylo: &Hylo,
) -> Instruction {
  let accounts =
    account_builders::execute_order_l2s_lst(executor, owner, order, hylo);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args::ExecuteOrderL2sLst {}.data(),
  }
}

#[must_use]
pub fn execute_order_s2l_exo(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  hylo: &Hylo,
  exo_pair: &ExoPair,
) -> Instruction {
  let accounts = account_builders::execute_order_s2l_exo(
    executor,
    owner,
    order,
    collateral_mint,
    hylo,
    exo_pair,
  );
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args::ExecuteOrderS2lExo {}.data(),
  }
}

#[must_use]
pub fn execute_order_l2s_exo(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  hylo: &Hylo,
  exo_pair: &ExoPair,
) -> Instruction {
  let accounts = account_builders::execute_order_l2s_exo(
    executor,
    owner,
    order,
    collateral_mint,
    hylo,
    exo_pair,
  );
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args::ExecuteOrderL2sExo {}.data(),
  }
}

#[must_use]
pub fn cancel_order_s2l(
  owner: Pubkey,
  order: Pubkey,
  args: &args::CancelOrderS2l,
) -> Instruction {
  let accounts = account_builders::cancel_order_s2l(owner, order);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn cancel_order_l2s_lst(
  owner: Pubkey,
  order: Pubkey,
  args: &args::CancelOrderL2sLst,
) -> Instruction {
  let accounts = account_builders::cancel_order_l2s_lst(owner, order);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn cancel_order_l2s_exo(
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  args: &args::CancelOrderL2sExo,
) -> Instruction {
  let accounts =
    account_builders::cancel_order_l2s_exo(owner, order, collateral_mint);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::exchange::types::{
    BorrowRateConfig, HarvestCache, LevercoinFees, PoolDrawdown,
    RebalanceCurveConfig, UFixValue64, VirtualStablecoin,
  };
  use crate::trigger_orders::types::TriggerDirection;

  fn create_args_l2s_lst() -> args::CreateOrderL2sLst {
    args::CreateOrderL2sLst {
      nonce: 42,
      escrow_amount: 1000,
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
    }
  }

  fn create_args_l2s_exo() -> args::CreateOrderL2sExo {
    args::CreateOrderL2sExo {
      nonce: 42,
      escrow_amount: 1000,
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
    }
  }

  fn create_args_s2l_exo() -> args::CreateOrderS2lExo {
    args::CreateOrderS2lExo {
      nonce: 42,
      escrow_amount: 1000,
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
    }
  }

  fn healthy_hylo_min() -> Hylo {
    Hylo::default()
  }

  // `ExoPair` does NOT derive `Default`, so list every field explicitly. The
  // execute builders only read `exo_pair.oracle`, set explicitly by callers.
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
  fn create_order_s2l_lst_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let args = args::CreateOrderS2lLst {
      nonce: 42,
      escrow_amount: 1000,
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
    };
    let ix = create_order_s2l_lst(owner, order, &args);
    assert_eq!(ix.program_id, trigger_orders::ID);
    // Anchor: 8-byte discriminator + borsh args.
    // 8 + 8(nonce u64) + 8(escrow u64) + 8(trigger_price i64) + 4(expo i32)
    //   + 1(direction) = 37.
    assert_eq!(ix.data.len(), 37);
    assert_eq!(ix.accounts.len(), 11);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert!(ix.accounts[0].is_signer);
    assert!(ix.accounts[0].is_writable);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn create_order_s2l_exo_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let ix = create_order_s2l_exo(
      owner,
      order,
      collateral_mint,
      &create_args_s2l_exo(),
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 37);
    assert_eq!(ix.accounts.len(), 12);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn create_order_l2s_lst_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let ix = create_order_l2s_lst(owner, order, &create_args_l2s_lst());
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 37);
    assert_eq!(ix.accounts.len(), 11);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn create_order_l2s_exo_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let ix = create_order_l2s_exo(
      owner,
      order,
      collateral_mint,
      &create_args_l2s_exo(),
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 37);
    assert_eq!(ix.accounts.len(), 12);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn execute_order_s2l_lst_returns_well_formed_instruction() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let ix = execute_order_s2l_lst(executor, owner, order, &hylo);
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 21);
    assert_eq!(ix.accounts[0].pubkey, executor);
  }

  #[test]
  fn execute_order_l2s_lst_returns_well_formed_instruction() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let ix = execute_order_l2s_lst(executor, owner, order, &hylo);
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 21);
    assert_eq!(ix.accounts[0].pubkey, executor);
  }

  #[test]
  fn execute_order_s2l_exo_returns_well_formed_instruction() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let exo_pair = healthy_exo_pair_min();
    let ix = execute_order_s2l_exo(
      executor,
      owner,
      order,
      collateral_mint,
      &hylo,
      &exo_pair,
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 25);
    assert_eq!(ix.accounts[0].pubkey, executor);
  }

  #[test]
  fn execute_order_l2s_exo_returns_well_formed_instruction() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let exo_pair = healthy_exo_pair_min();
    let ix = execute_order_l2s_exo(
      executor,
      owner,
      order,
      collateral_mint,
      &hylo,
      &exo_pair,
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 25);
    assert_eq!(ix.accounts[0].pubkey, executor);
  }

  #[test]
  fn cancel_order_s2l_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let ix = cancel_order_s2l(owner, order, &args::CancelOrderS2l {});
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 8);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn cancel_order_l2s_lst_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let ix = cancel_order_l2s_lst(owner, order, &args::CancelOrderL2sLst {});
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 8);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn cancel_order_l2s_exo_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let ix = cancel_order_l2s_exo(
      owner,
      order,
      collateral_mint,
      &args::CancelOrderL2sExo {},
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 9);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }
}
