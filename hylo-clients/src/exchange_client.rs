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

  /// Updates the levercoin fee configuration.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_levercoin_fees(
    &self,
    args: &args::UpdateLevercoinFees,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_levercoin_fees(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the oracle staleness interval.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_oracle_interval(
    &self,
    args: &args::UpdateOracleInterval,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_oracle_interval(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the LST stablecoin mint threshold.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_lst_stablecoin_mint_threshold(
    &self,
    args: &args::UpdateLstStablecoinMintThreshold,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_lst_stablecoin_mint_threshold(
        self.program.payer(),
        args,
      );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the protocol paused state.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_paused(
    &self,
    args: &args::UpdatePaused,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_paused(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the LST buy curve configuration.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_lst_buy_curve_config(
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
  pub fn update_lst_sell_curve_config(
    &self,
    args: &args::UpdateLstSellCurveConfig,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_lst_sell_curve_config(
      self.program.payer(),
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the treasury address.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_treasury(
    &self,
    args: &args::UpdateTreasury,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_treasury(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the yield harvest configuration.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_yield_harvest_config(
    &self,
    args: &args::UpdateYieldHarvestConfig,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_yield_harvest_config(
      self.program.payer(),
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the USDC oracle confidence tolerance.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_usdc_oracle_conf_tolerance(
    &self,
    args: &args::UpdateUsdcOracleConfTolerance,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_usdc_oracle_conf_tolerance(
      self.program.payer(),
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the USDC oracle staleness interval.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_usdc_oracle_interval(
    &self,
    args: &args::UpdateUsdcOracleInterval,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_usdc_oracle_interval(
      self.program.payer(),
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the USDC swap fee.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_usdc_swap_fee(
    &self,
    args: &args::UpdateUsdcSwapFee,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_usdc_swap_fee(self.program.payer(), args);
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the protocol admin.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_admin(
    &self,
    upgrade_authority: Pubkey,
    args: &args::UpdateAdmin,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_admin(
      self.program.payer(),
      upgrade_authority,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the rebalance fee for an LST.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_lst_rebalance_fee(
    &self,
    lst_mint: Pubkey,
    args: &args::UpdateLstRebalanceFee,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_lst_rebalance_fee(
      self.program.payer(),
      lst_mint,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the funding rate for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_exo_funding_rate(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoFundingRate,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_funding_rate(
      self.program.payer(),
      collateral_mint,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the oracle for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_exo_oracle(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracle,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_oracle(
      self.program.payer(),
      collateral_mint,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the oracle confidence tolerance for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_exo_oracle_conf_tolerance(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracleConfTolerance,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_oracle_conf_tolerance(
      self.program.payer(),
      collateral_mint,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the oracle staleness interval for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_exo_oracle_interval(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracleInterval,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_oracle_interval(
      self.program.payer(),
      collateral_mint,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the stablecoin mint threshold for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_exo_stablecoin_mint_threshold(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoStablecoinMintThreshold,
  ) -> Result<VersionedTransactionData> {
    let instruction =
      instruction_builders::update_exo_stablecoin_mint_threshold(
        self.program.payer(),
        collateral_mint,
        args,
      );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the buy curve for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_exo_buy_curve(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoBuyCurve,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_buy_curve(
      self.program.payer(),
      collateral_mint,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the sell curve for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_exo_sell_curve(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoSellCurve,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_sell_curve(
      self.program.payer(),
      collateral_mint,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Updates the levercoin fees for an exo collateral.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn update_exo_levercoin_fees(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoLevercoinFees,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::update_exo_levercoin_fees(
      self.program.payer(),
      collateral_mint,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
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
  pub fn register_exo(
    &self,
    collateral_mint: Pubkey,
    exo_usd_pyth_feed: Pubkey,
    args: &args::RegisterExo,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::register_exo(
      self.program.payer(),
      collateral_mint,
      exo_usd_pyth_feed,
      args,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Withdraws accumulated fees to the treasury.
  ///
  /// # Errors
  /// - Failed to build transaction instructions
  pub fn withdraw_fees(
    &self,
    treasury: Pubkey,
    fee_token_mint: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::withdraw_fees(
      self.program.payer(),
      treasury,
      fee_token_mint,
    );
    Ok(VersionedTransactionData::one(instruction))
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
}
