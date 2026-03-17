use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};

use crate::router;
use crate::router::account_builders;
use crate::router::client::args;

/// Routes through the proxy program, forwarding the given accounts
/// to the target program via CPI.
#[must_use]
pub fn route<A: ToAccountMetas>(
  args: &args::Route,
  inner_accounts: &A,
) -> Instruction {
  let accounts = account_builders::route();
  Instruction {
    program_id: router::ID,
    accounts: [
      accounts.to_account_metas(None),
      inner_accounts.to_account_metas(None),
    ]
    .concat(),
    data: args.data(),
  }
}
