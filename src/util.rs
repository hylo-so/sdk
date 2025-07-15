use std::rc::Rc;

use anchor_client::solana_client::rpc_config::RpcSimulateTransactionConfig;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{Client, Cluster, Program};
use anchor_lang::prelude::Pubkey;
use anyhow::Result;

/// Default configuration to use in simulated transactions.
#[must_use]
pub fn simulation_config() -> RpcSimulateTransactionConfig {
  RpcSimulateTransactionConfig {
    sig_verify: false,
    replace_recent_blockhash: true,
    commitment: Some(CommitmentConfig::confirmed()),
    ..Default::default()
  }
}

/// Abstracts the construction of client structs with `anchor_client::Program`.
pub trait ProgramClient: Sized {
  const PROGRAM_ID: Pubkey;

  fn build_client(program: Program<Rc<Keypair>>) -> Self;

  /// Constructs the given client with ID `Self::PROGRAM_ID`.
  ///
  /// # Errors
  /// - Underlying Anchor program creation
  fn new_from_keypair(
    cluster: Cluster,
    keypair: Keypair,
    config: CommitmentConfig,
  ) -> Result<Self> {
    let client = Client::new_with_options(cluster, Rc::new(keypair), config);
    let program = client.program(Self::PROGRAM_ID)?;
    Ok(Self::build_client(program))
  }
}
