//! Instruction builders for Hylo Exchange.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::sysvar::rent;
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::{associated_token, token};
use solana_address_lookup_table_interface::program as address_lookup_table;

use crate::exchange::account_builders;
use crate::exchange::client::{accounts, args};
use crate::exchange::types::{AddressField, TokenMetadata, UFixValue64};
use crate::pda::{self, metadata};
use crate::tokens::{TokenMint, HYUSD, XSOL};
use crate::{earn_pool, exchange};

#[must_use]
pub fn mint_stablecoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::MintStablecoinLst,
) -> Instruction {
  let accounts = account_builders::mint_stablecoin_lst(user, lst_mint);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn mint_levercoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::MintLevercoinLst,
) -> Instruction {
  let accounts = account_builders::mint_levercoin_lst(user, lst_mint);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn redeem_stablecoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::RedeemStablecoinLst,
) -> Instruction {
  let accounts = account_builders::redeem_stablecoin_lst(user, lst_mint);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn redeem_levercoin_lst(
  user: Pubkey,
  lst_mint: Pubkey,
  args: &args::RedeemLevercoinLst,
) -> Instruction {
  let accounts = account_builders::redeem_levercoin_lst(user, lst_mint);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn convert_stable_to_lever_lst(
  user: Pubkey,
  args: &args::ConvertStableToLeverLst,
) -> Instruction {
  let accounts = account_builders::convert_stable_to_lever_lst(user);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn convert_lever_to_stable_lst(
  user: Pubkey,
  args: &args::ConvertLeverToStableLst,
) -> Instruction {
  let accounts = account_builders::convert_lever_to_stable_lst(user);
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
    hylo: pda::HYLO,
    treasury,
    system_program: system_program::ID,
    program_data: pda::EXCHANGE_PROGRAM_DATA,
    hylo_exchange: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_mints(
  admin: Pubkey,
  stablecoin_metadata: TokenMetadata,
  levercoin_metadata: TokenMetadata,
) -> Instruction {
  let accounts = accounts::InitializeMints {
    admin,
    hylo: pda::HYLO,
    stablecoin_auth: pda::HYUSD_AUTH,
    levercoin_auth: pda::XSOL_AUTH,
    stablecoin_mint: HYUSD::MINT,
    levercoin_mint: XSOL::MINT,
    stablecoin_metadata: metadata(HYUSD::MINT),
    levercoin_metadata: metadata(XSOL::MINT),
    metadata_program: mpl_token_metadata::ID,
    token_program: token::ID,
    associated_token_program: associated_token::ID,
    rent: rent::ID,
    system_program: system_program::ID,
  };
  let args = args::InitializeMints {
    stablecoin_metadata,
    levercoin_metadata,
  };
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
    hylo: pda::HYLO,
    registry_auth: pda::LST_REGISTRY_AUTH,
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
    hylo: pda::HYLO,
    lst_registry_auth: pda::LST_REGISTRY_AUTH,
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
pub fn register_lst(
  lst_mint: Pubkey,
  lst_stake_pool_state: Pubkey,
  sanctum_calculator_program: Pubkey,
  sanctum_calculator_state: Pubkey,
  stake_pool_program: Pubkey,
  stake_pool_program_data: Pubkey,
  lst_registry: Pubkey,
  admin: Pubkey,
  rebalance_fee: UFixValue64,
) -> Instruction {
  let accounts = accounts::RegisterLst {
    admin,
    hylo: pda::HYLO,
    lst_header: pda::lst_header(lst_mint),
    fee_auth: pda::fee_auth(lst_mint),
    vault_auth: pda::lst_vault_auth(lst_mint),
    registry_auth: pda::LST_REGISTRY_AUTH,
    fee_vault: pda::fee_vault(lst_mint),
    lst_vault: pda::lst_vault(lst_mint),
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
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::RegisterLst { rebalance_fee };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_lst_rebalance_fee(
  admin: Pubkey,
  lst_mint: Pubkey,
  args: &args::UpdateLstRebalanceFee,
) -> Instruction {
  let accounts = account_builders::update_lst_rebalance_fee(admin, lst_mint);
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
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
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
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn settle_virtual_stablecoin_lst() -> Instruction {
  let accounts = account_builders::settle_virtual_stablecoin_lst();
  let args = args::SettleVirtualStablecoinLst {};
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn settle_virtual_stablecoin_exo(
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> Instruction {
  let accounts = account_builders::settle_virtual_stablecoin_exo(
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  let args = args::SettleVirtualStablecoinExo {};
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn harvest_yield(
  lst_registry: Pubkey,
  remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
  let accounts = accounts::HarvestYield {
    hylo: pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    stablecoin_auth: pda::HYUSD_AUTH,
    stablecoin_fee_auth: pda::fee_auth(HYUSD::MINT),
    stablecoin_fee_vault: pda::fee_vault(HYUSD::MINT),
    stablecoin_pool: pda::HYUSD_POOL,
    pool_auth: pda::POOL_AUTH,
    sol_usd_pyth_feed: pda::SOL_USD_PYTH_FEED,
    hylo_earn_pool: earn_pool::ID,
    lst_registry,
    lut_program: address_lookup_table::ID,
    token_program: token::ID,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
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
    hylo: pda::HYLO,
    lst_registry,
    lut_program: address_lookup_table::ID,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::UpdateLstPrices {};
  Instruction {
    program_id: exchange::ID,
    accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_lst_to_lst(
  user: Pubkey,
  lst_a: Pubkey,
  lst_b: Pubkey,
  args: &args::SwapLstToLst,
) -> Instruction {
  let accounts = account_builders::swap_lst_to_lst(user, lst_a, lst_b);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn register_exo(
  admin: Pubkey,
  collateral_mint: Pubkey,
  exo_usd_pyth_feed: Pubkey,
  args: &args::RegisterExo,
) -> Instruction {
  let accounts =
    account_builders::register_exo(admin, collateral_mint, exo_usd_pyth_feed);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn mint_levercoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
  args: &args::MintLevercoinExo,
) -> Instruction {
  let accounts = account_builders::mint_levercoin_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn mint_stablecoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
  args: &args::MintStablecoinExo,
) -> Instruction {
  let accounts = account_builders::mint_stablecoin_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn redeem_levercoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
  args: &args::RedeemLevercoinExo,
) -> Instruction {
  let accounts = account_builders::redeem_levercoin_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn redeem_stablecoin_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
  args: &args::RedeemStablecoinExo,
) -> Instruction {
  let accounts = account_builders::redeem_stablecoin_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn harvest_borrow_rate(
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
) -> Instruction {
  let accounts = account_builders::harvest_borrow_rate(
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  let args = args::HarvestBorrowRate {};
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn convert_lever_to_stable_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
  args: &args::ConvertLeverToStableExo,
) -> Instruction {
  let accounts = account_builders::convert_lever_to_stable_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn convert_stable_to_lever_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
  args: &args::ConvertStableToLeverExo,
) -> Instruction {
  let accounts = account_builders::convert_stable_to_lever_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_lst_swap_fee(
  admin: Pubkey,
  args: &args::UpdateLstSwapFee,
) -> Instruction {
  let accounts = accounts::UpdateLstSwapFee {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_levercoin_fees(
  admin: Pubkey,
  args: &args::UpdateLevercoinFees,
) -> Instruction {
  let accounts = accounts::UpdateLevercoinFees {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_oracle_interval(
  admin: Pubkey,
  args: &args::UpdateOracleInterval,
) -> Instruction {
  let accounts = accounts::UpdateOracleInterval {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_lst_stablecoin_mint_threshold(
  admin: Pubkey,
  args: &args::UpdateLstStablecoinMintThreshold,
) -> Instruction {
  let accounts = accounts::UpdateLstStablecoinMintThreshold {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn pause_protocol(pause_authority: Pubkey) -> Instruction {
  let accounts = accounts::PauseProtocol {
    pause_authority,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::PauseProtocol {}.data(),
  }
}

#[must_use]
pub fn unpause_protocol(admin: Pubkey) -> Instruction {
  let accounts = accounts::UnpauseProtocol {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::UnpauseProtocol {}.data(),
  }
}

#[must_use]
pub fn pause_lst_pair(pause_authority: Pubkey) -> Instruction {
  let accounts = accounts::PauseLstPair {
    pause_authority,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::PauseLstPair {}.data(),
  }
}

#[must_use]
pub fn unpause_lst_pair(admin: Pubkey) -> Instruction {
  let accounts = accounts::UnpauseLstPair {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::UnpauseLstPair {}.data(),
  }
}

#[must_use]
pub fn pause_exo_pair(
  pause_authority: Pubkey,
  collateral_mint: Pubkey,
) -> Instruction {
  let accounts = accounts::PauseExoPair {
    pause_authority,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::PauseExoPair {}.data(),
  }
}

#[must_use]
pub fn unpause_exo_pair(admin: Pubkey, collateral_mint: Pubkey) -> Instruction {
  let accounts = accounts::UnpauseExoPair {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::UnpauseExoPair {}.data(),
  }
}

#[must_use]
pub fn update_exo_rebalance_deviation_tolerance(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoRebalanceDeviationTolerance,
) -> Instruction {
  let accounts = accounts::UpdateExoRebalanceDeviationTolerance {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_lst_rebalance_deviation_tolerance(
  admin: Pubkey,
  args: &args::UpdateLstRebalanceDeviationTolerance,
) -> Instruction {
  let accounts = accounts::UpdateLstRebalanceDeviationTolerance {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn pause_usdc_pair(pause_authority: Pubkey) -> Instruction {
  let accounts = accounts::PauseUsdcPair {
    pause_authority,
    hylo: pda::HYLO,
    usdc_pair: pda::USDC_PAIR,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::PauseUsdcPair {}.data(),
  }
}

#[must_use]
pub fn unpause_usdc_pair(admin: Pubkey) -> Instruction {
  let accounts = accounts::UnpauseUsdcPair {
    admin,
    hylo: pda::HYLO,
    usdc_pair: pda::USDC_PAIR,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::UnpauseUsdcPair {}.data(),
  }
}

#[must_use]
pub fn update_lst_buy_curve_config(
  admin: Pubkey,
  args: &args::UpdateLstBuyCurveConfig,
) -> Instruction {
  let accounts = accounts::UpdateLstBuyCurveConfig {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_lst_sell_curve_config(
  admin: Pubkey,
  args: &args::UpdateLstSellCurveConfig,
) -> Instruction {
  let accounts = accounts::UpdateLstSellCurveConfig {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_yield_harvest_config(
  admin: Pubkey,
  args: &args::UpdateYieldHarvestConfig,
) -> Instruction {
  let accounts = accounts::UpdateYieldHarvestConfig {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_borrow_rate(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoBorrowRate,
) -> Instruction {
  let accounts = accounts::UpdateExoBorrowRate {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_oracle(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoOracle,
) -> Instruction {
  let accounts = accounts::UpdateExoOracle {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_oracle_conf_tolerance(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoOracleConfTolerance,
) -> Instruction {
  let accounts = accounts::UpdateExoOracleConfTolerance {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_oracle_interval(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoOracleInterval,
) -> Instruction {
  let accounts = accounts::UpdateExoOracleInterval {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_stablecoin_mint_threshold(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoStablecoinMintThreshold,
) -> Instruction {
  let accounts = accounts::UpdateExoStablecoinMintThreshold {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_buy_curve(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoBuyCurve,
) -> Instruction {
  let accounts = accounts::UpdateExoBuyCurve {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_sell_curve(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoSellCurve,
) -> Instruction {
  let accounts = accounts::UpdateExoSellCurve {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_levercoin_fees(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoLevercoinFees,
) -> Instruction {
  let accounts = accounts::UpdateExoLevercoinFees {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_exo_levercoin_market_cap_limit(
  admin: Pubkey,
  collateral_mint: Pubkey,
  args: &args::UpdateExoLevercoinMarketCapLimit,
) -> Instruction {
  let accounts = account_builders::update_exo_levercoin_market_cap_limit(
    admin,
    collateral_mint,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_usdc(
  admin: Pubkey,
  usdc_usd_pyth_feed: Pubkey,
  args: &args::InitializeUsdc,
) -> Instruction {
  let accounts = account_builders::initialize_usdc(admin, usdc_usd_pyth_feed);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn redeem_stablecoin_usdc(
  user: Pubkey,
  args: &args::RedeemStablecoinUsdc,
) -> Instruction {
  let accounts = account_builders::redeem_stablecoin_usdc(user);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn mint_stablecoin_usdc(
  user: Pubkey,
  args: &args::MintStablecoinUsdc,
) -> Instruction {
  let accounts = account_builders::mint_stablecoin_usdc(user);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_usdc_oracle_conf_tolerance(
  admin: Pubkey,
  args: &args::UpdateUsdcOracleConfTolerance,
) -> Instruction {
  let accounts = accounts::UpdateUsdcOracleConfTolerance {
    admin,
    hylo: pda::HYLO,
    usdc_pair: pda::USDC_PAIR,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_usdc_oracle_interval(
  admin: Pubkey,
  args: &args::UpdateUsdcOracleInterval,
) -> Instruction {
  let accounts = accounts::UpdateUsdcOracleInterval {
    admin,
    hylo: pda::HYLO,
    usdc_pair: pda::USDC_PAIR,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn update_usdc_swap_fee(
  admin: Pubkey,
  args: &args::UpdateUsdcSwapFee,
) -> Instruction {
  let accounts = accounts::UpdateUsdcSwapFee {
    admin,
    hylo: pda::HYLO,
    usdc_pair: pda::USDC_PAIR,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_lst_virtual_stablecoin(admin: Pubkey) -> Instruction {
  let accounts = accounts::InitializeLstVirtualStablecoin {
    admin,
    hylo: pda::HYLO,
    stablecoin_mint: HYUSD::MINT,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::InitializeLstVirtualStablecoin {};
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_pool_drawdown_lst(admin: Pubkey) -> Instruction {
  let accounts = accounts::InitializePoolDrawdownLst {
    admin,
    hylo: pda::HYLO,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::InitializePoolDrawdownLst {};
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn initialize_pool_drawdown_exo(
  admin: Pubkey,
  collateral_mint: Pubkey,
) -> Instruction {
  let accounts = accounts::InitializePoolDrawdownExo {
    admin,
    hylo: pda::HYLO,
    exo_pair: pda::exo_pair(collateral_mint),
    collateral_mint,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::InitializePoolDrawdownExo {};
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_exo_to_usdc(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
  args: &args::SwapExoToUsdc,
) -> Instruction {
  let accounts = account_builders::swap_exo_to_usdc(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_usdc_to_exo(
  user: Pubkey,
  collateral_mint: Pubkey,
  collateral_usd_pyth_feed: Pubkey,
  args: &args::SwapUsdcToExo,
) -> Instruction {
  let accounts = account_builders::swap_usdc_to_exo(
    user,
    collateral_mint,
    collateral_usd_pyth_feed,
  );
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_lst_to_usdc(
  user: Pubkey,
  lst_mint: Pubkey,
  pool_state: Pubkey,
  args: &args::SwapLstToUsdc,
) -> Instruction {
  let accounts = account_builders::swap_lst_to_usdc(user, lst_mint, pool_state);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn swap_usdc_to_lst(
  user: Pubkey,
  lst_mint: Pubkey,
  pool_state: Pubkey,
  args: &args::SwapUsdcToLst,
) -> Instruction {
  let accounts = account_builders::swap_usdc_to_lst(user, lst_mint, pool_state);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

#[must_use]
pub fn withdraw_fees(
  payer: Pubkey,
  treasury: Pubkey,
  fee_token_mint: Pubkey,
) -> Instruction {
  let accounts =
    account_builders::withdraw_fees(payer, treasury, fee_token_mint);
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args::WithdrawFees {}.data(),
  }
}

/// Proposes an update to one of the protocol's privileged addresses
/// (admin, treasury, or pause authority). The admin signs.
#[must_use]
pub fn propose_address_update(
  admin: Pubkey,
  address_field: AddressField,
  new_address: Pubkey,
  ttl_secs: u64,
) -> Instruction {
  let accounts = accounts::ProposeAddressUpdate {
    admin,
    hylo: pda::HYLO,
    proposal: pda::address_update_proposal(address_field),
    system_program: system_program::ID,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::ProposeAddressUpdate {
    address_field,
    new_address,
    ttl_secs,
  };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Approves an outstanding address update proposal. The program upgrade
/// authority signs.
#[must_use]
pub fn approve_address_update(
  upgrade_authority: Pubkey,
  address_field: AddressField,
) -> Instruction {
  let accounts = accounts::ApproveAddressUpdate {
    upgrade_authority,
    proposal: pda::address_update_proposal(address_field),
    program_data: pda::EXCHANGE_PROGRAM_DATA,
    hylo_exchange: exchange::ID,
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::ApproveAddressUpdate { address_field };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Accepts an approved address update proposal. The new address signs;
/// rent on the proposal account refunds to the current admin.
#[must_use]
pub fn accept_address_update(
  new_address: Pubkey,
  admin: Pubkey,
  address_field: AddressField,
) -> Instruction {
  let accounts = accounts::AcceptAddressUpdate {
    new_address,
    admin,
    hylo: pda::HYLO,
    proposal: pda::address_update_proposal(address_field),
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::AcceptAddressUpdate { address_field };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Cancels an outstanding address update proposal. The admin signs.
#[must_use]
pub fn cancel_address_update(
  admin: Pubkey,
  address_field: AddressField,
) -> Instruction {
  let accounts = accounts::CancelAddressUpdate {
    admin,
    hylo: pda::HYLO,
    proposal: pda::address_update_proposal(address_field),
    event_authority: pda::EXCHANGE_EVENT_AUTHORITY,
    program: exchange::ID,
  };
  let args = args::CancelAddressUpdate { address_field };
  Instruction {
    program_id: exchange::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}
