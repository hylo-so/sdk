use anchor_lang::prelude::Pubkey;
use anchor_spl::token;

use crate::earn_pool::client::accounts::{
  DeprecateLevercoinPool, UserDeposit, UserWithdraw,
};
use crate::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use crate::{earn_pool, pda};

/// Builds account context for earn pool deposit (hyUSD -> sHYUSD).
#[must_use]
pub fn deposit(user: Pubkey) -> UserDeposit {
  UserDeposit {
    user,
    pool_config: pda::POOL_CONFIG,
    hylo: pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_lp_token_ta: pda::shyusd_ata(user),
    pool_auth: pda::POOL_AUTH,
    stablecoin_pool: pda::HYUSD_POOL,
    lp_token_auth: pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD::MINT,
    token_program: token::ID,
    event_authority: pda::EARN_POOL_EVENT_AUTHORITY,
    program: earn_pool::ID,
  }
}

/// Builds account context for earn pool withdrawal (sHYUSD -> hyUSD).
#[must_use]
pub fn withdraw(user: Pubkey) -> UserWithdraw {
  UserWithdraw {
    user,
    pool_config: pda::POOL_CONFIG,
    hylo: pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    user_stablecoin_ta: pda::hyusd_ata(user),
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_lp_token_ta: pda::shyusd_ata(user),
    pool_auth: pda::POOL_AUTH,
    stablecoin_pool: pda::HYUSD_POOL,
    lp_token_mint: SHYUSD::MINT,
    token_program: token::ID,
    event_authority: pda::EARN_POOL_EVENT_AUTHORITY,
    program: earn_pool::ID,
  }
}

#[must_use]
pub fn deprecate_levercoin_pool(admin: Pubkey) -> DeprecateLevercoinPool {
  DeprecateLevercoinPool {
    admin,
    hylo: pda::HYLO,
    pool_config: pda::POOL_CONFIG,
    pool_auth: pda::POOL_AUTH,
    levercoin_pool: pda::XSOL_POOL,
    admin_levercoin_ta: pda::xsol_ata(admin),
    levercoin_mint: XSOL::MINT,
    token_program: token::ID,
  }
}
