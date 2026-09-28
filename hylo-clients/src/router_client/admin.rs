use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use hylo_idl::router::instruction_builders;

use super::RouterClient;
use crate::memo::build_memo;
use crate::program_client::VersionedTransactionData;
use crate::squads::{SquadsContext, SquadsTransactionData};

impl RouterClient {
  /// Initializes the registry of routable exo pairs.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn initialize_exo_registry(
    &self,
    squads: &SquadsContext,
  ) -> Result<SquadsTransactionData> {
    let instruction =
      instruction_builders::initialize_exo_registry(squads.vault_pda());
    let memo = build_memo("initialize_exo_registry", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.build_proposal(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::initialize_exo_registry`].
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn initialize_exo_registry_direct(
    &self,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::initialize_exo_registry(self.program.payer());
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Registers the exo pair for the given collateral mint with the router.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn register_exo_entry(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
  ) -> Result<SquadsTransactionData> {
    let instruction = instruction_builders::register_exo_entry(
      squads.vault_pda(),
      collateral_mint,
    );
    let memo = build_memo("register_exo_entry", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.build_proposal(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::register_exo_entry`].
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn register_exo_entry_direct(
    &self,
    collateral_mint: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::register_exo_entry(
      self.program.payer(),
      collateral_mint,
    );
    Ok(VersionedTransactionData::one(instruction))
  }
}
