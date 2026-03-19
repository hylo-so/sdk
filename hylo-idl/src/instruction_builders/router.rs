use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};

use crate::router;
use crate::router::client::args;

/// Routes through the proxy program, forwarding the given accounts
/// to the target program via CPI.
#[must_use]
pub fn route<A: ToAccountMetas>(
  args: &args::Route,
  inner_accounts: &A,
) -> Instruction {
  Instruction {
    program_id: router::ID,
    accounts: inner_accounts.to_account_metas(None),
    data: args.data(),
  }
}
