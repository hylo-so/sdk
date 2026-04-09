use std::sync::Arc;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::Result;
use hylo_core::idl::exchange;
use hylo_idl::exchange::client::args;
use hylo_idl::exchange::instruction_builders;
use hylo_idl::exchange::types::{TokenMetadata, UFixValue64};

use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::util::{HYLO_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE};

/// Admin client for the Hylo exchange program. Manages LST
/// registration, oracle configuration, fee updates, and protocol
/// stats. User-facing operations go through [`RouterClient`].
pub struct ExchangeClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for ExchangeClient {
  const PROGRAM_ID: Pubkey = exchange::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> ExchangeClient {
    ExchangeClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl ExchangeClient {
  /// Initializes the Hylo exchange protocol.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn initialize_protocol(
    &self,
    upgrade_authority: Pubkey,
    treasury: Pubkey,
    args: &args::InitializeProtocol,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::initialize_protocol(
      self.program.payer(),
      upgrade_authority,
      treasury,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Initializes hyUSD and xSOL token mints.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn initialize_mints(
    &self,
    stablecoin_metadata: TokenMetadata,
    levercoin_metadata: TokenMetadata,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::initialize_mints(
      self.program.payer(),
      stablecoin_metadata,
      levercoin_metadata,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Initializes the LST registry lookup table.
  ///
  /// # Errors
  /// - Failed to get current slot
  /// - Failed to build transaction instructions
  pub fn initialize_lst_registry(
    &self,
    slot: u64,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::initialize_lst_registry(slot, self.program.payer());
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Initializes LST price calculators in registry.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn initialize_lst_registry_calculators(
    &self,
    lst_registry: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::initialize_lst_registry_calculators(
      lst_registry,
      self.program.payer(),
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Registers a new LST for mint/redeem.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  #[allow(clippy::too_many_arguments)]
  pub fn register_lst(
    &self,
    lst_registry: Pubkey,
    lst_mint: Pubkey,
    lst_stake_pool_state: Pubkey,
    sanctum_calculator_program: Pubkey,
    sanctum_calculator_state: Pubkey,
    stake_pool_program: Pubkey,
    stake_pool_program_data: Pubkey,
    rebalance_fee: UFixValue64,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::register_lst(
      lst_mint,
      lst_stake_pool_state,
      sanctum_calculator_program,
      sanctum_calculator_state,
      stake_pool_program,
      stake_pool_program_data,
      lst_registry,
      self.program.payer(),
      rebalance_fee,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Builds transaction data for LST price oracle crank.
  ///
  /// # Errors
  /// - Failed to build transaction data
  pub async fn update_lst_prices(&self) -> Result<VersionedTransactionData> {
    let (remaining_accounts, registry_lut) = self.load_lst_registry().await?;
    let instruction = instruction_builders::update_lst_prices(
      self.program().payer(),
      LST_REGISTRY_LOOKUP_TABLE,
      remaining_accounts,
    );
    let instructions = self
      .program
      .request()
      .instruction(instruction)
      .instructions()?;
    let exchange_lut = self.load_lookup_table(&HYLO_LOOKUP_TABLE).await?;
    let lookup_tables = vec![registry_lut, exchange_lut];
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }

  /// Builds transaction data for harvesting yield from LST vaults to stability
  /// pool.
  ///
  /// # Errors
  /// - Failed to build transaction data
  pub async fn harvest_yield(&self) -> Result<VersionedTransactionData> {
    let (remaining_accounts, registry_lut) = self.load_lst_registry().await?;
    let instruction = instruction_builders::harvest_yield(
      LST_REGISTRY_LOOKUP_TABLE,
      remaining_accounts,
    );
    let instructions = self
      .program()
      .request()
      .instruction(instruction)
      .instructions()?;
    let exchange_lut = self.load_lookup_table(&HYLO_LOOKUP_TABLE).await?;
    let lookup_tables = vec![registry_lut, exchange_lut];
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }

  /// Updates the oracle confidence tolerance.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_oracle_conf_tolerance(
    &self,
    args: &args::UpdateOracleConfTolerance,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_oracle_conf_tolerance(
      self.program.payer(),
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the SOL/USD oracle address.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_sol_usd_oracle(
    &self,
    args: &args::UpdateSolUsdOracle,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_sol_usd_oracle(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the stability pool address.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_stability_pool(
    &self,
    args: &args::UpdateStabilityPool,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_stability_pool(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the LST swap fee.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_lst_swap_fee(
    &self,
    args: &args::UpdateLstSwapFee,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_lst_swap_fee(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }
}
