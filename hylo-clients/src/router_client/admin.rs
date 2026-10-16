use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use hylo_idl::router::instruction_builders;

use super::RouterClient;
use crate::memo::build_memo;
use crate::program_client::VersionedTransactionData;
use crate::signing::{SignedTransaction, SigningClient};

impl RouterClient {
  /// Initializes the registry of routable exo pairs.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_exo_registry(&self) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::initialize_exo_registry(self.signer());
    let memo = build_memo("initialize_exo_registry", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Registers the exo pair for the given collateral mint with the router.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn register_exo_entry(
    &self,
    collateral_mint: Pubkey,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::register_exo_entry(self.signer(), collateral_mint);
    let memo = build_memo("register_exo_entry", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }
}
