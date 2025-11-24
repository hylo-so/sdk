use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};

use crate::hylo_stability_pool::client::accounts::{UserDeposit, UserWithdraw};
use crate::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use crate::{hylo_exchange, hylo_stability_pool, pda};

/// Builds account context for stability pool deposit (hyUSD -> sHYUSD).
#[must_use]
pub fn deposit(user: Pubkey) -> UserDeposit {
  UserDeposit {
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
  }
}

/// Builds account context for stability pool withdrawal (sHYUSD -> hyUSD).
#[must_use]
pub fn withdraw(user: Pubkey) -> UserWithdraw {
  UserWithdraw {
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
  }
}
