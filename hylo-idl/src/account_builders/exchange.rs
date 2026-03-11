use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};

use crate::exchange::client::accounts::{
  ConvertLeverToStableExo, ConvertLeverToStableLst, ConvertStableToLeverExo,
  ConvertStableToLeverLst, HarvestFundingRate, InitializeUsdc,
  MintLevercoinExo, MintLevercoinLst, MintStablecoinExo, MintStablecoinLst,
  MintStablecoinUsdc, RedeemLevercoinExo, RedeemLevercoinLst,
  RedeemStablecoinExo, RedeemStablecoinLst, RedeemStablecoinUsdc, RegisterExo,
  SwapExoToUsdc, SwapLstToLst, SwapLstToUsdc, SwapUsdcToExo, SwapUsdcToLst,
  WithdrawFees,
};
use crate::tokens::{TokenMint, HYUSD, USDC, XSOL};
use crate::{ata, exchange, pda, stability_pool};

/// Builds account context for stablecoin mint (LST -> hyUSD).
#[must_use]
pub fn mint_stablecoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
) -> MintStablecoinLst {
  MintStablecoinLst {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_lst_ta: ata!(user, lst_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    lst_mint,
    stablecoin_mint: HYUSD::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for levercoin mint (LST -> xSOL).
#[must_use]
pub fn mint_levercoin_lst(user: Pubkey, lst_mint: Pubkey) -> MintLevercoinLst {
  MintLevercoinLst {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    levercoin_auth: *pda::XSOL_AUTH,
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_lst_ta: ata!(user, lst_mint),
    user_levercoin_ta: pda::xsol_ata(user),
    lst_mint,
    levercoin_mint: XSOL::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for stablecoin redemption (hyUSD -> LST).
#[must_use]
pub fn redeem_stablecoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
) -> RedeemStablecoinLst {
  RedeemStablecoinLst {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_lst_ta: ata!(user, lst_mint),
    stablecoin_mint: HYUSD::MINT,
    lst_mint,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for levercoin redemption (xSOL -> LST).
#[must_use]
pub fn redeem_levercoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
) -> RedeemLevercoinLst {
  RedeemLevercoinLst {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_levercoin_ta: pda::xsol_ata(user),
    user_lst_ta: ata!(user, lst_mint),
    levercoin_mint: XSOL::MINT,
    lst_mint,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for stable-to-lever convert (hyUSD -> xSOL).
#[must_use]
pub fn convert_stable_to_lever_lst(user: Pubkey) -> ConvertStableToLeverLst {
  ConvertStableToLeverLst {
    user,
    hylo: *pda::HYLO,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    stablecoin_mint: HYUSD::MINT,
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_stablecoin_ta: pda::hyusd_ata(user),
    levercoin_mint: XSOL::MINT,
    levercoin_auth: *pda::XSOL_AUTH,
    user_levercoin_ta: pda::xsol_ata(user),
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for lever-to-stable convert (xSOL -> hyUSD).
#[must_use]
pub fn convert_lever_to_stable_lst(user: Pubkey) -> ConvertLeverToStableLst {
  ConvertLeverToStableLst {
    user,
    hylo: *pda::HYLO,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    stablecoin_mint: HYUSD::MINT,
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_stablecoin_ta: pda::hyusd_ata(user),
    levercoin_mint: XSOL::MINT,
    levercoin_auth: *pda::XSOL_AUTH,
    user_levercoin_ta: pda::xsol_ata(user),
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for registering an EXO pair.
#[must_use]
pub fn register_exo(
  admin: Pubkey,
  collateral_mint: Pubkey,
  exo_usd_pyth_feed: Pubkey,
) -> RegisterExo {
  let exo_pair = pda::exo_pair(collateral_mint);
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  let levercoin_auth = pda::mint_auth(levercoin_mint);
  let vault_auth = pda::vault_auth(collateral_mint);
  let collateral_vault = ata!(vault_auth, collateral_mint);
  let fee_auth = pda::fee_auth(collateral_mint);
  let fee_vault = ata!(fee_auth, collateral_mint);
  RegisterExo {
    admin,
    hylo: *pda::HYLO,
    collateral_mint,
    exo_pair,
    levercoin_auth,
    levercoin_mint,
    vault_auth,
    collateral_vault,
    fee_auth,
    fee_vault,
    levercoin_metadata: pda::metadata(levercoin_mint),
    exo_usd_pyth_feed,
    metadata_program: mpl_token_metadata::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Exo levercoin mint (collateral -> exo levercoin).
#[must_use]
pub fn mint_levercoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> MintLevercoinExo {
  let vault_auth = pda::vault_auth(collateral_mint);
  let fee_auth = pda::fee_auth(collateral_mint);
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  MintLevercoinExo {
    user,
    exo_pair: pda::exo_pair(collateral_mint),
    levercoin_auth: pda::mint_auth(levercoin_mint),
    vault_auth,
    fee_auth,
    collateral_vault: ata!(vault_auth, collateral_mint),
    fee_vault: ata!(fee_auth, collateral_mint),
    user_collateral_ta: ata!(user, collateral_mint),
    user_levercoin_ta: ata!(user, levercoin_mint),
    collateral_mint,
    levercoin_mint,
    collateral_usd_pyth_feed,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Exo stablecoin mint (collateral -> hyUSD).
#[must_use]
pub fn mint_stablecoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> MintStablecoinExo {
  let vault_auth = pda::vault_auth(collateral_mint);
  let fee_auth = pda::fee_auth(collateral_mint);
  MintStablecoinExo {
    user,
    hylo: *pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    stablecoin_auth: *pda::HYUSD_AUTH,
    vault_auth,
    fee_auth,
    collateral_vault: ata!(vault_auth, collateral_mint),
    fee_vault: ata!(fee_auth, collateral_mint),
    user_collateral_ta: ata!(user, collateral_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    collateral_mint,
    stablecoin_mint: HYUSD::MINT,
    collateral_usd_pyth_feed,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Exo levercoin redemption (exo levercoin -> collateral).
#[must_use]
pub fn redeem_levercoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> RedeemLevercoinExo {
  let vault_auth = pda::vault_auth(collateral_mint);
  let fee_auth = pda::fee_auth(collateral_mint);
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  RedeemLevercoinExo {
    user,
    exo_pair: pda::exo_pair(collateral_mint),
    vault_auth,
    fee_auth,
    collateral_vault: ata!(vault_auth, collateral_mint),
    fee_vault: ata!(fee_auth, collateral_mint),
    user_levercoin_ta: ata!(user, levercoin_mint),
    user_collateral_ta: ata!(user, collateral_mint),
    collateral_mint,
    levercoin_mint,
    collateral_usd_pyth_feed,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Exo stablecoin redemption (hyUSD -> collateral).
#[must_use]
pub fn redeem_stablecoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> RedeemStablecoinExo {
  let vault_auth = pda::vault_auth(collateral_mint);
  let fee_auth = pda::fee_auth(collateral_mint);
  RedeemStablecoinExo {
    user,
    hylo: *pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    vault_auth,
    fee_auth,
    collateral_vault: ata!(vault_auth, collateral_mint),
    fee_vault: ata!(fee_auth, collateral_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_collateral_ta: ata!(user, collateral_mint),
    collateral_mint,
    stablecoin_mint: HYUSD::MINT,
    collateral_usd_pyth_feed,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for harvesting exo funding rate.
#[must_use]
pub fn harvest_funding_rate(
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> HarvestFundingRate {
  let vault_auth = pda::vault_auth(collateral_mint);
  HarvestFundingRate {
    hylo: *pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    stablecoin_auth: *pda::HYUSD_AUTH,
    vault_auth,
    stablecoin_fee_auth: pda::fee_auth(HYUSD::MINT),
    pool_auth: *pda::POOL_AUTH,
    collateral_vault: ata!(vault_auth, collateral_mint),
    stablecoin_pool: *pda::HYUSD_POOL,
    stablecoin_fee_vault: pda::fee_vault(HYUSD::MINT),
    stablecoin_mint: HYUSD::MINT,
    collateral_mint,
    collateral_usd_pyth_feed,
    hylo_stability_pool: stability_pool::ID,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Lever-to-stable convert exo (xAsset -> hyUSD).
#[must_use]
pub fn convert_lever_to_stable_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> ConvertLeverToStableExo {
  let vault_auth = pda::vault_auth(collateral_mint);
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  ConvertLeverToStableExo {
    user,
    hylo: *pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    levercoin_auth: pda::mint_auth(levercoin_mint),
    stablecoin_auth: *pda::HYUSD_AUTH,
    vault_auth,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    collateral_vault: ata!(vault_auth, collateral_mint),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_levercoin_ta: ata!(user, levercoin_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint,
    collateral_mint,
    collateral_usd_pyth_feed,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Stable-to-lever convert exo (hyUSD -> xAsset).
#[must_use]
pub fn convert_stable_to_lever_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> ConvertStableToLeverExo {
  let vault_auth = pda::vault_auth(collateral_mint);
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  ConvertStableToLeverExo {
    user,
    hylo: *pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    levercoin_auth: pda::mint_auth(levercoin_mint),
    stablecoin_auth: *pda::HYUSD_AUTH,
    vault_auth,
    fee_auth: pda::fee_auth(HYUSD::MINT),
    collateral_vault: ata!(vault_auth, collateral_mint),
    fee_vault: pda::fee_vault(HYUSD::MINT),
    user_levercoin_ta: ata!(user, levercoin_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint,
    collateral_mint,
    collateral_usd_pyth_feed,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for withdrawing protocol fees.
#[must_use]
pub fn withdraw_fees(
  payer: Pubkey,
  treasury: Pubkey,
  fee_token_mint: Pubkey,
) -> WithdrawFees {
  let fee_auth = pda::fee_auth(fee_token_mint);
  WithdrawFees {
    payer,
    treasury,
    hylo: *pda::HYLO,
    fee_auth,
    fee_vault: ata!(fee_auth, fee_token_mint),
    treasury_ata: ata!(treasury, fee_token_mint),
    fee_token_mint,
    associated_token_program: associated_token::ID,
    token_program: token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for LST-to-LST swap.
#[must_use]
pub fn swap_lst_to_lst(
  user: Pubkey,
  lst_a: Pubkey,
  lst_b: Pubkey,
) -> SwapLstToLst {
  SwapLstToLst {
    user,
    hylo: *pda::HYLO,
    lst_a_mint: lst_a,
    lst_a_user_ta: ata!(user, lst_a),
    lst_a_vault_auth: pda::vault_auth(lst_a),
    lst_a_vault: pda::vault(lst_a),
    lst_a_header: pda::lst_header(lst_a),
    lst_b_mint: lst_b,
    lst_b_user_ta: ata!(user, lst_b),
    lst_b_vault_auth: pda::vault_auth(lst_b),
    lst_b_vault: pda::vault(lst_b),
    lst_b_header: pda::lst_header(lst_b),
    fee_auth: pda::fee_auth(lst_a),
    fee_vault: pda::fee_vault(lst_a),
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Exo collateral to USDC swap.
#[must_use]
pub fn swap_exo_to_usdc(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapExoToUsdc {
  let collateral_vault_auth = pda::vault_auth(collateral_mint);
  let usdc_vault_auth = pda::vault_auth(USDC::MINT);
  SwapExoToUsdc {
    user,
    exo_pair: pda::exo_pair(collateral_mint),
    usdc_pair: *pda::USDC_PAIR,
    collateral_vault_auth,
    usdc_vault_auth,
    collateral_vault: ata!(collateral_vault_auth, collateral_mint),
    usdc_collateral_vault: ata!(usdc_vault_auth, USDC::MINT),
    user_usdc_ta: pda::usdc_ata(user),
    user_collateral_ta: ata!(user, collateral_mint),
    usdc_mint: USDC::MINT,
    collateral_mint,
    levercoin_mint: pda::exo_levercoin_mint(collateral_mint),
    collateral_usd_pyth_feed,
    usdc_usd_pyth_feed: pda::USDC_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// USDC to exo collateral swap.
#[must_use]
pub fn swap_usdc_to_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapUsdcToExo {
  let collateral_vault_auth = pda::vault_auth(collateral_mint);
  let usdc_vault_auth = pda::vault_auth(USDC::MINT);
  SwapUsdcToExo {
    user,
    exo_pair: pda::exo_pair(collateral_mint),
    usdc_pair: *pda::USDC_PAIR,
    collateral_vault_auth,
    usdc_vault_auth,
    collateral_vault: ata!(collateral_vault_auth, collateral_mint),
    usdc_collateral_vault: ata!(usdc_vault_auth, USDC::MINT),
    user_usdc_ta: pda::usdc_ata(user),
    user_collateral_ta: ata!(user, collateral_mint),
    usdc_mint: USDC::MINT,
    collateral_mint,
    levercoin_mint: pda::exo_levercoin_mint(collateral_mint),
    collateral_usd_pyth_feed,
    usdc_usd_pyth_feed: pda::USDC_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// LST to USDC swap.
#[must_use]
pub fn swap_lst_to_usdc(user: Pubkey, lst_mint: Pubkey) -> SwapLstToUsdc {
  let usdc_vault_auth = pda::vault_auth(USDC::MINT);
  SwapLstToUsdc {
    user,
    hylo: *pda::HYLO,
    lst_header: pda::lst_header(lst_mint),
    usdc_pair: *pda::USDC_PAIR,
    lst_vault_auth: pda::vault_auth(lst_mint),
    usdc_vault_auth,
    lst_vault: pda::vault(lst_mint),
    usdc_vault: ata!(usdc_vault_auth, USDC::MINT),
    user_lst_ta: ata!(user, lst_mint),
    user_usdc_ta: pda::usdc_ata(user),
    lst_mint,
    usdc_mint: USDC::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    usdc_usd_pyth_feed: pda::USDC_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// USDC to LST swap.
#[must_use]
pub fn swap_usdc_to_lst(user: Pubkey, lst_mint: Pubkey) -> SwapUsdcToLst {
  let usdc_vault_auth = pda::vault_auth(USDC::MINT);
  SwapUsdcToLst {
    user,
    hylo: *pda::HYLO,
    lst_header: pda::lst_header(lst_mint),
    usdc_pair: *pda::USDC_PAIR,
    lst_vault_auth: pda::vault_auth(lst_mint),
    usdc_vault_auth,
    lst_vault: pda::vault(lst_mint),
    usdc_vault: ata!(usdc_vault_auth, USDC::MINT),
    user_lst_ta: ata!(user, lst_mint),
    user_usdc_ta: pda::usdc_ata(user),
    lst_mint,
    usdc_mint: USDC::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    usdc_usd_pyth_feed: pda::USDC_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for initializing the USDC pair.
#[must_use]
pub fn initialize_usdc(
  admin: Pubkey,
  usdc_usd_pyth_feed: Pubkey,
) -> InitializeUsdc {
  let usdc_vault_auth = pda::vault_auth(USDC::MINT);
  let usdc_fee_auth = pda::fee_auth(USDC::MINT);
  InitializeUsdc {
    admin,
    hylo: *pda::HYLO,
    usdc_pair: *pda::USDC_PAIR,
    usdc_vault_auth,
    usdc_fee_auth,
    usdc_collateral_vault: ata!(usdc_vault_auth, USDC::MINT),
    usdc_fee_vault: ata!(usdc_fee_auth, USDC::MINT),
    usdc_mint: USDC::MINT,
    usdc_usd_pyth_feed,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for hyUSD mint from USDC.
#[must_use]
pub fn mint_stablecoin_usdc(user: Pubkey) -> MintStablecoinUsdc {
  let usdc_vault_auth = pda::vault_auth(USDC::MINT);
  let usdc_fee_auth = pda::fee_auth(USDC::MINT);
  MintStablecoinUsdc {
    user,
    hylo: *pda::HYLO,
    usdc_pair: *pda::USDC_PAIR,
    stablecoin_auth: *pda::HYUSD_AUTH,
    usdc_vault_auth,
    usdc_fee_auth,
    stablecoin_fee_auth: pda::fee_auth(HYUSD::MINT),
    usdc_collateral_vault: ata!(usdc_vault_auth, USDC::MINT),
    usdc_fee_vault: ata!(usdc_fee_auth, USDC::MINT),
    stablecoin_fee_vault: pda::fee_vault(HYUSD::MINT),
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_usdc_ta: pda::usdc_ata(user),
    stablecoin_mint: HYUSD::MINT,
    usdc_mint: USDC::MINT,
    usdc_usd_pyth_feed: pda::USDC_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}

/// Builds account context for hyUSD redeem to USDC.
#[must_use]
pub fn redeem_stablecoin_usdc(user: Pubkey) -> RedeemStablecoinUsdc {
  let usdc_vault_auth = pda::vault_auth(USDC::MINT);
  let usdc_fee_auth = pda::fee_auth(USDC::MINT);
  RedeemStablecoinUsdc {
    user,
    hylo: *pda::HYLO,
    usdc_pair: *pda::USDC_PAIR,
    stablecoin_auth: *pda::HYUSD_AUTH,
    usdc_vault_auth,
    usdc_fee_auth,
    stablecoin_fee_auth: pda::fee_auth(HYUSD::MINT),
    usdc_collateral_vault: ata!(usdc_vault_auth, USDC::MINT),
    usdc_fee_vault: ata!(usdc_fee_auth, USDC::MINT),
    stablecoin_fee_vault: pda::fee_vault(HYUSD::MINT),
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_usdc_ta: pda::usdc_ata(user),
    stablecoin_mint: HYUSD::MINT,
    usdc_mint: USDC::MINT,
    usdc_usd_pyth_feed: pda::USDC_USD_PYTH_FEED,
    token_program: token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  }
}
