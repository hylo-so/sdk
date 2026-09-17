use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};

use crate::router;
use crate::router::account_builders;
use crate::router::client::args;

/// Initializes the canonical EXO registry.
#[must_use]
pub fn initialize_exo_registry(admin: Pubkey) -> Instruction {
  let accounts = account_builders::initialize_exo_registry(admin);
  let args = args::InitializeExoRegistry {};
  Instruction {
    program_id: router::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Registers a collateral/levercoin EXO pair with the router.
#[must_use]
pub fn register_exo_entry(
  admin: Pubkey,
  collateral_mint: Pubkey,
) -> Instruction {
  let accounts = account_builders::register_exo_entry(admin, collateral_mint);
  let args = args::RegisterExoEntry {};
  Instruction {
    program_id: router::ID,
    accounts: accounts.to_account_metas(None),
    data: args.data(),
  }
}

/// Routes through the proxy program, forwarding the given accounts
/// to the target program via CPI.
#[must_use]
pub fn route<A: ToAccountMetas>(
  args: &args::Route,
  inner_accounts: &A,
) -> Instruction {
  Instruction {
    program_id: router::ID,
    accounts: account_builders::route()
      .to_account_metas(None)
      .into_iter()
      .chain(inner_accounts.to_account_metas(None))
      .collect(),
    data: args.data(),
  }
}
