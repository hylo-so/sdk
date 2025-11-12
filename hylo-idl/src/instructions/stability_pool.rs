//! Instruction builders for Hylo Stability Pool.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::{associated_token, token};

use crate::hylo_stability_pool::client::{accounts, args};
use crate::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use crate::{hylo_exchange, hylo_stability_pool, pda};

#[must_use]
pub fn user_deposit(amount_stablecoin: u64, user: Pubkey) -> Instruction {
  let accounts = accounts::UserDeposit {
    user,
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint: XSOL::MINT,
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_lp_token_ta: pda::shyusd_ata(user),
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_pool: *pda::XSOL_POOL,
    lp_token_auth: *pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  };
  let instruction_args = args::UserDeposit { amount_stablecoin };
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn user_withdraw(amount_lp_token: u64, user: Pubkey) -> Instruction {
  let accounts = accounts::UserWithdraw {
    user,
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    user_stablecoin_ta: pda::hyusd_ata(user),
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_lp_token_ta: pda::shyusd_ata(user),
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_mint: XSOL::MINT,
    levercoin_pool: *pda::XSOL_POOL,
    user_levercoin_ta: pda::xsol_ata(user),
    lp_token_auth: *pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
    hylo_exchange_program: hylo_exchange::ID,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  };
  let instruction_args = args::UserWithdraw { amount_lp_token };
  Instruction {
    program_id: hylo_stability_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
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
