use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::{InstructionData, ToAccountMetas};

use crate::oracle::account_builders;
use crate::oracle::client::args;
use crate::{oracle, pda};

/// Builds the instruction that initializes oracle administration.
#[must_use]
pub fn initialize_oracle(
  admin: Pubkey,
  upgrade_authority: Pubkey,
) -> Instruction {
  let accounts =
    account_builders::initialize_oracle(admin, upgrade_authority);
  Instruction {
    program_id: oracle::ID,
    accounts: accounts.to_account_metas(None),
    data: args::InitializeOracle {}.data(),
  }
}

/// Builds the instruction that proposes an oracle administrator update.
#[must_use]
pub fn propose_address_update(
  admin: Pubkey,
  address_field: oracle::types::AddressField,
  new_address: Pubkey,
  ttl_secs: u64,
) -> Instruction {
  let accounts = account_builders::propose_address_update(admin, new_address);
  let args = args::ProposeAddressUpdate {
    address_field,
    ttl_secs,
  };
  Instruction {
    program_id: oracle::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Builds the instruction that approves an oracle administrator update.
#[must_use]
pub fn approve_address_update(
  upgrade_authority: Pubkey,
  new_address: Pubkey,
  address_field: oracle::types::AddressField,
) -> Instruction {
  let accounts =
    account_builders::approve_address_update(upgrade_authority, new_address);
  let args = args::ApproveAddressUpdate { address_field };
  Instruction {
    program_id: oracle::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Builds the instruction that accepts an oracle administrator update.
#[must_use]
pub fn accept_address_update(
  new_address: Pubkey,
  admin: Pubkey,
  address_field: oracle::types::AddressField,
) -> Instruction {
  let accounts = account_builders::accept_address_update(new_address, admin);
  let args = args::AcceptAddressUpdate { address_field };
  Instruction {
    program_id: oracle::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Builds the instruction that cancels an oracle administrator update.
#[must_use]
pub fn cancel_address_update(
  admin: Pubkey,
  address_field: oracle::types::AddressField,
) -> Instruction {
  let accounts = account_builders::cancel_address_update(admin);
  let args = args::CancelAddressUpdate { address_field };
  Instruction {
    program_id: oracle::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Builds the instruction that creates the price aggregate for `mint`.
#[must_use]
pub fn initialize_price_aggregate(admin: Pubkey, mint: Pubkey) -> Instruction {
  let accounts = account_builders::initialize_price_aggregate(admin, mint);
  let args = args::InitializePriceAggregate { mint };
  Instruction {
    program_id: oracle::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Builds the Pyth Lazer update instruction for `mints` in payload order.
/// Each mint appends its price aggregate as a writable account.
#[must_use]
pub fn update_with_pyth_lazer(
  payer: Pubkey,
  pyth_lazer_treasury: Pubkey,
  mints: &[Pubkey],
  args: &args::UpdateWithPythLazer,
) -> Instruction {
  let accounts =
    account_builders::update_with_pyth_lazer(payer, pyth_lazer_treasury);
  let price_aggregates = mints
    .iter()
    .map(|mint| AccountMeta::new(pda::price_aggregate(*mint), false));
  Instruction {
    program_id: oracle::ID,
    accounts: accounts
      .to_account_metas(None)
      .into_iter()
      .chain(price_aggregates)
      .collect(),
    data: args.data(),
  }
}
