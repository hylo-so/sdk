use anchor_lang::prelude::{Pubkey, ToAccountMetas};
use hylo_clients::instruction_accounts;
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use jupiter_amm_interface::{Swap, SwapAndAccountMetas};

/// Creates account metas for minting stablecoin (LST -> hyUSD).
#[must_use]
pub fn mint_stablecoin(user: Pubkey, lst_mint: Pubkey) -> SwapAndAccountMetas {
  let accounts = instruction_accounts::mint_stablecoin(user, lst_mint);
  let account_metas = accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: lst_mint,
      out_token: HYUSD::MINT,
    },
    account_metas,
  }
}

/// Creates account metas for minting levercoin (LST -> xSOL).
#[must_use]
pub fn mint_levercoin(user: Pubkey, lst_mint: Pubkey) -> SwapAndAccountMetas {
  let accounts = instruction_accounts::mint_levercoin(user, lst_mint);
  let account_metas = accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: lst_mint,
      out_token: XSOL::MINT,
    },
    account_metas,
  }
}

/// Creates account metas for redeeming stablecoin (hyUSD -> LST).
#[must_use]
pub fn redeem_stablecoin(
  user: Pubkey,
  lst_mint: Pubkey,
) -> SwapAndAccountMetas {
  let accounts = instruction_accounts::redeem_stablecoin(user, lst_mint);
  let account_metas = accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: HYUSD::MINT,
      out_token: lst_mint,
    },
    account_metas,
  }
}

/// Creates account metas for redeeming levercoin (xSOL -> LST).
#[must_use]
pub fn redeem_levercoin(user: Pubkey, lst_mint: Pubkey) -> SwapAndAccountMetas {
  let accounts = instruction_accounts::redeem_levercoin(user, lst_mint);
  let account_metas = accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: XSOL::MINT,
      out_token: lst_mint,
    },
    account_metas,
  }
}

/// Creates account metas for swapping stablecoin to levercoin (hyUSD -> xSOL).
#[must_use]
pub fn swap_stable_to_lever(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = instruction_accounts::swap_stable_to_lever(user);
  let account_metas = accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: HYUSD::MINT,
      out_token: XSOL::MINT,
    },
    account_metas,
  }
}

/// Creates account metas for swapping levercoin to stablecoin (xSOL -> hyUSD).
#[must_use]
pub fn swap_lever_to_stable(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = instruction_accounts::swap_lever_to_stable(user);
  let account_metas = accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: XSOL::MINT,
      out_token: HYUSD::MINT,
    },
    account_metas,
  }
}

/// Creates account metas for depositing into stability pool (hyUSD -> shyUSD).
#[must_use]
pub fn stability_pool_deposit(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = instruction_accounts::stability_pool_deposit(user);
  let account_metas = accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: HYUSD::MINT,
      out_token: SHYUSD::MINT,
    },
    account_metas,
  }
}

/// Creates account metas for withdrawing from stability pool (shyUSD -> hyUSD).
#[must_use]
pub fn stability_pool_withdraw(user: Pubkey) -> SwapAndAccountMetas {
  let accounts = instruction_accounts::stability_pool_withdraw(user);
  let account_metas = accounts.to_account_metas(None);
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: SHYUSD::MINT,
      out_token: HYUSD::MINT,
    },
    account_metas,
  }
}

/// Creates account metas for stability pool withdrawal and redemption with only
/// stablecoin in pool (shyUSD -> hyUSD -> LST).
#[must_use]
pub fn stability_pool_liquidate(
  user: Pubkey,
  lst_mint: Pubkey,
) -> SwapAndAccountMetas {
  let withdraw_account_metas =
    instruction_accounts::stability_pool_withdraw(user).to_account_metas(None);
  let redeem_stablecoin_account_metas =
    instruction_accounts::redeem_stablecoin(user, lst_mint)
      .to_account_metas(None);
  let account_metas =
    [withdraw_account_metas, redeem_stablecoin_account_metas].concat();
  SwapAndAccountMetas {
    swap: Swap::Hylo {
      in_token: SHYUSD::MINT,
      out_token: lst_mint,
    },
    account_metas,
  }
}

/// Creates account metas for fully liquidating withdrawal from stability pool
/// (shyUSD -> LST via hyUSD and xSOL).
#[must_use]
pub fn stability_pool_liquidate_levercoin(
  user: Pubkey,
  lst_mint: Pubkey,
) -> SwapAndAccountMetas {
  let base_liquidation = stability_pool_liquidate(user, lst_mint);
  let redeem_levercoin_account_metas =
    instruction_accounts::redeem_levercoin(user, lst_mint)
      .to_account_metas(None);
  let account_metas = [
    base_liquidation.account_metas,
    redeem_levercoin_account_metas,
  ]
  .concat();
  SwapAndAccountMetas {
    swap: base_liquidation.swap,
    account_metas,
  }
}
