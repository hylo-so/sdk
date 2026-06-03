//! Instruction builders for `hylo-trigger-orders`.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};

use crate::exchange::accounts::{ExoPair, Hylo};
use crate::trigger_orders;
use crate::trigger_orders::account_builders;
use crate::trigger_orders::client::args;

#[must_use]
pub fn create_order_stable_to_lever_lst(
  owner: Pubkey,
  order: Pubkey,
  args: &args::CreateOrderStableToLeverLst,
) -> Instruction {
  let accounts =
    account_builders::create_order_stable_to_lever_lst(owner, order);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn create_order_stable_to_lever_exo(
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  args: &args::CreateOrderStableToLeverExo,
) -> Instruction {
  let accounts = account_builders::create_order_stable_to_lever_exo(
    owner,
    order,
    collateral_mint,
  );
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn create_order_lever_to_stable_lst(
  owner: Pubkey,
  order: Pubkey,
  args: &args::CreateOrderLeverToStableLst,
) -> Instruction {
  let accounts =
    account_builders::create_order_lever_to_stable_lst(owner, order);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn create_order_lever_to_stable_exo(
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  args: &args::CreateOrderLeverToStableExo,
) -> Instruction {
  let accounts = account_builders::create_order_lever_to_stable_exo(
    owner,
    order,
    collateral_mint,
  );
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn execute_order_stable_to_lever_lst(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  hylo: &Hylo,
) -> Instruction {
  let accounts = account_builders::execute_order_stable_to_lever_lst(
    executor, owner, order, hylo,
  );
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args::ExecuteOrderStableToLeverLst {}.data(),
  }
}

#[must_use]
pub fn execute_order_lever_to_stable_lst(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  hylo: &Hylo,
) -> Instruction {
  let accounts = account_builders::execute_order_lever_to_stable_lst(
    executor, owner, order, hylo,
  );
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args::ExecuteOrderLeverToStableLst {}.data(),
  }
}

#[must_use]
pub fn execute_order_stable_to_lever_exo(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  hylo: &Hylo,
  exo_pair: &ExoPair,
) -> Instruction {
  let accounts = account_builders::execute_order_stable_to_lever_exo(
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
    data: args::ExecuteOrderStableToLeverExo {}.data(),
  }
}

#[must_use]
pub fn execute_order_lever_to_stable_exo(
  executor: Pubkey,
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  hylo: &Hylo,
  exo_pair: &ExoPair,
) -> Instruction {
  let accounts = account_builders::execute_order_lever_to_stable_exo(
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
    data: args::ExecuteOrderLeverToStableExo {}.data(),
  }
}

#[must_use]
pub fn cancel_order_stable_to_lever(
  owner: Pubkey,
  order: Pubkey,
  args: &args::CancelOrderStableToLever,
) -> Instruction {
  let accounts = account_builders::cancel_order_stable_to_lever(owner, order);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn cancel_order_lever_to_stable_lst(
  owner: Pubkey,
  order: Pubkey,
  args: &args::CancelOrderLeverToStableLst,
) -> Instruction {
  let accounts =
    account_builders::cancel_order_lever_to_stable_lst(owner, order);
  Instruction {
    program_id: trigger_orders::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn cancel_order_lever_to_stable_exo(
  owner: Pubkey,
  order: Pubkey,
  collateral_mint: Pubkey,
  args: &args::CancelOrderLeverToStableExo,
) -> Instruction {
  let accounts = account_builders::cancel_order_lever_to_stable_exo(
    owner,
    order,
    collateral_mint,
  );
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

  fn create_args_lever_to_stable_lst() -> args::CreateOrderLeverToStableLst {
    args::CreateOrderLeverToStableLst {
      nonce: 42,
      escrow_amount: 1000,
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
    }
  }

  fn create_args_lever_to_stable_exo() -> args::CreateOrderLeverToStableExo {
    args::CreateOrderLeverToStableExo {
      nonce: 42,
      escrow_amount: 1000,
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
    }
  }

  fn create_args_stable_to_lever_exo() -> args::CreateOrderStableToLeverExo {
    args::CreateOrderStableToLeverExo {
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
  fn create_order_stable_to_lever_lst_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let args = args::CreateOrderStableToLeverLst {
      nonce: 42,
      escrow_amount: 1000,
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
    };
    let ix = create_order_stable_to_lever_lst(owner, order, &args);
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
  fn create_order_stable_to_lever_exo_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let ix = create_order_stable_to_lever_exo(
      owner,
      order,
      collateral_mint,
      &create_args_stable_to_lever_exo(),
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 37);
    assert_eq!(ix.accounts.len(), 12);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn create_order_lever_to_stable_lst_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let ix = create_order_lever_to_stable_lst(
      owner,
      order,
      &create_args_lever_to_stable_lst(),
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 37);
    assert_eq!(ix.accounts.len(), 11);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn create_order_lever_to_stable_exo_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let ix = create_order_lever_to_stable_exo(
      owner,
      order,
      collateral_mint,
      &create_args_lever_to_stable_exo(),
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 37);
    assert_eq!(ix.accounts.len(), 12);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn execute_order_stable_to_lever_lst_returns_well_formed_instruction() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let ix = execute_order_stable_to_lever_lst(executor, owner, order, &hylo);
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 21);
    assert_eq!(ix.accounts[0].pubkey, executor);
  }

  #[test]
  fn execute_order_lever_to_stable_lst_returns_well_formed_instruction() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let ix = execute_order_lever_to_stable_lst(executor, owner, order, &hylo);
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 21);
    assert_eq!(ix.accounts[0].pubkey, executor);
  }

  #[test]
  fn execute_order_stable_to_lever_exo_returns_well_formed_instruction() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let exo_pair = healthy_exo_pair_min();
    let ix = execute_order_stable_to_lever_exo(
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
  fn execute_order_lever_to_stable_exo_returns_well_formed_instruction() {
    let executor = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let hylo = healthy_hylo_min();
    let exo_pair = healthy_exo_pair_min();
    let ix = execute_order_lever_to_stable_exo(
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
  fn cancel_order_stable_to_lever_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let ix = cancel_order_stable_to_lever(
      owner,
      order,
      &args::CancelOrderStableToLever {},
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 8);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn cancel_order_lever_to_stable_lst_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let ix = cancel_order_lever_to_stable_lst(
      owner,
      order,
      &args::CancelOrderLeverToStableLst {},
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 8);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }

  #[test]
  fn cancel_order_lever_to_stable_exo_returns_well_formed_instruction() {
    let owner = Pubkey::new_unique();
    let order = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let ix = cancel_order_lever_to_stable_exo(
      owner,
      order,
      collateral_mint,
      &args::CancelOrderLeverToStableExo {},
    );
    assert_eq!(ix.program_id, trigger_orders::ID);
    assert_eq!(ix.data.len(), 8);
    assert_eq!(ix.accounts.len(), 9);
    assert_eq!(ix.accounts[0].pubkey, owner);
    assert_eq!(ix.accounts[1].pubkey, order);
  }
}
