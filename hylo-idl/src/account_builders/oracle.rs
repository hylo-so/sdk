use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar::instructions;
use anchor_lang::system_program;

use crate::oracle::client::accounts::{
  AddPythLazerFeed, InitializePriceAggregate, UpdateObservationsWithPythLazer,
};
use crate::pda::{PYTH_LAZER_PROGRAM, PYTH_LAZER_STORAGE};
use crate::{oracle, pda};

/// Builds the account context for creating the price aggregate for `mint`.
#[must_use]
pub const fn initialize_price_aggregate(
  admin: Pubkey,
  mint: Pubkey,
) -> InitializePriceAggregate {
  InitializePriceAggregate {
    admin,
    hylo: pda::HYLO,
    price_aggregate: pda::price_aggregate(mint),
    system_program: system_program::ID,
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}

/// Builds the account context for adding a Pyth Lazer feed to the price
/// aggregate for `mint`.
#[must_use]
pub const fn add_pyth_lazer_feed(
  admin: Pubkey,
  mint: Pubkey,
) -> AddPythLazerFeed {
  AddPythLazerFeed {
    admin,
    hylo: pda::HYLO,
    price_aggregate: pda::price_aggregate(mint),
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}

/// Builds the fixed account context for a Pyth Lazer update.
#[must_use]
pub const fn update_observations_with_pyth_lazer(
  payer: Pubkey,
  pyth_lazer_treasury: Pubkey,
) -> UpdateObservationsWithPythLazer {
  UpdateObservationsWithPythLazer {
    payer,
    pyth_lazer_storage: PYTH_LAZER_STORAGE,
    pyth_lazer_treasury,
    pyth_lazer_program: PYTH_LAZER_PROGRAM,
    system_program: system_program::ID,
    instructions_sysvar: instructions::ID,
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}
