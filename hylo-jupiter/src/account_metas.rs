//! Account meta builders for Jupiter AMM swap instructions.
//!
//! Each function builds account metas for a router `Route`
//! instruction wrapping the appropriate exchange or earn pool
//! accounts.

use anchor_lang::prelude::{Pubkey, ToAccountMetas};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, USDC, XSOL};
use hylo_idl::{earn_pool, exchange, pda};
use hylo_jupiter_amm_interface::{Swap, SwapAndAccountMetas};

fn route_account_metas<A: ToAccountMetas>(
  in_token: Pubkey,
  out_token: Pubkey,
  inner_accounts: &A,
) -> SwapAndAccountMetas {
  let account_metas = inner_accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token,
      out_token,
    },
    account_metas,
  }
}

/// Mint hyUSD from LST.
#[must_use]
pub fn mint_stablecoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
) -> SwapAndAccountMetas {
  let accounts =
    exchange::account_builders::mint_stablecoin_lst(user, lst_mint);
  route_account_metas(lst_mint, HYUSD::MINT, &accounts)
}

/// Mint xSOL from LST.
#[must_use]
pub fn mint_levercoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::mint_levercoin_lst(user, lst_mint);
  route_account_metas(lst_mint, XSOL::MINT, &accounts)
}

/// Redeem hyUSD for LST.
#[must_use]
pub fn redeem_stablecoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
) -> SwapAndAccountMetas {
  let accounts =
    exchange::account_builders::redeem_stablecoin_lst(user, lst_mint);
  route_account_metas(HYUSD::MINT, lst_mint, &accounts)
}

/// Redeem xSOL for LST.
#[must_use]
pub fn redeem_levercoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
) -> SwapAndAccountMetas {
  let accounts =
    exchange::account_builders::redeem_levercoin_lst(user, lst_mint);
  route_account_metas(XSOL::MINT, lst_mint, &accounts)
}

/// Convert hyUSD to xSOL.
#[must_use]
pub fn convert_stable_to_lever_lst(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::convert_stable_to_lever_lst(user);
  route_account_metas(HYUSD::MINT, XSOL::MINT, &accounts)
}

/// Convert xSOL to hyUSD.
#[must_use]
pub fn convert_lever_to_stable_lst(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::convert_lever_to_stable_lst(user);
  route_account_metas(XSOL::MINT, HYUSD::MINT, &accounts)
}

/// Deposit hyUSD to earn pool.
#[must_use]
pub fn earn_pool_deposit(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = earn_pool::account_builders::deposit(user);
  route_account_metas(HYUSD::MINT, SHYUSD::MINT, &accounts)
}

/// Withdraw hyUSD from earn pool.
#[must_use]
pub fn earn_pool_withdraw(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = earn_pool::account_builders::withdraw(user);
  route_account_metas(SHYUSD::MINT, HYUSD::MINT, &accounts)
}

/// Swap one LST for another LST.
#[must_use]
pub fn swap_lst_to_lst(
  user: Pubkey,
  in_lst: Pubkey,
  out_lst: Pubkey,
) -> SwapAndAccountMetas {
  let accounts =
    exchange::account_builders::swap_lst_to_lst(user, in_lst, out_lst);
  route_account_metas(in_lst, out_lst, &accounts)
}

/// Swap LST for USDC.
#[must_use]
pub fn swap_lst_to_usdc(
  user: Pubkey,
  lst_mint: Pubkey,
  pool_state: Pubkey,
) -> SwapAndAccountMetas {
  let accounts =
    exchange::account_builders::swap_lst_to_usdc(user, lst_mint, pool_state);
  route_account_metas(lst_mint, USDC::MINT, &accounts)
}

/// Swap USDC for LST.
#[must_use]
pub fn swap_usdc_to_lst(
  user: Pubkey,
  lst_mint: Pubkey,
  pool_state: Pubkey,
) -> SwapAndAccountMetas {
  let accounts =
    exchange::account_builders::swap_usdc_to_lst(user, lst_mint, pool_state);
  route_account_metas(USDC::MINT, lst_mint, &accounts)
}

/// Mint hyUSD from USDC.
#[must_use]
pub fn mint_stablecoin_usdc(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::mint_stablecoin_usdc(user);
  route_account_metas(USDC::MINT, HYUSD::MINT, &accounts)
}

/// Redeem hyUSD for USDC.
#[must_use]
pub fn redeem_stablecoin_usdc(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::redeem_stablecoin_usdc(user);
  route_account_metas(HYUSD::MINT, USDC::MINT, &accounts)
}

/// Swap exo collateral for USDC.
#[must_use]
pub fn swap_exo_to_usdc(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::swap_exo_to_usdc(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  route_account_metas(collateral_mint, USDC::MINT, &accounts)
}

/// Swap USDC for exo collateral.
#[must_use]
pub fn swap_usdc_to_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::swap_usdc_to_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  route_account_metas(USDC::MINT, collateral_mint, &accounts)
}

/// Mint hyUSD from exo collateral.
#[must_use]
pub fn mint_stablecoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::mint_stablecoin_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  route_account_metas(collateral_mint, HYUSD::MINT, &accounts)
}

/// Redeem hyUSD for exo collateral.
#[must_use]
pub fn redeem_stablecoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::redeem_stablecoin_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  route_account_metas(HYUSD::MINT, collateral_mint, &accounts)
}

/// Mint exo levercoin from exo collateral.
#[must_use]
pub fn mint_levercoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::mint_levercoin_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  route_account_metas(
    collateral_mint,
    pda::exo_levercoin_mint(collateral_mint),
    &accounts,
  )
}

/// Redeem exo levercoin for exo collateral.
#[must_use]
pub fn redeem_levercoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::redeem_levercoin_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  route_account_metas(
    pda::exo_levercoin_mint(collateral_mint),
    collateral_mint,
    &accounts,
  )
}

/// Convert hyUSD to exo levercoin.
#[must_use]
pub fn convert_stable_to_lever_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::convert_stable_to_lever_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  route_account_metas(
    HYUSD::MINT,
    pda::exo_levercoin_mint(collateral_mint),
    &accounts,
  )
}

/// Convert exo levercoin to hyUSD.
#[must_use]
pub fn convert_lever_to_stable_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = exchange::account_builders::convert_lever_to_stable_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  route_account_metas(
    pda::exo_levercoin_mint(collateral_mint),
    HYUSD::MINT,
    &accounts,
  )
}
