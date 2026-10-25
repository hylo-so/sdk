use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar::instructions;
use anchor_lang::system_program;
use pyth_lazer_solana_contract::{ID_CONST, STORAGE_ID};

use crate::oracle::client::accounts::{
  InitializePriceAggregate, UpdateWithPythLazer,
};
use crate::{oracle, pda};

/// Builds the account context for creating the price aggregate for `mint`.
#[must_use]
pub const fn initialize_price_aggregate(
  payer: Pubkey,
  mint: Pubkey,
) -> InitializePriceAggregate {
  InitializePriceAggregate {
    payer,
    price_aggregate: pda::price_aggregate(mint),
    system_program: system_program::ID,
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}

/// Builds the fixed account context for a Pyth Lazer update.
#[must_use]
pub const fn update_with_pyth_lazer(
  payer: Pubkey,
  pyth_lazer_treasury: Pubkey,
) -> UpdateWithPythLazer {
  UpdateWithPythLazer {
    payer,
    pyth_lazer_program: ID_CONST,
    pyth_lazer_storage: STORAGE_ID,
    pyth_lazer_treasury,
    system_program: system_program::ID,
    instructions_sysvar: instructions::ID,
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}
