//! Instruction builders for Hylo Exchange.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::{associated_token, token};
use solana_address_lookup_table_interface::program as address_lookup_table;

use crate::hylo_exchange::client::{accounts, args};
use crate::hylo_exchange::types::SlippageConfig;
use crate::pda::{self, metadata};
use crate::tokens::{TokenMint, HYUSD, XSOL};
use crate::{ata, hylo_exchange};

#[must_use]
pub fn mint_stablecoin(
  amount_lst_to_deposit: u64,
  user: Pubkey,
  lst_mint: Pubkey,
  slippage_config: Option<SlippageConfig>,
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
    stablecoin_mint: HYUSD::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  let instruction_args = args::MintStablecoin {
    amount_lst_to_deposit,
    slippage_config,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn mint_levercoin(
  amount_lst_to_deposit: u64,
  user: Pubkey,
  lst_mint: Pubkey,
  slippage_config: Option<SlippageConfig>,
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
    levercoin_mint: XSOL::MINT,
    stablecoin_mint: HYUSD::MINT,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    system_program: system_program::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  let instruction_args = args::MintLevercoin {
    amount_lst_to_deposit,
    slippage_config,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn redeem_stablecoin(
  amount_to_redeem: u64,
  user: Pubkey,
  lst_mint: Pubkey,
  slippage_config: Option<SlippageConfig>,
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
    stablecoin_mint: HYUSD::MINT,
    lst_mint,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  let instruction_args = args::RedeemStablecoin {
    amount_to_redeem,
    slippage_config,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn redeem_levercoin(
  amount_to_redeem: u64,
  user: Pubkey,
  lst_mint: Pubkey,
  slippage_config: Option<SlippageConfig>,
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
    levercoin_mint: XSOL::MINT,
    stablecoin_mint: HYUSD::MINT,
    lst_mint,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    system_program: system_program::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };

  let instruction_args = args::RedeemLevercoin {
    amount_to_redeem,
    slippage_config,
  };

  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn swap_stable_to_lever(
  amount_stablecoin: u64,
  user: Pubkey,
  slippage_config: Option<SlippageConfig>,
) -> Instruction {
  let accounts = accounts::SwapStableToLever {
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
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };

  let instruction_args = args::SwapStableToLever {
    amount_stablecoin,
    slippage_config,
  };

  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn swap_lever_to_stable(
  amount_levercoin: u64,
  user: Pubkey,
  slippage_config: Option<SlippageConfig>,
) -> Instruction {
  let accounts = accounts::SwapLeverToStable {
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
    event_authority: *pda::EXCHANGE_EVENT_AUTH,
    program: hylo_exchange::ID,
  };
  let instruction_args = args::SwapLeverToStable {
    amount_levercoin,
    slippage_config,
  };
  Instruction {
    program_id: hylo_exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: instruction_args.data(),
  }
}

#[must_use]
pub fn initialize_protocol(
  upgrade_authority: Pubkey,
  treasury: Pubkey,
  args: &args::InitializeProtocol,
  admin: Pubkey,
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
    associated_token_program: associated_token::ID,
    token_program: token::ID,
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
