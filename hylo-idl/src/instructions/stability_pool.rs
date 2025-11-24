//! Instruction builders for Hylo Stability Pool.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::{associated_token, token};

use crate::accounts::stability_pool;
use crate::hylo_stability_pool::client::{accounts, args};
use crate::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use crate::{hylo_exchange, hylo_stability_pool, pda};

#[must_use]
pub fn user_deposit(user: Pubkey, args: &args::UserDeposit) -> Instruction {
  let accounts = stability_pool::deposit(user);
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn user_withdraw(user: Pubkey, args: &args::UserWithdraw) -> Instruction {
  let accounts = stability_pool::withdraw(user);
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn rebalance_stable_to_lever(payer: Pubkey) -> Instruction {
  let accounts = accounts::RebalanceStableToLever {
    payer,
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    stablecoin_pool: *pda::HYUSD_POOL,
    pool_auth: *pda::POOL_AUTH,
    levercoin_pool: *pda::XSOL_POOL,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    levercoin_mint: XSOL::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    stablecoin_auth: *pda::HYUSD_AUTH,
    levercoin_auth: *pda::XSOL_AUTH,
    hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
    hylo_exchange_program: hylo_exchange::ID,
    token_program: token::ID,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  };
  let instruction_args = args::RebalanceStableToLever {};
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn rebalance_lever_to_stable(payer: Pubkey) -> Instruction {
  let accounts = accounts::RebalanceLeverToStable {
    payer,
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    stablecoin_pool: *pda::HYUSD_POOL,
    pool_auth: *pda::POOL_AUTH,
    levercoin_pool: *pda::XSOL_POOL,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    levercoin_mint: XSOL::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    stablecoin_auth: *pda::HYUSD_AUTH,
    levercoin_auth: *pda::XSOL_AUTH,
    hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
    hylo_exchange_program: hylo_exchange::ID,
    token_program: token::ID,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  };
  let instruction_args = args::RebalanceLeverToStable {};
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn get_stats() -> Instruction {
  let accounts = accounts::GetStats {
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint: XSOL::MINT,
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_pool: *pda::XSOL_POOL,
    lp_token_mint: SHYUSD::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
  };
  let instruction_args = args::GetStats {};
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn initialize_stability_pool(
  admin: Pubkey,
  upgrade_authority: Pubkey,
) -> Instruction {
  let accounts = accounts::InitializeStabilityPool {
    admin,
    upgrade_authority,
    pool_config: *pda::POOL_CONFIG,
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_pool: *pda::XSOL_POOL,
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint: XSOL::MINT,
    associated_token_program: associated_token::ID,
    token_program: token::ID,
    system_program: system_program::ID,
    program_data: *pda::STABILITY_POOL_PROGRAM_DATA,
    hylo_stability_pool: hylo_stability_pool::ID,
  };
  let args = args::InitializeStabilityPool {};
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_lp_token_mint(admin: Pubkey) -> Instruction {
  let accounts = accounts::InitializeLpTokenMint {
    admin,
    pool_config: *pda::POOL_CONFIG,
    lp_token_auth: *pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD::MINT,
    lp_token_metadata: pda::metadata(SHYUSD::MINT),
    metadata_program: mpl_token_metadata::ID,
    token_program: token::ID,
    system_program: system_program::ID,
  };
  let args = args::InitializeLpTokenMint {};
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_withdrawal_fee(
  admin: Pubkey,
  args: &args::UpdateWithdrawalFee,
) -> Instruction {
  let accounts = accounts::UpdateWithdrawalFee {
    admin,
    pool_config: *pda::POOL_CONFIG,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  };
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}
