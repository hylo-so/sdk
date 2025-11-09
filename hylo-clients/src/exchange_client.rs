use std::sync::Arc;

use anchor_client::solana_sdk::address_lookup_table::program::ID as LOOKUP_TABLE_PROGRAM;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};
use anyhow::Result;
use fix::prelude::*;
use hylo_core::idl::hylo_exchange::client::{accounts, args};
use hylo_core::idl::hylo_exchange::events::{
  ExchangeStats, MintLevercoinEventV2, MintStablecoinEventV2,
  RedeemLevercoinEventV2, RedeemStablecoinEventV2, SwapLeverToStableEventV1,
  SwapStableToLeverEventV1,
};
use hylo_core::idl::pda::{self, metadata};
use hylo_core::idl::tokens::{TokenMint, HYUSD, JITOSOL, XSOL};
use hylo_core::idl::{ata, hylo_exchange, hylo_stability_pool};
use hylo_core::pyth::SOL_USD_PYTH_FEED;
use solana_address_lookup_table_interface::program as address_lookup_table;

use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::transaction::{
  BuildTransactionData, MintArgs, RedeemArgs, SimulatePrice, SwapArgs,
  TransactionSyntax,
};
use crate::util::{
  user_ata_instruction, EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE,
  TREASURY, UPGRADE_AUTHORITY,
};

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
  const PROGRAM_ID: Pubkey = hylo_exchange::ID;

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
    args: args::InitializeProtocol,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::InitializeProtocol {
      admin: self.program.payer(),
      upgrade_authority: UPGRADE_AUTHORITY,
      hylo: *pda::HYLO,
      treasury: TREASURY,
      system_program: system_program::ID,
      program_data: pda::program_data(hylo_exchange::ID),
      hylo_exchange: hylo_exchange::ID,
    };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    Ok(VersionedTransactionData::no_lookup(instructions))
  }

  /// Initializes hyUSD and xSOL token mints.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn initialize_mints(&self) -> Result<VersionedTransactionData> {
    let accounts = accounts::InitializeMints {
      admin: self.program.payer(),
      hylo: *pda::HYLO,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      stablecoin_mint: HYUSD::MINT,
      levercoin_mint: XSOL::MINT,
      stablecoin_metadata: metadata(HYUSD::MINT),
      levercoin_metadata: metadata(XSOL::MINT),
      metadata_program: mpl_token_metadata::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      system_program: system_program::ID,
    };
    let args = args::InitializeMints {};
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    Ok(VersionedTransactionData::no_lookup(instructions))
  }

  /// Initializes the LST registry lookup table.
  ///
  /// # Errors
  /// - Failed to get current slot
  /// - Failed to build transaction instructions
  pub async fn initialize_lst_registry(
    &self,
  ) -> Result<VersionedTransactionData> {
    let slot = self.program().rpc().get_slot().await?;
    let accounts = accounts::InitializeLstRegistry {
      admin: self.program.payer(),
      hylo: *pda::HYLO,
      registry_auth: *pda::LST_REGISTRY_AUTH,
      lst_registry: pda::new_lst_registry(slot),
      lut_program: address_lookup_table::ID,
      system_program: system_program::ID,
    };
    let args = args::InitializeLstRegistry { slot };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    Ok(VersionedTransactionData::no_lookup(instructions))
  }

  /// Initializes LST price calculators in registry.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn initialize_lst_registry_calculators(
    &self,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::InitializeLstRegistryCalculators {
      admin: self.program.payer(),
      hylo: *pda::HYLO,
      lst_registry_auth: *pda::LST_REGISTRY_AUTH,
      lst_registry: LST_REGISTRY_LOOKUP_TABLE,
      lut_program: address_lookup_table::ID,
      system_program: system_program::ID,
    };
    let args = args::InitializeLstRegistryCalculators {};
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    Ok(VersionedTransactionData::no_lookup(instructions))
  }

  /// Registers a new LST for mint/redeem.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn register_lst(
    &self,
    lst_mint: Pubkey,
    lst_stake_pool_state: Pubkey,
    sanctum_calculator_program: Pubkey,
    sanctum_calculator_state: Pubkey,
    stake_pool_program: Pubkey,
    stake_pool_program_data: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let args = args::RegisterLst {};
    let accounts = accounts::RegisterLst {
      admin: self.program.payer(),
      hylo: *pda::HYLO,
      lst_header: pda::lst_header(lst_mint),
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      registry_auth: *pda::LST_REGISTRY_AUTH,
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_mint,
      lst_registry: LST_REGISTRY_LOOKUP_TABLE,
      lst_stake_pool_state,
      sanctum_calculator_program,
      sanctum_calculator_state,
      stake_pool_program_data,
      stake_pool_program,
      lut_program: address_lookup_table::ID,
      associated_token_program: associated_token::ID,
      token_program: token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
    };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    Ok(VersionedTransactionData::no_lookup(instructions))
  }

  /// Builds transaction data for LST price oracle crank.
  ///
  /// # Errors
  /// - Failed to build transaction data
  pub async fn update_lst_prices(&self) -> Result<VersionedTransactionData> {
    let accounts = accounts::UpdateLstPrices {
      payer: self.program.payer(),
      hylo: *pda::HYLO,
      lst_registry: LST_REGISTRY_LOOKUP_TABLE,
      lut_program: LOOKUP_TABLE_PROGRAM,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
    };
    let args = args::UpdateLstPrices {};
    let (remaining_accounts, registry_lut) = self.load_lst_registry().await?;
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .accounts(remaining_accounts)
      .args(args)
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
    let accounts = accounts::HarvestYield {
      payer: self.program.payer(),
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD::MINT,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_mint: XSOL::MINT,
      levercoin_auth: *pda::XSOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      pool_auth: *pda::POOL_AUTH,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_stability_pool: hylo_stability_pool::ID,
      lst_registry: LST_REGISTRY_LOOKUP_TABLE,
      lut_program: LOOKUP_TABLE_PROGRAM,
      associated_token_program: associated_token::ID,
      token_program: token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
      stablecoin_fee_auth: pda::fee_auth(HYUSD::MINT),
      stablecoin_fee_vault: pda::fee_vault(HYUSD::MINT),
      levercoin_fee_auth: pda::fee_auth(XSOL::MINT),
      levercoin_fee_vault: pda::fee_vault(XSOL::MINT),
    };
    let args = args::HarvestYield {};
    let (remaining_accounts, registry_lut) = self.load_lst_registry().await?;
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .accounts(remaining_accounts)
      .args(args)
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
}

#[async_trait::async_trait]
impl BuildTransactionData<HYUSD, JITOSOL> for ExchangeClient {
  type Inputs = RedeemArgs;

  async fn build(
    &self,
    RedeemArgs {
      amount,
      user,
      slippage_config,
    }: RedeemArgs,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::RedeemStablecoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(JITOSOL::MINT),
      vault_auth: pda::vault_auth(JITOSOL::MINT),
      fee_vault: pda::fee_vault(JITOSOL::MINT),
      lst_vault: pda::vault(JITOSOL::MINT),
      lst_header: pda::lst_header(JITOSOL::MINT),
      user_stablecoin_ta: pda::hyusd_ata(user),
      user_lst_ta: ata!(user, JITOSOL::MINT),
      stablecoin_mint: HYUSD::MINT,
      lst_mint: JITOSOL::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
    };
    let args = args::RedeemStablecoin {
      amount_to_redeem: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let ata = vec![user_ata_instruction(&user, &JITOSOL::MINT)];
    let program = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let instructions = [ata, program].concat();
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl SimulatePrice<HYUSD, JITOSOL> for ExchangeClient {
  type OutExp = N9;
  type Event = RedeemStablecoinEventV2;
  fn from_event(e: &Self::Event) -> Result<UFix64<N9>> {
    Ok(UFix64::new(e.collateral_withdrawn.bits))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<XSOL, JITOSOL> for ExchangeClient {
  type Inputs = RedeemArgs;

  async fn build(
    &self,
    RedeemArgs {
      amount,
      user,
      slippage_config,
    }: RedeemArgs,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::RedeemLevercoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(JITOSOL::MINT),
      vault_auth: pda::vault_auth(JITOSOL::MINT),
      fee_vault: pda::fee_vault(JITOSOL::MINT),
      lst_vault: pda::vault(JITOSOL::MINT),
      lst_header: pda::lst_header(JITOSOL::MINT),
      user_levercoin_ta: pda::xsol_ata(user),
      user_lst_ta: ata!(user, JITOSOL::MINT),
      levercoin_mint: XSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      lst_mint: JITOSOL::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
    };
    let args = args::RedeemLevercoin {
      amount_to_redeem: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let ata = vec![user_ata_instruction(&user, &JITOSOL::MINT)];
    let program = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let instructions = [ata, program].concat();
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl SimulatePrice<XSOL, JITOSOL> for ExchangeClient {
  type OutExp = N9;
  type Event = RedeemLevercoinEventV2;
  fn from_event(e: &Self::Event) -> Result<UFix64<N9>> {
    Ok(UFix64::new(e.collateral_withdrawn.bits))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<JITOSOL, HYUSD> for ExchangeClient {
  type Inputs = MintArgs;

  async fn build(
    &self,
    MintArgs {
      amount,
      user,
      slippage_config,
    }: MintArgs,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::MintStablecoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(JITOSOL::MINT),
      vault_auth: pda::vault_auth(JITOSOL::MINT),
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_vault: pda::fee_vault(JITOSOL::MINT),
      lst_vault: pda::vault(JITOSOL::MINT),
      lst_header: pda::lst_header(JITOSOL::MINT),
      user_lst_ta: ata!(user, JITOSOL::MINT),
      user_stablecoin_ta: pda::hyusd_ata(user),
      lst_mint: JITOSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
    };
    let args = args::MintStablecoin {
      amount_lst_to_deposit: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let ata = vec![user_ata_instruction(&user, &HYUSD::MINT)];
    let program = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let instructions = [ata, program].concat();
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl SimulatePrice<JITOSOL, HYUSD> for ExchangeClient {
  type OutExp = N6;
  type Event = MintStablecoinEventV2;
  fn from_event(e: &Self::Event) -> Result<UFix64<N6>> {
    Ok(UFix64::new(e.minted.bits))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<JITOSOL, XSOL> for ExchangeClient {
  type Inputs = MintArgs;

  async fn build(
    &self,
    MintArgs {
      amount,
      user,
      slippage_config,
    }: MintArgs,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::MintLevercoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(JITOSOL::MINT),
      vault_auth: pda::vault_auth(JITOSOL::MINT),
      levercoin_auth: *pda::XSOL_AUTH,
      fee_vault: pda::fee_vault(JITOSOL::MINT),
      lst_vault: pda::vault(JITOSOL::MINT),
      lst_header: pda::lst_header(JITOSOL::MINT),
      user_lst_ta: ata!(user, JITOSOL::MINT),
      user_levercoin_ta: pda::xsol_ata(user),
      lst_mint: JITOSOL::MINT,
      levercoin_mint: XSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
    };
    let args = args::MintLevercoin {
      amount_lst_to_deposit: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let ata = vec![user_ata_instruction(&user, &XSOL::MINT)];
    let program = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let instructions = [ata, program].concat();
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

impl SimulatePrice<JITOSOL, XSOL> for ExchangeClient {
  type OutExp = N6;
  type Event = MintLevercoinEventV2;
  fn from_event(e: &Self::Event) -> Result<UFix64<N6>> {
    Ok(UFix64::new(e.minted.bits))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<HYUSD, XSOL> for ExchangeClient {
  type Inputs = SwapArgs;

  async fn build(
    &self,
    SwapArgs {
      amount,
      user,
      slippage_config,
    }: SwapArgs,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::SwapStableToLever {
      user,
      hylo: *pda::HYLO,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_mint: HYUSD::MINT,
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_auth: pda::fee_auth(HYUSD::MINT),
      fee_vault: pda::fee_vault(HYUSD::MINT),
      user_stablecoin_ta: pda::hyusd_ata(user),
      levercoin_mint: XSOL::MINT,
      levercoin_auth: *pda::XSOL_AUTH,
      user_levercoin_ta: pda::xsol_ata(user),
      token_program: token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
    };
    let args = args::SwapStableToLever {
      amount_stablecoin: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let ata = vec![user_ata_instruction(&user, &XSOL::MINT)];
    let program = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let instructions = [ata, program].concat();
    let lookup_tables =
      vec![self.load_lookup_table(&EXCHANGE_LOOKUP_TABLE).await?];
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

  async fn build(
    &self,
    SwapArgs {
      amount,
      user,
      slippage_config,
    }: SwapArgs,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::SwapLeverToStable {
      user,
      hylo: *pda::HYLO,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_mint: HYUSD::MINT,
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_auth: pda::fee_auth(HYUSD::MINT),
      fee_vault: pda::fee_vault(HYUSD::MINT),
      user_stablecoin_ta: pda::hyusd_ata(user),
      levercoin_mint: XSOL::MINT,
      levercoin_auth: *pda::XSOL_AUTH,
      user_levercoin_ta: pda::xsol_ata(user),
      token_program: token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: hylo_exchange::ID,
    };
    let args = args::SwapLeverToStable {
      amount_levercoin: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let ata = vec![user_ata_instruction(&user, &HYUSD::MINT)];
    let program = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let instructions = [ata, program].concat();
    let lookup_tables =
      vec![self.load_lookup_table(&EXCHANGE_LOOKUP_TABLE).await?];
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
