use std::sync::Arc;

use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anchor_spl::token;
use anyhow::{anyhow, Result};
use fix::prelude::{UFix64, N6, *};
use hylo_core::pyth::SOL_USD_PYTH_FEED;
use hylo_idl::exchange::events::{
  RedeemLevercoinEventV2, RedeemStablecoinEventV2,
};
use hylo_idl::stability_pool::client::{accounts, args};
use hylo_idl::stability_pool::events::{
  StabilityPoolStats, UserDepositEvent, UserWithdrawEventV1,
};
use hylo_idl::tokens::{TokenMint, HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_idl::{exchange, pda, stability_pool};

use crate::exchange_client::ExchangeClient;
use crate::instruction_accounts;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::transaction::{
  BuildTransactionData, QuoteInput, RedeemArgs, SimulatePrice,
  SimulatePriceWithEnv, StabilityPoolArgs, TransactionSyntax,
};
use crate::util::{
  parse_event, simulation_config, EXCHANGE_LOOKUP_TABLE,
  LST_REGISTRY_LOOKUP_TABLE, REFERENCE_WALLET, STABILITY_POOL_LOOKUP_TABLE,
};

/// Client for interacting with the Hylo Stability Pool program.
///
/// Provides functionality for depositing and withdrawing sHYUSD from the
/// stability pool. Supports transaction execution and price simulation for
/// offchain quoting.
///
/// # Examples
///
/// ## Setup
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # fn setup_client() -> Result<StabilityPoolClient> {
/// let client = StabilityPoolClient::new_random_keypair(
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
/// # async fn execute_transaction(client: StabilityPoolClient) -> Result<Signature> {
/// // Deposit HYUSD → sHYUSD
/// let user = Pubkey::new_unique();
/// let signature = client.run_transaction::<HYUSD, SHYUSD>(StabilityPoolArgs {
///   amount: UFix64::new(100),
///   user,
/// }).await?;
/// # Ok(signature)
/// # }
/// ```
///
/// ## Price Quote
/// ```rust,no_run
/// use hylo_clients::prelude::*;
///
/// # async fn simulate_price(client: StabilityPoolClient) -> Result<()> {
/// // Get price quote for 1 HYUSD → sHYUSD
/// let price = client.quote::<HYUSD, SHYUSD>().await?;
/// # Ok(())
/// # }
/// ```
pub struct StabilityPoolClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for StabilityPoolClient {
  const PROGRAM_ID: Pubkey = stability_pool::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> StabilityPoolClient {
    StabilityPoolClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl StabilityPoolClient {
  /// Rebalances stability pool by swapping stablecoin to levercoin.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn rebalance_stable_to_lever(&self) -> Result<Signature> {
    let accounts = accounts::RebalanceStableToLever {
      payer: self.program.payer(),
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD::MINT,
      stablecoin_pool: *pda::HYUSD_POOL,
      pool_auth: *pda::POOL_AUTH,
      levercoin_pool: *pda::XSOL_POOL,
      fee_auth: pda::fee_auth(HYUSD::MINT),
      fee_vault: pda::fee_vault(HYUSD::MINT),
      levercoin_mint: XSOL::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      token_program: token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::RebalanceStableToLever {};
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    let tx_args = VersionedTransactionData {
      instructions,
      lookup_tables,
    };
    let sig = self.send_v0_transaction(&tx_args).await?;
    Ok(sig)
  }

  /// Rebalances levercoin from the stability pool back to stablecoin.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn rebalance_lever_to_stable(&self) -> Result<Signature> {
    let accounts = accounts::RebalanceLeverToStable {
      payer: self.program.payer(),
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD::MINT,
      stablecoin_pool: *pda::HYUSD_POOL,
      pool_auth: *pda::POOL_AUTH,
      levercoin_pool: *pda::XSOL_POOL,
      fee_auth: pda::fee_auth(HYUSD::MINT),
      fee_vault: pda::fee_vault(HYUSD::MINT),
      levercoin_mint: XSOL::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      token_program: token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::RebalanceLeverToStable {};
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    let tx_args = VersionedTransactionData {
      instructions,
      lookup_tables,
    };
    let sig = self.send_v0_transaction(&tx_args).await?;
    Ok(sig)
  }

  /// Simulates the `get_stats` instruction on the stability pool.
  ///
  /// # Errors
  /// - Simulation failure
  /// - Return data access or deserialization
  pub async fn get_stats(&self) -> Result<StabilityPoolStats> {
    let accounts = accounts::GetStats {
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD::MINT,
      levercoin_mint: XSOL::MINT,
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      lp_token_mint: SHYUSD::MINT,
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
impl BuildTransactionData<HYUSD, SHYUSD> for StabilityPoolClient {
  type Inputs = StabilityPoolArgs;

  async fn build(
    &self,
    StabilityPoolArgs { amount, user }: StabilityPoolArgs,
  ) -> Result<VersionedTransactionData> {
    let accounts = instruction_accounts::stability_pool_deposit(user);
    let args = args::UserDeposit {
      amount_stablecoin: amount.bits,
    };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData {
      instructions,
      lookup_tables,
    })
  }
}

impl SimulatePrice<HYUSD, SHYUSD> for StabilityPoolClient {
  type OutExp = N6;
  type Event = UserDepositEvent;
  fn from_event(e: &Self::Event) -> Result<UFix64<N6>> {
    Ok(UFix64::new(e.lp_token_minted.bits))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<SHYUSD, HYUSD> for StabilityPoolClient {
  type Inputs = StabilityPoolArgs;

  async fn build(
    &self,
    StabilityPoolArgs { amount, user }: StabilityPoolArgs,
  ) -> Result<VersionedTransactionData> {
    let accounts = instruction_accounts::stability_pool_withdraw(user);
    let args = args::UserWithdraw {
      amount_lp_token: amount.bits,
    };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData {
      instructions,
      lookup_tables,
    })
  }
}

impl SimulatePrice<SHYUSD, HYUSD> for StabilityPoolClient {
  type OutExp = N6;
  type Event = UserWithdrawEventV1;
  fn from_event(event: &Self::Event) -> Result<UFix64<N6>> {
    if event.levercoin_withdrawn.bits > 0 {
      Err(anyhow!(
        "Cannot quote sHYUSD/hyUSD: levercoin present in pool"
      ))
    } else {
      Ok(UFix64::new(event.stablecoin_withdrawn.bits))
    }
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<SHYUSD, JITOSOL> for StabilityPoolClient {
  type Inputs = (ExchangeClient, StabilityPoolArgs);

  async fn build(
    &self,
    (exchange, StabilityPoolArgs { amount, user }): (
      ExchangeClient,
      StabilityPoolArgs,
    ),
  ) -> Result<VersionedTransactionData> {
    let redeem_shyusd_args = self
      .build_transaction_data::<SHYUSD, HYUSD>(StabilityPoolArgs {
        amount,
        user,
      })
      .await?;
    let redeem_shyusd_tx = self
      .build_simulation_transaction(&user, &redeem_shyusd_args)
      .await?;
    let redeem_shyusd_sim = self
      .simulate_transaction_event::<UserWithdrawEventV1>(&redeem_shyusd_tx)
      .await?;
    let mut instructions = redeem_shyusd_args.instructions;
    if redeem_shyusd_sim.stablecoin_withdrawn.bits > 0 {
      let redeem_hyusd_args = exchange
        .build_transaction_data::<HYUSD, JITOSOL>(RedeemArgs {
          amount: UFix64::new(redeem_shyusd_sim.stablecoin_withdrawn.bits),
          user,
          slippage_config: None,
        })
        .await?;
      instructions.extend(redeem_hyusd_args.instructions);
    }
    if redeem_shyusd_sim.levercoin_withdrawn.bits > 0 {
      let redeem_xsol_args = exchange
        .build_transaction_data::<XSOL, JITOSOL>(RedeemArgs {
          amount: UFix64::new(redeem_shyusd_sim.levercoin_withdrawn.bits),
          user,
          slippage_config: None,
        })
        .await?;
      instructions.extend(redeem_xsol_args.instructions);
    }
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData {
      instructions,
      lookup_tables,
    })
  }
}

#[async_trait::async_trait]
impl SimulatePriceWithEnv<SHYUSD, JITOSOL> for StabilityPoolClient {
  type OutExp = N9;
  type Env = ExchangeClient;
  async fn simulate_with_env(
    &self,
    exchange: ExchangeClient,
  ) -> Result<UFix64<N9>> {
    let args = self
      .build_transaction_data::<SHYUSD, JITOSOL>((
        exchange,
        StabilityPoolArgs::quote_input(REFERENCE_WALLET),
      ))
      .await?;
    let tx = self
      .build_simulation_transaction(&REFERENCE_WALLET, &args)
      .await?;
    let rpc = self.program().rpc();
    let sim_result = rpc
      .simulate_transaction_with_config(&tx, simulation_config())
      .await?;
    let from_xsol = parse_event::<RedeemLevercoinEventV2>(&sim_result)
      .map_or(UFix64::zero(), |e| {
        UFix64::<N9>::new(e.collateral_withdrawn.bits)
      });
    let from_hyusd = parse_event::<RedeemStablecoinEventV2>(&sim_result)
      .map_or(UFix64::zero(), |e| {
        UFix64::<N9>::new(e.collateral_withdrawn.bits)
      });
    let total_out = from_hyusd
      .checked_add(&from_xsol)
      .ok_or(anyhow!("total_out overflow"))?;
    Ok(total_out)
  }
}

#[async_trait::async_trait]
impl TransactionSyntax for StabilityPoolClient {}
