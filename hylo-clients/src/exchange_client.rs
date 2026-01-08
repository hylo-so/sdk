use std::sync::Arc;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::Result;
use fix::prelude::*;
use hylo_core::idl::tokens::{TokenMint, HYUSD, XSOL};
use hylo_core::idl::{exchange, pda};
use hylo_core::pyth::SOL_USD_PYTH_FEED;
use hylo_idl::exchange::client::{accounts, args};
use hylo_idl::exchange::events::{
  ExchangeStats, MintLevercoinEventV2, MintStablecoinEventV2,
  RedeemLevercoinEventV2, RedeemStablecoinEventV2, SwapLeverToStableEventV1,
  SwapStableToLeverEventV1,
};
use hylo_idl::exchange::instruction_builders;

use crate::instructions::ExchangeInstructionBuilder;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::syntax_helpers::InstructionBuilderExt;
use crate::transaction::{
  BuildTransactionData, MintArgs, RedeemArgs, SimulatePrice, SwapArgs,
  TransactionSyntax,
};
use crate::util::{EXCHANGE_LOOKUP_TABLE, LST, LST_REGISTRY_LOOKUP_TABLE};

/// Client for interacting with the Hylo Exchange program.
///
/// Provides functionality for minting/redeem/swap between hyUSD and xSOL and
/// LST collateral. Supports transaction execution and price simulation for
/// offchain quoting.
///
/// # Examples
///
/// ## Setup
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # fn setup_client() -> Result<ExchangeClient> {
/// let client = ExchangeClient::new_random_keypair(
///   Cluster::Mainnet,
///   CommitmentConfig::confirmed(),
/// )?;
/// # Ok(client)
/// # }
/// ```
///
/// ## Transaction Execution
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # async fn execute_transaction(client: ExchangeClient) -> Result<Signature> {
/// // Mint JITOSOL → hyUSD
/// let user = Pubkey::new_unique();
/// let signature = client.run_transaction::<JITOSOL, HYUSD>(MintArgs {
///   amount: UFix64::one(),
///   user,
///   slippage_config: None,
/// }).await?;
/// # Ok(signature)
/// # }
/// ```
///
/// ## Transaction Building
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # async fn build_transaction(client: ExchangeClient) -> Result<()> {
/// let user = Pubkey::new_unique();
///
/// // Build transaction data without executing
/// let tx_data = client.build_transaction_data::<JITOSOL, HYUSD>(MintArgs {
///   amount: UFix64::new(50),
///   user,
///   slippage_config: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Price Quote
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # async fn simulate_price(client: ExchangeClient) -> Result<()> {
/// // Get price quote for 1 JITOSOL → hyUSD
/// let price = client.quote::<JITOSOL, HYUSD>().await?;
/// # Ok(())
/// # }
/// ```
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
  pub fn initialize_mints(&self) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::initialize_mints(self.program.payer());
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
    let exchange_lut = self.load_lookup_table(&EXCHANGE_LOOKUP_TABLE).await?;
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
      self.program.payer(),
      LST_REGISTRY_LOOKUP_TABLE,
      remaining_accounts,
    );
    let instructions = self
      .program()
      .request()
      .instruction(instruction)
      .instructions()?;
    let exchange_lut = self.load_lookup_table(&EXCHANGE_LOOKUP_TABLE).await?;
    let lookup_tables = vec![registry_lut, exchange_lut];
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }

  /// Gets exchange stats via RPC simulation.
  ///
  /// # Errors
  /// - Failed to simulate transaction
  /// - Failed to deserialize return data
  pub async fn get_stats(&self) -> Result<ExchangeStats> {
    let accounts = accounts::GetStats {
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD::MINT,
      levercoin_mint: XSOL::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    };
    let args = args::GetStats {};
    let tx = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .signed_transaction()
      .await?;
    let stats = self.simulate_transaction_return(tx.into()).await?;
    Ok(stats)
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
}

#[async_trait::async_trait]
impl<OUT: LST> BuildTransactionData<HYUSD, OUT> for ExchangeClient {
  type Inputs = RedeemArgs;

  async fn build(
    &self,
    inputs: RedeemArgs,
  ) -> Result<VersionedTransactionData> {
    let instructions =
      ExchangeInstructionBuilder::build_instructions::<HYUSD, OUT>(inputs)?;
    let lookup_tables = self
      .load_multiple_lookup_tables(ExchangeInstructionBuilder::lookup_tables::<
        HYUSD,
        OUT,
      >())
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl<OUT: LST> SimulatePrice<HYUSD, OUT> for ExchangeClient {
  type OutExp = N9;
  type Event = RedeemStablecoinEventV2;
  fn from_event(e: &Self::Event) -> Result<UFix64<N9>> {
    Ok(UFix64::new(e.collateral_withdrawn.bits))
  }
}

#[async_trait::async_trait]
impl<OUT: TokenMint + LST> BuildTransactionData<XSOL, OUT> for ExchangeClient {
  type Inputs = RedeemArgs;

  async fn build(
    &self,
    inputs: RedeemArgs,
  ) -> Result<VersionedTransactionData> {
    let instructions =
      ExchangeInstructionBuilder::build_instructions::<XSOL, OUT>(inputs)?;
    let lookup_tables = self
      .load_multiple_lookup_tables(ExchangeInstructionBuilder::lookup_tables::<
        XSOL,
        OUT,
      >())
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl<OUT: LST> SimulatePrice<XSOL, OUT> for ExchangeClient {
  type OutExp = N9;
  type Event = RedeemLevercoinEventV2;
  fn from_event(e: &Self::Event) -> Result<UFix64<N9>> {
    Ok(UFix64::new(e.collateral_withdrawn.bits))
  }
}

#[async_trait::async_trait]
impl<IN: LST> BuildTransactionData<IN, HYUSD> for ExchangeClient {
  type Inputs = MintArgs;

  async fn build(&self, inputs: MintArgs) -> Result<VersionedTransactionData> {
    let instructions =
      ExchangeInstructionBuilder::build_instructions::<IN, HYUSD>(inputs)?;
    let lookup_tables = self
      .load_multiple_lookup_tables(ExchangeInstructionBuilder::lookup_tables::<
        IN,
        HYUSD,
      >())
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl<IN: LST> SimulatePrice<IN, HYUSD> for ExchangeClient {
  type OutExp = N6;
  type Event = MintStablecoinEventV2;
  fn from_event(e: &Self::Event) -> Result<UFix64<N6>> {
    Ok(UFix64::new(e.minted.bits))
  }
}

#[async_trait::async_trait]
impl<IN: LST> BuildTransactionData<IN, XSOL> for ExchangeClient {
  type Inputs = MintArgs;

  async fn build(&self, inputs: MintArgs) -> Result<VersionedTransactionData> {
    let instructions =
      ExchangeInstructionBuilder::build_instructions::<IN, XSOL>(inputs)?;
    let lookup_tables = self
      .load_multiple_lookup_tables(ExchangeInstructionBuilder::lookup_tables::<
        IN,
        XSOL,
      >())
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl<IN: LST> SimulatePrice<IN, XSOL> for ExchangeClient {
  type OutExp = N6;
  type Event = MintLevercoinEventV2;
  fn from_event(e: &Self::Event) -> Result<UFix64<N6>> {
    Ok(UFix64::new(e.minted.bits))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<HYUSD, XSOL> for ExchangeClient {
  type Inputs = SwapArgs;

  async fn build(&self, inputs: SwapArgs) -> Result<VersionedTransactionData> {
    let instructions =
      ExchangeInstructionBuilder::build_instructions::<HYUSD, XSOL>(inputs)?;
    let lookup_tables = self
      .load_multiple_lookup_tables(ExchangeInstructionBuilder::lookup_tables::<
        HYUSD,
        XSOL,
      >())
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl SimulatePrice<HYUSD, XSOL> for ExchangeClient {
  type OutExp = N6;
  type Event = SwapStableToLeverEventV1;
  fn from_event(e: &Self::Event) -> Result<UFix64<N6>> {
    Ok(UFix64::new(e.levercoin_minted.bits))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<XSOL, HYUSD> for ExchangeClient {
  type Inputs = SwapArgs;

  async fn build(&self, inputs: SwapArgs) -> Result<VersionedTransactionData> {
    let instructions =
      ExchangeInstructionBuilder::build_instructions::<XSOL, HYUSD>(inputs)?;
    let lookup_tables = self
      .load_multiple_lookup_tables(ExchangeInstructionBuilder::lookup_tables::<
        XSOL,
        HYUSD,
      >())
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl SimulatePrice<XSOL, HYUSD> for ExchangeClient {
  type OutExp = N6;
  type Event = SwapLeverToStableEventV1;
  fn from_event(e: &Self::Event) -> Result<UFix64<N6>> {
    Ok(UFix64::new(e.stablecoin_minted_user.bits))
  }
}

#[async_trait::async_trait]
impl TransactionSyntax for ExchangeClient {}
