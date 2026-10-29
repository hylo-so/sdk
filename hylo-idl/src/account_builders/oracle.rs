use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar::instructions;
use anchor_lang::system_program;
use pyth_lazer_solana_contract::{ID_CONST, STORAGE_ID};

use crate::oracle::client::accounts::{
  AcceptAddressUpdate, ApproveAddressUpdate, CancelAddressUpdate,
  InitializeOracle, InitializePriceAggregate, ProposeAddressUpdate,
  UpdateWithPythLazer,
};
use crate::{oracle, pda};

/// Builds the account context for initializing oracle administration.
#[must_use]
pub const fn initialize_oracle(
  admin: Pubkey,
  upgrade_authority: Pubkey,
) -> InitializeOracle {
  InitializeOracle {
    admin,
    upgrade_authority,
    oracle_config: pda::oracle_config(),
    system_program: system_program::ID,
    program_data: pda::ORACLE_PROGRAM_DATA,
    hylo_oracle: oracle::ID,
  }
}

/// Builds the account context for creating the price aggregate for `mint`.
#[must_use]
pub const fn initialize_price_aggregate(
  admin: Pubkey,
  mint: Pubkey,
) -> InitializePriceAggregate {
  InitializePriceAggregate {
    admin,
    price_aggregate: pda::price_aggregate(mint),
    oracle_config: pda::oracle_config(),
    system_program: system_program::ID,
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}

/// Builds the account context for proposing an oracle administrator update.
#[must_use]
pub const fn propose_address_update(
  admin: Pubkey,
  new_address: Pubkey,
) -> ProposeAddressUpdate {
  ProposeAddressUpdate {
    admin,
    oracle_config: pda::oracle_config(),
    proposal: pda::oracle_address_update_proposal(
      oracle::types::AddressField::Admin,
    ),
    new_address,
    system_program: system_program::ID,
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}

/// Builds the account context for approving an oracle administrator update.
#[must_use]
pub const fn approve_address_update(
  upgrade_authority: Pubkey,
  new_address: Pubkey,
) -> ApproveAddressUpdate {
  ApproveAddressUpdate {
    upgrade_authority,
    proposal: pda::oracle_address_update_proposal(
      oracle::types::AddressField::Admin,
    ),
    new_address,
    program_data: pda::ORACLE_PROGRAM_DATA,
    hylo_oracle: oracle::ID,
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}

/// Builds the account context for accepting an oracle administrator update.
#[must_use]
pub const fn accept_address_update(
  new_address: Pubkey,
  admin: Pubkey,
) -> AcceptAddressUpdate {
  AcceptAddressUpdate {
    new_address,
    admin,
    oracle_config: pda::oracle_config(),
    proposal: pda::oracle_address_update_proposal(
      oracle::types::AddressField::Admin,
    ),
    event_authority: pda::ORACLE_EVENT_AUTHORITY,
    program: oracle::ID,
  }
}

/// Builds the account context for cancelling an oracle administrator update.
#[must_use]
pub const fn cancel_address_update(admin: Pubkey) -> CancelAddressUpdate {
  CancelAddressUpdate {
    admin,
    oracle_config: pda::oracle_config(),
    proposal: pda::oracle_address_update_proposal(
      oracle::types::AddressField::Admin,
    ),
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
