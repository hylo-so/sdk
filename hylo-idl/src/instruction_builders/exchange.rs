//! Instruction builders for Hylo Exchange.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::{associated_token, token};
use solana_address_lookup_table_interface::program as address_lookup_table;

use crate::exchange::account_builders;
use crate::exchange::client::{accounts, args};
use crate::pda::{self, metadata};
use crate::tokens::{TokenMint, HYUSD, XSOL};
use crate::{exchange, stability_pool};

#[must_use]
pub fn mint_stablecoin(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::MintStablecoin,
) -> Instruction {
  let accounts = account_builders::mint_stablecoin(user, lst_mint);
  Instruction {
    program_id: exchange::ID,
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
  let accounts = account_builders::mint_levercoin(user, lst_mint);
  Instruction {
    program_id: exchange::ID,
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
  let accounts = account_builders::redeem_stablecoin(user, lst_mint);
  Instruction {
    program_id: exchange::ID,
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
  let accounts = account_builders::redeem_levercoin(user, lst_mint);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_stable_to_lever(
  user: Pubkey,
  args: &args::SwapStableToLever,
) -> Instruction {
  let accounts = account_builders::swap_stable_to_lever(user);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_lever_to_stable(
  user: Pubkey,
  args: &args::SwapLeverToStable,
) -> Instruction {
  let accounts = account_builders::swap_lever_to_stable(user);
  Instruction {
    program_id: exchange::ID,
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
    hylo_exchange: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
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
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint: XSOL::MINT,
    stablecoin_metadata: metadata(HYUSD::MINT),
    levercoin_metadata: metadata(XSOL::MINT),
    metadata_program: mpl_token_metadata::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
  };
  let args = args::InitializeMints {};
  Instruction {
    program_id: exchange::ID,
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
    program_id: exchange::ID,
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
    program_id: exchange::ID,
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
    associated_token_program: associated_token::ID,
    token_program: token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: exchange::ID,
  };
  let args = args::RegisterLst {};
  Instruction {
    program_id: exchange::ID,
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
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
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
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
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
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
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
    stablecoin_mint: HYUSD::MINT,
    stablecoin_auth: *pda::HYUSD_AUTH,
    levercoin_mint: XSOL::MINT,
    levercoin_auth: *pda::XSOL_AUTH,
    stablecoin_fee_auth: pda::fee_auth(HYUSD::MINT),
    stablecoin_fee_vault: pda::fee_vault(HYUSD::MINT),
    levercoin_fee_auth: pda::fee_auth(XSOL::MINT),
    levercoin_fee_vault: pda::fee_vault(XSOL::MINT),
    stablecoin_pool: *pda::HYUSD_POOL,
    levercoin_pool: *pda::XSOL_POOL,
    pool_auth: *pda::POOL_AUTH,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    hylo_stability_pool: stability_pool::ID,
    lst_registry,
    lut_program: address_lookup_table::ID,
    associated_token_program: associated_token::ID,
    token_program: token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: exchange::ID,
  };
  let args = args::HarvestYield {};
  Instruction {
    program_id: exchange::ID,
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
    program: exchange::ID,
  };
  let args = args::UpdateLstPrices {};
  Instruction {
    program_id: exchange::ID,
    accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
    data: args.data(),
  }
}
