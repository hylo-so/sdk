//! Instruction builders for Hylo Stability Pool.

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use spl_associated_token_account_interface::program::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token_interface::ID as TOKEN_PROGRAM_ID;

use crate::hylo_stability_pool::client::{accounts, args};
use crate::tokens::{HYUSD, SHYUSD, XSOL};
use crate::{hylo_exchange, hylo_stability_pool, pda, MPL_TOKEN_METADATA_ID};

#[must_use]
pub fn user_deposit(user: Pubkey, args: &args::UserDeposit) -> Instruction {
  let accounts = accounts::UserDeposit {
    user,
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD,
    levercoin_mint: XSOL,
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_lp_token_ta: pda::shyusd_ata(user),
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_pool: *pda::XSOL_POOL,
    lp_token_auth: *pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: TOKEN_PROGRAM_ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  };
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn user_withdraw(user: Pubkey, args: &args::UserWithdraw) -> Instruction {
  let accounts = accounts::UserWithdraw {
    user,
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD,
    user_stablecoin_ta: pda::hyusd_ata(user),
    fee_auth: pda::fee_auth(HYUSD),
    fee_vault: pda::fee_vault(HYUSD),
    user_lp_token_ta: pda::shyusd_ata(user),
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_mint: XSOL,
    levercoin_pool: *pda::XSOL_POOL,
    user_levercoin_ta: pda::xsol_ata(user),
    lp_token_auth: *pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
    hylo_exchange_program: hylo_exchange::ID,
    system_program: system_program::ID,
    token_program: TOKEN_PROGRAM_ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  };
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
    stablecoin_mint: HYUSD,
    stablecoin_pool: *pda::HYUSD_POOL,
    pool_auth: *pda::POOL_AUTH,
    levercoin_pool: *pda::XSOL_POOL,
    fee_auth: pda::fee_auth(HYUSD),
    fee_vault: pda::fee_vault(HYUSD),
    levercoin_mint: XSOL,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    stablecoin_auth: *pda::HYUSD_AUTH,
    levercoin_auth: *pda::XSOL_AUTH,
    hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
    hylo_exchange_program: hylo_exchange::ID,
    token_program: TOKEN_PROGRAM_ID,
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
    stablecoin_mint: HYUSD,
    stablecoin_pool: *pda::HYUSD_POOL,
    pool_auth: *pda::POOL_AUTH,
    levercoin_pool: *pda::XSOL_POOL,
    fee_auth: pda::fee_auth(HYUSD),
    fee_vault: pda::fee_vault(HYUSD),
    levercoin_mint: XSOL,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    stablecoin_auth: *pda::HYUSD_AUTH,
    levercoin_auth: *pda::XSOL_AUTH,
    hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
    hylo_exchange_program: hylo_exchange::ID,
    token_program: TOKEN_PROGRAM_ID,
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
    stablecoin_mint: HYUSD,
    levercoin_mint: XSOL,
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_pool: *pda::XSOL_POOL,
    lp_token_mint: SHYUSD,
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
    stablecoin_mint: HYUSD,
    levercoin_mint: XSOL,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    token_program: TOKEN_PROGRAM_ID,
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
    lp_token_mint: SHYUSD,
    lp_token_metadata: pda::metadata(SHYUSD),
    metadata_program: MPL_TOKEN_METADATA_ID,
    token_program: TOKEN_PROGRAM_ID,
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
