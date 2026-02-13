use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};

use crate::exchange::client::accounts::{
  HarvestFundingRate, MintLevercoin, MintLevercoinExo, MintStablecoin,
  MintStablecoinExo, RedeemLevercoin, RedeemLevercoinExo, RedeemStablecoin,
  RedeemStablecoinExo, RegisterExo, SwapLeverToStable, SwapLeverToStableExo,
  SwapLst, SwapStableToLever, SwapStableToLeverExo, WithdrawFees,
};
use crate::tokens::{TokenMint, HYUSD, XSOL};
use crate::{ata, pda, stability_pool};

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
    user_lst_ta: ata!(user, lst_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    lst_mint,
    stablecoin_mint: HYUSD::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
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
    user_lst_ta: ata!(user, lst_mint),
    user_levercoin_ta: pda::xsol_ata(user),
    lst_mint,
    levercoin_mint: XSOL::MINT,
    stablecoin_mint: HYUSD::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
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
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_lst_ta: ata!(user, lst_mint),
    stablecoin_mint: HYUSD::MINT,
    lst_mint,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
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
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_levercoin_ta: pda::xsol_ata(user),
    user_lst_ta: ata!(user, lst_mint),
    levercoin_mint: XSOL::MINT,
    stablecoin_mint: HYUSD::MINT,
    lst_mint,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
  }
}

/// Builds account context for stable-to-lever swap (hyUSD -> xSOL).
#[must_use]
pub fn swap_stable_to_lever(user: Pubkey) -> SwapStableToLever {
  SwapStableToLever {
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
  }
}

/// Builds account context for lever-to-stable swap (xSOL -> hyUSD).
#[must_use]
pub fn swap_lever_to_stable(user: Pubkey) -> SwapLeverToStable {
  SwapLeverToStable {
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
  }
}

/// Builds account context for registering an EXO pair.
#[must_use]
pub fn register_exo(admin: Pubkey, collateral_mint: Pubkey) -> RegisterExo {
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
    metadata_program: mpl_token_metadata::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
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
    hylo: *pda::HYLO,
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
    hylo: *pda::HYLO,
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
  }
}

/// Builds account context for harvesting exo funding rate.
#[must_use]
pub fn harvest_funding_rate(
  payer: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> HarvestFundingRate {
  let vault_auth = pda::vault_auth(collateral_mint);
  HarvestFundingRate {
    payer,
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
  }
}

/// Lever-to-stable swap (xAsset -> hyUSD).
#[must_use]
pub fn swap_lever_to_stable_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapLeverToStableExo {
  let vault_auth = pda::vault_auth(collateral_mint);
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  SwapLeverToStableExo {
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
  }
}

/// Stable-to-lever swap (hyUSD -> xAsset).
#[must_use]
pub fn swap_stable_to_lever_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> SwapStableToLeverExo {
  let vault_auth = pda::vault_auth(collateral_mint);
  let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
  SwapStableToLeverExo {
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
  }
}

/// Builds account context for LST swap feature
#[must_use]
pub fn swap_lst(user: Pubkey, lst_a: Pubkey, lst_b: Pubkey) -> SwapLst {
  SwapLst {
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
    associated_token_program: associated_token::ID,
  }
}
