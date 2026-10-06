use std::sync::Arc;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::Result;
use hylo_core::idl::exchange;
use hylo_idl::exchange::client::args;
use hylo_idl::exchange::instruction_builders;
use hylo_idl::exchange::types::{AddressField, TokenMetadata, UFixValue64};
use hylo_idl::pda;
use hylo_idl::tokens::{TokenMint, HYUSD};

use crate::memo::build_memo;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::signing::{SignedTransaction, SigningClient, SigningMethod};
use crate::util::{
  ata_instruction, HYLO_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE,
};

/// Admin client for the Hylo exchange program. Manages LST
/// registration, oracle configuration, fee updates, and protocol
/// stats. User-facing operations go through
/// [`crate::router_client::RouterClient`].
pub struct ExchangeClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
  signing_method: SigningMethod,
}

impl ProgramClient for ExchangeClient {
  const PROGRAM_ID: Pubkey = exchange::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> ExchangeClient {
    ExchangeClient {
      program,
      keypair,
      signing_method: SigningMethod::Direct,
    }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl SigningClient for ExchangeClient {
  fn signing_method(&self) -> SigningMethod {
    self.signing_method
  }

  fn with_signing_method(mut self, signing_method: SigningMethod) -> Self {
    self.signing_method = signing_method;
    self
  }
}

impl ExchangeClient {
  /// Initializes the Hylo exchange protocol.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_protocol(
    &self,
    upgrade_authority: Pubkey,
    treasury: Pubkey,
    args: &args::InitializeProtocol,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::initialize_protocol(
      self.signer(),
      upgrade_authority,
      treasury,
      args,
    );
    let memo = build_memo("initialize_protocol", &instruction);
    self
      .sign(VersionedTransactionData::one(instruction), memo)
      .await
  }

  /// Initializes hyUSD and xSOL token mints.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_mints(
    &self,
    stablecoin_metadata: TokenMetadata,
    levercoin_metadata: TokenMetadata,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::initialize_mints(
      self.signer(),
      stablecoin_metadata,
      levercoin_metadata,
    );
    let memo = build_memo("initialize_mints", &instruction);
    self
      .sign(VersionedTransactionData::one(instruction), memo)
      .await
  }

  /// Initializes the LST registry lookup table.
  ///
  /// # Errors
  /// * Failed to get current slot
  /// * Failed to build transaction instructions
  pub async fn initialize_lst_registry(
    &self,
    slot: u64,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::initialize_lst_registry(slot, self.signer());
    let memo = build_memo("initialize_lst_registry", &instruction);
    self
      .sign(VersionedTransactionData::one(instruction), memo)
      .await
  }

  /// Initializes LST price calculators in registry.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_lst_registry_calculators(
    &self,
    lst_registry: Pubkey,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::initialize_lst_registry_calculators(
      lst_registry,
      self.signer(),
    );
    let memo = build_memo("initialize_lst_registry_calculators", &instruction);
    self
      .sign(VersionedTransactionData::one(instruction), memo)
      .await
  }

  /// Registers a new LST for mint/redeem.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  #[allow(clippy::too_many_arguments)]
  pub async fn register_lst(
    &self,
    lst_registry: Pubkey,
    lst_mint: Pubkey,
    lst_stake_pool_state: Pubkey,
    sanctum_calculator_program: Pubkey,
    sanctum_calculator_state: Pubkey,
    stake_pool_program: Pubkey,
    stake_pool_program_data: Pubkey,
    rebalance_fee: UFixValue64,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::register_lst(
      lst_mint,
      lst_stake_pool_state,
      sanctum_calculator_program,
      sanctum_calculator_state,
      stake_pool_program,
      stake_pool_program_data,
      lst_registry,
      self.signer(),
      rebalance_fee,
    );
    let memo = build_memo("register_lst", &instruction);
    let exchange_lut = self.load_lookup_table(&HYLO_LOOKUP_TABLE).await?;
    let inner =
      VersionedTransactionData::new(vec![instruction], vec![exchange_lut]);
    self.sign(inner, memo).await
  }

  /// Builds transaction data for LST price oracle crank.
  ///
  /// # Errors
  /// * Failed to build transaction data
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

  /// Builds transaction data for harvesting yield from LST vaults to earn
  /// pool.
  ///
  /// # Errors
  /// * Failed to build transaction data
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
  /// * Failed to build transaction instructions
  pub async fn update_oracle_conf_tolerance(
    &self,
    args: &args::UpdateOracleConfTolerance,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_oracle_conf_tolerance(self.signer(), args);
    let memo = build_memo("update_oracle_conf_tolerance", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the SOL/USD oracle address.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_sol_usd_oracle(
    &self,
    args: &args::UpdateSolUsdOracle,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_sol_usd_oracle(self.signer(), args);
    let memo = build_memo("update_sol_usd_oracle", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the LST swap fee.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_lst_swap_fee(
    &self,
    args: &args::UpdateLstSwapFee,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_lst_swap_fee(self.signer(), args);
    let memo = build_memo("update_lst_swap_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the levercoin fee configuration.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_levercoin_fees(
    &self,
    args: &args::UpdateLevercoinFees,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_levercoin_fees(self.signer(), args);
    let memo = build_memo("update_levercoin_fees", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the oracle staleness interval.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_oracle_interval(
    &self,
    args: &args::UpdateOracleInterval,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_oracle_interval(self.signer(), args);
    let memo = build_memo("update_oracle_interval", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the LST stablecoin mint threshold.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_lst_stablecoin_mint_threshold(
    &self,
    args: &args::UpdateLstStablecoinMintThreshold,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_lst_stablecoin_mint_threshold(
        self.signer(),
        args,
      );
    let memo = build_memo("update_lst_stablecoin_mint_threshold", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Pauses the protocol.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn pause_protocol(&self) -> Result<SignedTransaction> {
    let instruction = instruction_builders::pause_protocol(self.signer());
    let memo = build_memo("pause_protocol", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Unpauses the protocol.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn unpause_protocol(&self) -> Result<SignedTransaction> {
    let instruction = instruction_builders::unpause_protocol(self.signer());
    let memo = build_memo("unpause_protocol", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Pauses the LST pair.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn pause_lst_pair(&self) -> Result<SignedTransaction> {
    let instruction = instruction_builders::pause_lst_pair(self.signer());
    let memo = build_memo("pause_lst_pair", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Unpauses the LST pair.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn unpause_lst_pair(&self) -> Result<SignedTransaction> {
    let instruction = instruction_builders::unpause_lst_pair(self.signer());
    let memo = build_memo("unpause_lst_pair", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Pauses an EXO pair for the given collateral mint.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn pause_exo_pair(
    &self,
    collateral_mint: Pubkey,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::pause_exo_pair(self.signer(), collateral_mint);
    let memo = build_memo("pause_exo_pair", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Unpauses an EXO pair for the given collateral mint.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn unpause_exo_pair(
    &self,
    collateral_mint: Pubkey,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::unpause_exo_pair(self.signer(), collateral_mint);
    let memo = build_memo("unpause_exo_pair", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Pauses the USDC pair.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn pause_usdc_pair(&self) -> Result<SignedTransaction> {
    let instruction = instruction_builders::pause_usdc_pair(self.signer());
    let memo = build_memo("pause_usdc_pair", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Unpauses the USDC pair.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn unpause_usdc_pair(&self) -> Result<SignedTransaction> {
    let instruction = instruction_builders::unpause_usdc_pair(self.signer());
    let memo = build_memo("unpause_usdc_pair", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the LST buy curve configuration.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_lst_buy_curve_config(
    &self,
    args: &args::UpdateLstBuyCurveConfig,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_lst_buy_curve_config(self.signer(), args);
    let memo = build_memo("update_lst_buy_curve_config", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the LST sell curve configuration.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_lst_sell_curve_config(
    &self,
    args: &args::UpdateLstSellCurveConfig,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_lst_sell_curve_config(self.signer(), args);
    let memo = build_memo("update_lst_sell_curve_config", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the yield harvest configuration.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_yield_harvest_config(
    &self,
    args: &args::UpdateYieldHarvestConfig,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_yield_harvest_config(self.signer(), args);
    let memo = build_memo("update_yield_harvest_config", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the USDC oracle confidence tolerance.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_usdc_oracle_conf_tolerance(
    &self,
    args: &args::UpdateUsdcOracleConfTolerance,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_usdc_oracle_conf_tolerance(
      self.signer(),
      args,
    );
    let memo = build_memo("update_usdc_oracle_conf_tolerance", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the USDC oracle staleness interval.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_usdc_oracle_interval(
    &self,
    args: &args::UpdateUsdcOracleInterval,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_usdc_oracle_interval(self.signer(), args);
    let memo = build_memo("update_usdc_oracle_interval", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the USDC mint fee.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_usdc_mint_fee(
    &self,
    args: &args::UpdateUsdcMintFee,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_usdc_mint_fee(self.signer(), args);
    let memo = build_memo("update_usdc_mint_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the USDC redeem fee.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_usdc_redeem_fee(
    &self,
    args: &args::UpdateUsdcRedeemFee,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_usdc_redeem_fee(self.signer(), args);
    let memo = build_memo("update_usdc_redeem_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the USDC par tolerance.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_par_tolerance(
    &self,
    args: &args::UpdateParTolerance,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_par_tolerance(self.signer(), args);
    let memo = build_memo("update_par_tolerance", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the rebalance fee for an LST.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_lst_rebalance_fee(
    &self,
    lst_mint: Pubkey,
    args: &args::UpdateLstRebalanceFee,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_lst_rebalance_fee(
      self.signer(),
      lst_mint,
      args,
    );
    let memo = build_memo("update_lst_rebalance_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the borrow rate curve for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_borrow_rate_curve(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoBorrowRateCurve,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_exo_borrow_rate_curve(
      self.signer(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_borrow_rate_curve", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the borrow rate fee for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_borrow_rate_fee(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoBorrowRateFee,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_exo_borrow_rate_fee(
      self.signer(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_borrow_rate_fee", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the oracle for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_oracle(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracle,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_exo_oracle(
      self.signer(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_oracle", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the oracle confidence tolerance for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_oracle_conf_tolerance(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracleConfTolerance,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_exo_oracle_conf_tolerance(
      self.signer(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_oracle_conf_tolerance", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the oracle staleness interval for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_oracle_interval(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoOracleInterval,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_exo_oracle_interval(
      self.signer(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_oracle_interval", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the stablecoin mint threshold for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_stablecoin_mint_threshold(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoStablecoinMintThreshold,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_exo_stablecoin_mint_threshold(
        self.signer(),
        collateral_mint,
        args,
      );
    let memo = build_memo("update_exo_stablecoin_mint_threshold", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the buy curve for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_buy_curve(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoBuyCurve,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_exo_buy_curve(
      self.signer(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_buy_curve", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the sell curve for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_sell_curve(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoSellCurve,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_exo_sell_curve(
      self.signer(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_sell_curve", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the levercoin fees for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_levercoin_fees(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoLevercoinFees,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::update_exo_levercoin_fees(
      self.signer(),
      collateral_mint,
      args,
    );
    let memo = build_memo("update_exo_levercoin_fees", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Updates the levercoin market cap limit for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn update_exo_levercoin_market_cap_limit(
    &self,
    collateral_mint: Pubkey,
    args: &args::UpdateExoLevercoinMarketCapLimit,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::update_exo_levercoin_market_cap_limit(
        self.signer(),
        collateral_mint,
        args,
      );
    let memo =
      build_memo("update_exo_levercoin_market_cap_limit", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Initializes USDC support.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_usdc(
    &self,
    usdc_usd_pyth_feed: Pubkey,
    args: &args::InitializeUsdc,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::initialize_usdc(
      self.signer(),
      usdc_usd_pyth_feed,
      args,
    );
    let memo = build_memo("initialize_usdc", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Initializes the LST virtual stablecoin via Squads proposal.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_lst_virtual_stablecoin(
    &self,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::initialize_lst_virtual_stablecoin(self.signer());
    let memo = build_memo("initialize_lst_virtual_stablecoin", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Initializes the pool drawdown ledger for the LST pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_pool_drawdown_lst(
    &self,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::initialize_pool_drawdown_lst(self.signer());
    let memo = build_memo("initialize_pool_drawdown_lst", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Initializes the pool drawdown ledger for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn initialize_pool_drawdown_exo(
    &self,
    collateral_mint: Pubkey,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::initialize_pool_drawdown_exo(
      self.signer(),
      collateral_mint,
    );
    let memo = build_memo("initialize_pool_drawdown_exo", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Registers an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn register_exo(
    &self,
    collateral_mint: Pubkey,
    exo_usd_pyth_feed: Pubkey,
    args: &args::RegisterExo,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::register_exo(
      self.signer(),
      collateral_mint,
      exo_usd_pyth_feed,
      args,
    );
    let memo = build_memo("register_exo", &instruction);
    let exchange_lut = self.load_lookup_table(&HYLO_LOOKUP_TABLE).await?;
    let inner =
      VersionedTransactionData::new(vec![instruction], vec![exchange_lut]);
    self.sign(inner, memo).await
  }

  /// Seeds an empty exo pair with its initial collateral, minting
  /// levercoin and stablecoin to the dead address.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn genesis_mint_exo(
    &self,
    collateral_mint: Pubkey,
    collateral_usd_pyth_feed: Pubkey,
    args: &args::GenesisMintExo,
  ) -> Result<SignedTransaction> {
    let vault = self.signer();
    let levercoin_mint = pda::exo_levercoin_mint(collateral_mint);
    let dead_levercoin_ata =
      ata_instruction(&vault, &pda::DEAD, &levercoin_mint);
    let dead_stablecoin_ata = ata_instruction(&vault, &pda::DEAD, &HYUSD::MINT);
    let instruction = instruction_builders::genesis_mint_exo(
      vault,
      collateral_mint,
      collateral_usd_pyth_feed,
      args,
    );
    let memo = build_memo("genesis_mint_exo", &instruction);
    let inner = VersionedTransactionData::new(
      vec![dead_levercoin_ata, dead_stablecoin_ata, instruction],
      vec![],
    );
    self.sign(inner, memo).await
  }

  /// Withdraws accumulated fees to the treasury.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
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

  /// Harvests the borrow rate for an exo collateral.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn harvest_borrow_rate(
    &self,
    collateral_mint: Pubkey,
    collateral_usd_pyth_feed: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::harvest_borrow_rate(
      collateral_mint,
      collateral_usd_pyth_feed,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Settles the LST virtual stablecoin against the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn settle_virtual_stablecoin_lst(
    &self,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::settle_virtual_stablecoin_lst();
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Settles the USDC virtual stablecoin against the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn settle_virtual_stablecoin_usdc(
    &self,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::settle_virtual_stablecoin_usdc();
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Settles the exo virtual stablecoin against the earn pool.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub fn settle_virtual_stablecoin_exo(
    &self,
    collateral_mint: Pubkey,
    collateral_usd_pyth_feed: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let instruction = instruction_builders::settle_virtual_stablecoin_exo(
      collateral_mint,
      collateral_usd_pyth_feed,
    );
    Ok(VersionedTransactionData::one(instruction))
  }

  /// Proposes an update to a privileged protocol address.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn propose_address_update(
    &self,
    address_field: AddressField,
    new_address: Pubkey,
    ttl_secs: u64,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::propose_address_update(
      self.signer(),
      address_field,
      new_address,
      ttl_secs,
    );
    let memo = build_memo("propose_address_update", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Approves an outstanding address update proposal.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn approve_address_update(
    &self,
    new_address: Pubkey,
    address_field: AddressField,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::approve_address_update(
      self.signer(),
      new_address,
      address_field,
    );
    let memo = build_memo("approve_address_update", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Accepts an approved address update proposal. Rent on the proposal
  /// account refunds to the current admin.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn accept_address_update(
    &self,
    admin: Pubkey,
    address_field: AddressField,
  ) -> Result<SignedTransaction> {
    let instruction = instruction_builders::accept_address_update(
      self.signer(),
      admin,
      address_field,
    );
    let memo = build_memo("accept_address_update", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }

  /// Cancels an outstanding address update proposal.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  pub async fn cancel_address_update(
    &self,
    address_field: AddressField,
  ) -> Result<SignedTransaction> {
    let instruction =
      instruction_builders::cancel_address_update(self.signer(), address_field);
    let memo = build_memo("cancel_address_update", &instruction);
    let inner = VersionedTransactionData::one(instruction);
    self.sign(inner, memo).await
  }
}
