//! Instruction builders for Hylo Exchange.

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_sdk_ids::{address_lookup_table, system_program};
use spl_associated_token_account_interface::program::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token_interface::ID as TOKEN_PROGRAM_ID;

use crate::hylo_exchange::client::{accounts, args};
use crate::pda::{self, metadata};
use crate::tokens::{HYUSD, XSOL};
use crate::{ata, hylo_exchange, hylo_stability_pool, MPL_TOKEN_METADATA_ID};

#[must_use]
pub fn mint_stablecoin(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::MintStablecoin,
) -> Instruction {
  let accounts = accounts::MintStablecoin {
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
    stablecoin_mint: HYUSD,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: TOKEN_PROGRAM_ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn mint_levercoin(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::MintLevercoin,
) -> Instruction {
  let accounts = accounts::MintLevercoin {
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
    levercoin_mint: XSOL,
    stablecoin_mint: HYUSD,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: TOKEN_PROGRAM_ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn redeem_stablecoin(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::RedeemStablecoin,
) -> Instruction {
  let accounts = accounts::RedeemStablecoin {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_stablecoin_ta: pda::hyusd_ata(user),
    user_lst_ta: ata!(user, lst_mint),
    stablecoin_mint: HYUSD,
    lst_mint,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: TOKEN_PROGRAM_ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn redeem_levercoin(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::RedeemLevercoin,
) -> Instruction {
  let accounts = accounts::RedeemLevercoin {
    user,
    hylo: *pda::HYLO,
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_header: pda::lst_header(lst_mint),
    user_levercoin_ta: pda::xsol_ata(user),
    user_lst_ta: ata!(user, lst_mint),
    levercoin_mint: XSOL,
    stablecoin_mint: HYUSD,
    lst_mint,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: TOKEN_PROGRAM_ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };

  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_stable_to_lever(
  user: Pubkey,
  args: &args::SwapStableToLever,
) -> Instruction {
  let accounts = accounts::SwapStableToLever {
    user,
    hylo: *pda::HYLO,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    stablecoin_mint: HYUSD,
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD),
    fee_vault: pda::fee_vault(HYUSD),
    user_stablecoin_ta: pda::hyusd_ata(user),
    levercoin_mint: XSOL,
    levercoin_auth: *pda::XSOL_AUTH,
    user_levercoin_ta: pda::xsol_ata(user),
    token_program: TOKEN_PROGRAM_ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };

  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_lever_to_stable(
  user: Pubkey,
  args: &args::SwapLeverToStable,
) -> Instruction {
  let accounts = accounts::SwapLeverToStable {
    user,
    hylo: *pda::HYLO,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    stablecoin_mint: HYUSD,
    stablecoin_auth: *pda::HYUSD_AUTH,
    fee_auth: pda::fee_auth(HYUSD),
    fee_vault: pda::fee_vault(HYUSD),
    user_stablecoin_ta: pda::hyusd_ata(user),
    levercoin_mint: XSOL,
    levercoin_auth: *pda::XSOL_AUTH,
    user_levercoin_ta: pda::xsol_ata(user),
    token_program: TOKEN_PROGRAM_ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_protocol(
  admin: Pubkey,
  upgrade_authority: Pubkey,
  treasury: Pubkey,
  args: &args::InitializeProtocol,
) -> Instruction {
  let accounts = accounts::InitializeProtocol {
    admin,
    upgrade_authority,
    hylo: *pda::HYLO,
    treasury,
    system_program: system_program::ID,
    program_data: *pda::EXCHANGE_PROGRAM_DATA,
    hylo_exchange: hylo_exchange::ID,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_mints(admin: Pubkey) -> Instruction {
  let accounts = accounts::InitializeMints {
    admin,
    hylo: *pda::HYLO,
    stablecoin_auth: *pda::HYUSD_AUTH,
    levercoin_auth: *pda::XSOL_AUTH,
    stablecoin_mint: HYUSD,
    levercoin_mint: XSOL,
    stablecoin_metadata: metadata(HYUSD),
    levercoin_metadata: metadata(XSOL),
    metadata_program: MPL_TOKEN_METADATA_ID,
    token_program: TOKEN_PROGRAM_ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    system_program: system_program::ID,
  };
  let args = args::InitializeMints {};
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_lst_registry(slot: u64, admin: Pubkey) -> Instruction {
  let accounts = accounts::InitializeLstRegistry {
    admin,
    hylo: *pda::HYLO,
    registry_auth: *pda::LST_REGISTRY_AUTH,
    lst_registry: pda::new_lst_registry(slot),
    lut_program: address_lookup_table::ID,
    system_program: system_program::ID,
  };
  let args = args::InitializeLstRegistry { slot };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_lst_registry_calculators(
  lst_registry: Pubkey,
  admin: Pubkey,
) -> Instruction {
  let accounts = accounts::InitializeLstRegistryCalculators {
    admin,
    hylo: *pda::HYLO,
    lst_registry_auth: *pda::LST_REGISTRY_AUTH,
    lst_registry,
    lut_program: address_lookup_table::ID,
    system_program: system_program::ID,
  };
  let args = args::InitializeLstRegistryCalculators {};
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn register_lst(
  lst_mint: Pubkey,
  lst_stake_pool_state: Pubkey,
  sanctum_calculator_program: Pubkey,
  sanctum_calculator_state: Pubkey,
  stake_pool_program: Pubkey,
  stake_pool_program_data: Pubkey,
  lst_registry: Pubkey,
  admin: Pubkey,
) -> Instruction {
  let accounts = accounts::RegisterLst {
    admin,
    hylo: *pda::HYLO,
    lst_header: pda::lst_header(lst_mint),
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::vault_auth(lst_mint),
    registry_auth: *pda::LST_REGISTRY_AUTH,
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::vault(lst_mint),
    lst_mint,
    lst_registry,
    lst_stake_pool_state,
    sanctum_calculator_program,
    sanctum_calculator_state,
    stake_pool_program_data,
    stake_pool_program,
    lut_program: address_lookup_table::ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    token_program: TOKEN_PROGRAM_ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  let args = args::RegisterLst {};
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_oracle_conf_tolerance(
  admin: Pubkey,
  args: &args::UpdateOracleConfTolerance,
) -> Instruction {
  let accounts = accounts::UpdateOracleConfTolerance {
    admin,
    hylo: *pda::HYLO,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_sol_usd_oracle(
  admin: Pubkey,
  args: &args::UpdateSolUsdOracle,
) -> Instruction {
  let accounts = accounts::UpdateSolUsdOracle {
    admin,
    hylo: *pda::HYLO,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_stability_pool(
  admin: Pubkey,
  args: &args::UpdateStabilityPool,
) -> Instruction {
  let accounts = accounts::UpdateStabilityPool {
    admin,
    hylo: *pda::HYLO,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn harvest_yield(
  payer: Pubkey,
  lst_registry: Pubkey,
  remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
  let accounts = accounts::HarvestYield {
    payer,
    hylo: *pda::HYLO,
    stablecoin_mint: HYUSD,
    stablecoin_auth: *pda::HYUSD_AUTH,
    levercoin_mint: XSOL,
    levercoin_auth: *pda::XSOL_AUTH,
    stablecoin_fee_auth: pda::fee_auth(HYUSD),
    stablecoin_fee_vault: pda::fee_vault(HYUSD),
    levercoin_fee_auth: pda::fee_auth(XSOL),
    levercoin_fee_vault: pda::fee_vault(XSOL),
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_pool: *pda::XSOL_POOL,
    pool_auth: *pda::POOL_AUTH,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    hylo_stability_pool: hylo_stability_pool::ID,
    lst_registry,
    lut_program: address_lookup_table::ID,
    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
    token_program: TOKEN_PROGRAM_ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  let args = args::HarvestYield {};
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
    data: args.data(),
  }
}

#[must_use]
pub fn update_lst_prices(
  payer: Pubkey,
  lst_registry: Pubkey,
  remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
  let accounts = accounts::UpdateLstPrices {
    payer,
    hylo: *pda::HYLO,
    lst_registry,
    lut_program: address_lookup_table::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  let args = args::UpdateLstPrices {};
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
    data: args.data(),
  }
}
