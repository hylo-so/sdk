use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};
use hylo_core::pyth::SOL_USD_PYTH_FEED;
use hylo_idl::hylo_exchange::client::accounts::{
  MintLevercoin, MintStablecoin, RedeemLevercoin, RedeemStablecoin,
  SwapLeverToStable, SwapStableToLever,
};
use hylo_idl::hylo_stability_pool::client::accounts::{
  UserDeposit, UserWithdraw,
};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use hylo_idl::{ata, hylo_exchange, hylo_stability_pool, pda};

/// Builds account context for stablecoin mint (LST -> hyUSD).
#[must_use]
pub fn mint_stablecoin(user: Pubkey, lst_mint: Pubkey) -> MintStablecoin {
  MintStablecoin {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_lst_ata: ata!(user, lst_mint),
    user_stablecoin_ata: pda::hyusd_ata(user),
    lst_mint,
    stablecoin_mint: HYUSD::MINT,
    sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  }
}

/// Builds account context for levercoin mint (LST -> xSOL).
#[must_use]
pub fn mint_levercoin(user: Pubkey, lst_mint: Pubkey) -> MintLevercoin {
  MintLevercoin {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    levercoin_auth: *pda::XSOL_AUTH,
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_lst_ata: ata!(user, lst_mint),
    user_levercoin_ata: pda::xsol_ata(user),
    lst_mint,
    levercoin_mint: XSOL::MINT,
    stablecoin_mint: HYUSD::MINT,
    sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  }
}

/// Builds account context for stablecoin redemption (hyUSD -> LST).
#[must_use]
pub fn redeem_stablecoin(user: Pubkey, lst_mint: Pubkey) -> RedeemStablecoin {
  RedeemStablecoin {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_stablecoin_ata: pda::hyusd_ata(user),
    user_lst_ata: ata!(user, lst_mint),
    stablecoin_mint: HYUSD::MINT,
    lst_mint,
    sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  }
}

/// Builds account context for levercoin redemption (xSOL -> LST).
#[must_use]
pub fn redeem_levercoin(user: Pubkey, lst_mint: Pubkey) -> RedeemLevercoin {
  RedeemLevercoin {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    levercoin_auth: *pda::XSOL_AUTH,
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_levercoin_ata: pda::xsol_ata(user),
    user_lst_ata: ata!(user, lst_mint),
    levercoin_mint: XSOL::MINT,
    stablecoin_mint: HYUSD::MINT,
    lst_mint,
    sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  }
}

/// Builds account context for stable-to-lever swap (hyUSD -> xSOL).
#[must_use]
pub fn swap_stable_to_lever(user: Pubkey) -> SwapStableToLever {
  SwapStableToLever {
    user,
    hylo: *pda::HYLO,
    sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    stablecoin_mint: HYUSD::MINT,
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_stablecoin_ata: pda::hyusd_ata(user),
    levercoin_mint: XSOL::MINT,
    levercoin_auth: *pda::XSOL_AUTH,
    user_levercoin_ata: pda::xsol_ata(user),
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  }
}

/// Builds account context for lever-to-stable swap (xSOL -> hyUSD).
#[must_use]
pub fn swap_lever_to_stable(user: Pubkey) -> SwapLeverToStable {
  SwapLeverToStable {
    user,
    hylo: *pda::HYLO,
    sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    stablecoin_mint: HYUSD::MINT,
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_stablecoin_ata: pda::hyusd_ata(user),
    levercoin_mint: XSOL::MINT,
    levercoin_auth: *pda::XSOL_AUTH,
    user_levercoin_ata: pda::xsol_ata(user),
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  }
}

/// Builds account context for stability pool deposit (hyUSD -> sHYUSD).
#[must_use]
pub fn stability_pool_deposit(user: Pubkey) -> UserDeposit {
  UserDeposit {
    user,
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint: XSOL::MINT,
    user_stablecoin_ata: pda::hyusd_ata(user),
    user_lp_token_ata: pda::shyusd_ata(user),
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_pool: *pda::XSOL_POOL,
    lp_token_auth: *pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD::MINT,
    sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
    hylo_exchange_program: hylo_exchange::ID,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  }
}

/// Builds account context for stability pool withdrawal (sHYUSD -> hyUSD).
#[must_use]
pub fn stability_pool_withdraw(user: Pubkey) -> UserWithdraw {
  UserWithdraw {
    user,
    pool_config: *pda::POOL_CONFIG,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    user_stablecoin_ata: pda::hyusd_ata(user),
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_lp_token_ata: pda::shyusd_ata(user),
    pool_auth: *pda::POOL_AUTH,
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_mint: XSOL::MINT,
    levercoin_pool: *pda::XSOL_POOL,
    user_levercoin_ata: pda::xsol_ata(user),
    lp_token_auth: *pda::SHYUSD_AUTH,
    lp_token_mint: SHYUSD::MINT,
    sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
    hylo_exchange_program: hylo_exchange::ID,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
    program: hylo_stability_pool::ID,
  }
}
