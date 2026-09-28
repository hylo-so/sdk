use anchor_lang::AccountDeserialize;
use anyhow::{Context, Result};
use hylo_idl::pda;
use hylo_idl::router::accounts::ExoRegistry;

use super::RouterClient;
use crate::program_client::ProgramClient;

impl RouterClient {
  /// Fetches the registry of routable exo pairs.
  ///
  /// # Errors
  /// * Failed to fetch the account
  /// * Failed to deserialize account data
  pub async fn exo_registry(&self) -> Result<ExoRegistry> {
    let account = self.program().rpc().get_account(&pda::EXO_REGISTRY).await?;
    ExoRegistry::try_deserialize(&mut account.data.as_slice())
      .context("Exo registry deserialization")
  }
}
