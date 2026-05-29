//! Off-chain client for `hylo-trigger-orders`. Permissionless — no admin
//! authority, no Squads wrapping.

use std::sync::Arc;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use hylo_idl::trigger_orders;

use crate::program_client::ProgramClient;

/// Permissionless client for the Hylo trigger-orders program. Manages
/// trigger-order placement, cancellation, and execution. Unlike the admin
/// clients, there is no authority or Squads wrapping — any keypair can drive
/// these operations.
pub struct TriggerOrdersClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for TriggerOrdersClient {
  const PROGRAM_ID: Pubkey = trigger_orders::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> TriggerOrdersClient {
    TriggerOrdersClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

#[cfg(test)]
mod tests {
  use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
  use anchor_client::Cluster;

  use super::*;

  #[test]
  fn client_id_matches_trigger_orders_program_id() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .expect("build");
    assert_eq!(
      <TriggerOrdersClient as ProgramClient>::PROGRAM_ID,
      trigger_orders::ID
    );
    let _ = c.keypair(); // exercise the trait method
  }
}
