use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::{InstructionData, ToAccountMetas};

use crate::oracle::account_builders;
use crate::oracle::client::args;
use crate::{oracle, pda};

/// Builds the instruction that creates the price aggregate for `mint`.
#[must_use]
pub fn initialize_price_aggregate(
  payer: Pubkey,
  mint: Pubkey,
  id: u16,
) -> Instruction {
  let accounts = account_builders::initialize_price_aggregate(payer, mint);
  let args = args::InitializePriceAggregate { id };
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
