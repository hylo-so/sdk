use std::sync::Arc;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::Result;
use hylo_core::idl::exchange;
use hylo_idl::exchange::client::args;
use hylo_idl::exchange::instruction_builders;
use hylo_idl::exchange::types::{AddressField, TokenMetadata, UFixValue64};

use crate::memo::build_memo;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::squads::SquadsContext;
use crate::util::{HYLO_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE};

/// Admin client for the Hylo exchange program. Manages LST
/// registration, oracle configuration, fee updates, and protocol
/// stats. User-facing operations go through
/// [`crate::router_client::RouterClient`].
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
  /// - Failed to wrap inner instruction for Squads
  #[allow(clippy::too_many_arguments)]
  pub fn register_lst(
    &self,
    squads: &SquadsContext,
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
      squads.vault_pda(),
      rebalance_fee,
    );
    let memo = build_memo("register_lst", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
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
    Ok(VersionedTransactionData::new(
      instructions,
      vec![registry_lut],
    ))
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
  /// - Failed to wrap inner instruction for Squads
  pub fn update_oracle_conf_tolerance(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateOracleConfTolerance,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_oracle_conf_tolerance(
      squads.vault_pda(),
      args,
    );
    let memo = build_memo("update_oracle_conf_tolerance", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::update_oracle_conf_tolerance`] signed by
  /// the client's payer.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_oracle_conf_tolerance_direct(
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
  /// - Failed to wrap inner instruction for Squads
  pub fn update_sol_usd_oracle(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateSolUsdOracle,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_sol_usd_oracle(squads.vault_pda(), args);
    let memo = build_memo("update_sol_usd_oracle", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the LST swap fee.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_lst_swap_fee(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateLstSwapFee,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_lst_swap_fee(squads.vault_pda(), args);
    let memo = build_memo("update_lst_swap_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the levercoin fee configuration.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_levercoin_fees(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateLevercoinFees,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_levercoin_fees(squads.vault_pda(), args);
    let memo = build_memo("update_levercoin_fees", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the oracle staleness interval.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_oracle_interval(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateOracleInterval,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_oracle_interval(squads.vault_pda(), args);
    let memo = build_memo("update_oracle_interval", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the LST stablecoin mint threshold.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_lst_stablecoin_mint_threshold(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateLstStablecoinMintThreshold,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_lst_stablecoin_mint_threshold(
        squads.vault_pda(),
        args,
      );
    let memo = build_memo("update_lst_stablecoin_mint_threshold", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the protocol paused state. Signed by the pause authority.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_paused(
    &self,
    pause_authority: Pubkey,
    args: &args::UpdatePaused,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_paused(pause_authority, args);
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the LST buy curve configuration.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_lst_buy_curve_config(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateLstBuyCurveConfig,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_lst_buy_curve_config(
      squads.vault_pda(),
      args,
    );
    let memo = build_memo("update_lst_buy_curve_config", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::update_lst_buy_curve_config`] signed by
  /// the client's payer.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_lst_buy_curve_config_direct(
    &self,
    args: &args::UpdateLstBuyCurveConfig,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_lst_buy_curve_config(
      self.program.payer(),
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the LST sell curve configuration.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_lst_sell_curve_config(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateLstSellCurveConfig,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_lst_sell_curve_config(
      squads.vault_pda(),
      args,
    );
    let memo = build_memo("update_lst_sell_curve_config", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::update_lst_sell_curve_config`] signed by
  /// the client's payer.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_lst_sell_curve_config_direct(
    &self,
    args: &args::UpdateLstSellCurveConfig,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_lst_sell_curve_config(
      self.program.payer(),
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the yield harvest configuration.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_yield_harvest_config(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateYieldHarvestConfig,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_yield_harvest_config(
      squads.vault_pda(),
      args,
    );
    let memo = build_memo("update_yield_harvest_config", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the USDC oracle confidence tolerance.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_usdc_oracle_conf_tolerance(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateUsdcOracleConfTolerance,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_usdc_oracle_conf_tolerance(
      squads.vault_pda(),
      args,
    );
    let memo = build_memo("update_usdc_oracle_conf_tolerance", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the USDC oracle staleness interval.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_usdc_oracle_interval(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateUsdcOracleInterval,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_usdc_oracle_interval(
      squads.vault_pda(),
      args,
    );
    let memo = build_memo("update_usdc_oracle_interval", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the USDC swap fee.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_usdc_swap_fee(
    &self,
    squads: &SquadsContext,
    args: &args::UpdateUsdcSwapFee,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_usdc_swap_fee(squads.vault_pda(), args);
    let memo = build_memo("update_usdc_swap_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the rebalance fee for an LST.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_lst_rebalance_fee(
    &self,
    squads: &SquadsContext,
    lst_mint: Pubkey,
    args: &args::UpdateLstRebalanceFee,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_lst_rebalance_fee(
      squads.vault_pda(),
      lst_mint,
      args,
    );
    let memo = build_memo("update_lst_rebalance_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the funding rate for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_exo_funding_rate(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    args: &args::UpdateExoFundingRate,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_funding_rate(
      squads.vault_pda(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_funding_rate", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the oracle for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_exo_oracle(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracle,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_oracle(
      squads.vault_pda(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_oracle", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the oracle confidence tolerance for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_exo_oracle_conf_tolerance(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracleConfTolerance,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_oracle_conf_tolerance(
      squads.vault_pda(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_oracle_conf_tolerance", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the oracle staleness interval for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_exo_oracle_interval(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracleInterval,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_oracle_interval(
      squads.vault_pda(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_oracle_interval", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the stablecoin mint threshold for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_exo_stablecoin_mint_threshold(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    args: &args::UpdateExoStablecoinMintThreshold,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_exo_stablecoin_mint_threshold(
        squads.vault_pda(),
        collateral_mint,
        args,
      );
    let memo = build_memo("update_exo_stablecoin_mint_threshold", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the buy curve for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_exo_buy_curve(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    args: &args::UpdateExoBuyCurve,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_buy_curve(
      squads.vault_pda(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_buy_curve", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the sell curve for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_exo_sell_curve(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    args: &args::UpdateExoSellCurve,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_sell_curve(
      squads.vault_pda(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_sell_curve", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Updates the levercoin fees for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn update_exo_levercoin_fees(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    args: &args::UpdateExoLevercoinFees,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_levercoin_fees(
      squads.vault_pda(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_levercoin_fees", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Initializes USDC support.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn initialize_usdc(
    &self,
    usdc_usd_pyth_feed: Pubkey,
    args: &args::InitializeUsdc,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::initialize_usdc(
      self.program.payer(),
      usdc_usd_pyth_feed,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Initializes the LST virtual stablecoin.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn initialize_lst_virtual_stablecoin(
    &self,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::initialize_lst_virtual_stablecoin(
      self.program.payer(),
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Registers an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn register_exo(
    &self,
    squads: &SquadsContext,
    collateral_mint: Pubkey,
    exo_usd_pyth_feed: Pubkey,
    args: &args::RegisterExo,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::register_exo(
      squads.vault_pda(),
      collateral_mint,
      exo_usd_pyth_feed,
      args,
    );
    let memo = build_memo("register_exo", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Withdraws accumulated fees to the treasury.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn withdraw_fees(
    &self,
    squads: &SquadsContext,
    treasury: Pubkey,
    fee_token_mint: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::withdraw_fees(
      squads.vault_pda(),
      treasury,
      fee_token_mint,
    );
    let memo = build_memo("withdraw_fees", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Harvests the funding rate for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn harvest_funding_rate(
    &self,
    collateral_mint: Pubkey,
    collateral_usd_pyth_feed: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::harvest_funding_rate(
      collateral_mint,
      collateral_usd_pyth_feed,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Proposes an update to a privileged protocol address. Signed by
  /// the current admin.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn propose_address_update(
    &self,
    squads: &SquadsContext,
    address_field: AddressField,
    new_address: Pubkey,
    ttl_secs: u64,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::propose_address_update(
      squads.vault_pda(),
      address_field,
      new_address,
      ttl_secs,
    );
    let memo = build_memo("propose_address_update", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::propose_address_update`] signed by the
  /// client's payer.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn propose_address_update_direct(
    &self,
    address_field: AddressField,
    new_address: Pubkey,
    ttl_secs: u64,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::propose_address_update(
      self.program.payer(),
      address_field,
      new_address,
      ttl_secs,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Approves an outstanding address update proposal. Signed by the
  /// program upgrade authority.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn approve_address_update(
    &self,
    squads: &SquadsContext,
    address_field: AddressField,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::approve_address_update(
      squads.vault_pda(),
      address_field,
    );
    let memo = build_memo("approve_address_update", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::approve_address_update`] signed by the
  /// client's payer.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn approve_address_update_direct(
    &self,
    address_field: AddressField,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::approve_address_update(
      self.program.payer(),
      address_field,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Accepts an approved address update proposal. Signed by the
  /// incoming address; rent on the proposal account refunds to the
  /// current admin.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn accept_address_update(
    &self,
    squads: &SquadsContext,
    admin: Pubkey,
    address_field: AddressField,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::accept_address_update(
      squads.vault_pda(),
      admin,
      address_field,
    );
    let memo = build_memo("accept_address_update", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::accept_address_update`] signed by the
  /// client's payer.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn accept_address_update_direct(
    &self,
    admin: Pubkey,
    address_field: AddressField,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::accept_address_update(
      self.program.payer(),
      admin,
      address_field,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Cancels an outstanding address update proposal. Signed by the
  /// current admin.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  /// - Failed to wrap inner instruction for Squads
  pub fn cancel_address_update(
    &self,
    squads: &SquadsContext,
    address_field: AddressField,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::cancel_address_update(
      squads.vault_pda(),
      address_field,
    );
    let memo = build_memo("cancel_address_update", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    squads.wrap(&inner, self.program.payer(), memo)
  }

  /// Direct variant of [`Self::cancel_address_update`] signed by the
  /// client's payer.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn cancel_address_update_direct(
    &self,
    address_field: AddressField,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::cancel_address_update(
      self.program.payer(),
      address_field,
    );
    Ok(VersionedTransactionData::one(instruction))
  }
}
