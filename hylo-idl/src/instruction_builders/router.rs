use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};

use crate::router;
use crate::router::account_builders;
use crate::router::client::args;

/// Wraps an exchange or stability pool instruction, appending its
/// accounts as remaining accounts for the router CPI.
#[must_use]
pub fn route(inner_ix: &Instruction, args: &args::Route) -> Instruction {
  let accounts = account_builders::route();
  Instruction {
    program_id: router::ID,
    accounts: [accounts.to_account_metas(None), inner_ix.accounts.clone()]
      .concat(),
    data: args.data(),
  }
}
