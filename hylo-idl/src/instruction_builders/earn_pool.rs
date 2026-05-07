//! Instruction builders for Hylo Earn Pool.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::sysvar::rent;
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::{associated_token, token};

use crate::earn_pool::account_builders;
use crate::earn_pool::client::{accounts, args};
use crate::earn_pool::types::TokenMetadata;
use crate::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use crate::{earn_pool, pda};

#[must_use]
pub fn user_deposit(user: Pubkey, args: &args::UserDeposit) -> Instruction {
  let accounts = account_builders::deposit(user);
  Instruction {
    program_id: earn_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn user_withdraw(user: Pubkey, args: &args::UserWithdraw) -> Instruction {
  let accounts = account_builders::withdraw(user);
  Instruction {
    program_id: earn_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_earn_pool(
  admin: Pubkey,
  upgrade_authority: Pubkey,
) -> Instruction {
  let accounts = accounts::InitializeEarnPool {
    admin,
    upgrade_authority,
    pool_config: pda::POOL_CONFIG,
    hylo: pda::HYLO,
    pool_auth: pda::POOL_AUTH,
    stablecoin_pool: pda::HYUSD_POOL,
    levercoin_pool: pda::XSOL_POOL,
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint: XSOL::MINT,
    associated_token_program: associated_token::ID,
    token_program: token::ID,
    system_program: system_program::ID,
    program_data: pda::EARN_POOL_PROGRAM_DATA,
    hylo_earn_pool: earn_pool::ID,
  };
  let args = args::InitializeEarnPool {};
  Instruction {
    program_id: earn_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_lp_token_mint(
  admin: Pubkey,
  lp_token_metadata: TokenMetadata,
) -> Instruction {
  let accounts = accounts::InitializeLpTokenMint {
    admin,
    pool_config: pda::POOL_CONFIG,
    hylo: pda::HYLO,
    lp_token_auth: pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD::MINT,
    lp_token_metadata: pda::metadata(SHYUSD::MINT),
    metadata_program: mpl_token_metadata::ID,
    token_program: token::ID,
    rent: rent::ID,
    system_program: system_program::ID,
  };
  let args = args::InitializeLpTokenMint { lp_token_metadata };
  Instruction {
    program_id: earn_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn absorb_rebalance_loss(args: &args::AbsorbRebalanceLoss) -> Instruction {
  let accounts = accounts::AbsorbRebalanceLoss {
    rebalance_auth: pda::REBALANCE_AUTH,
    hylo: pda::HYLO,
    pool_config: pda::POOL_CONFIG,
    stablecoin_pool_auth: pda::POOL_AUTH,
    stablecoin_pool: pda::HYUSD_POOL,
    stablecoin_mint: HYUSD::MINT,
    token_program: token::ID,
    event_authority: pda::EARN_POOL_EVENT_AUTHORITY,
    program: earn_pool::ID,
  };
  Instruction {
    program_id: earn_pool::ID,
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
    pool_config: pda::POOL_CONFIG,
    hylo: pda::HYLO,
    event_authority: pda::EARN_POOL_EVENT_AUTHORITY,
    program: earn_pool::ID,
  };
  Instruction {
    program_id: earn_pool::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}
