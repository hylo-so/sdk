mod instructions;
mod transaction_data;

use std::sync::Arc;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use async_trait::async_trait;

use crate::program_client::ProgramClient;
use crate::transaction::TransactionSyntax;

/// Builds and executes transactions through the Hylo router program.
/// Handles all user-facing token operations: mint, redeem, swap, and
/// stability pool deposit/withdraw.
pub struct RouterClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for RouterClient {
  const PROGRAM_ID: Pubkey = hylo_idl::router::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> RouterClient {
    RouterClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

#[async_trait]
impl TransactionSyntax for RouterClient {}
