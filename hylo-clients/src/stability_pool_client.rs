use std::sync::Arc;

use anchor_client::solana_sdk::address_lookup_table::AddressLookupTableAccount;
use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};
use anyhow::{anyhow, Result};
use fix::prelude::{UFix64, N6, *};
use hylo_core::idl::hylo_exchange::events::{
  RedeemLevercoinEventV2, RedeemStablecoinEventV2, SwapLeverToStableEventV1,
  SwapStableToLeverEventV1,
};
use hylo_core::idl::hylo_stability_pool::client::{accounts, args};
use hylo_core::idl::hylo_stability_pool::events::{
  StabilityPoolStats, UserDepositEvent, UserWithdrawEventV1,
};
use hylo_core::idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};
use hylo_core::idl::{hylo_exchange, hylo_stability_pool, pda};
use hylo_core::pyth::SOL_USD_PYTH_FEED;

use crate::exchange_client::ExchangeClient;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::transaction::{
  BuildTransactionData, QuoteInput, RedeemArgs, SimulatePrice,
  SimulatePriceWithEnv, StabilityPoolArgs, SwapArgs, TransactionSyntax,
};
use crate::util::{
  parse_event, simulation_config, user_ata_instruction, EXCHANGE_LOOKUP_TABLE,
  LST, LST_REGISTRY_LOOKUP_TABLE, REFERENCE_WALLET,
  STABILITY_POOL_LOOKUP_TABLE,
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
  const PROGRAM_ID: Pubkey = hylo_stability_pool::ID;

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
      hylo_exchange_program: hylo_exchange::ID,
      token_program: token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: hylo_stability_pool::ID,
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
      hylo_exchange_program: hylo_exchange::ID,
      token_program: token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: hylo_stability_pool::ID,
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
    let accounts = accounts::UserDeposit {
      user,
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD::MINT,
      levercoin_mint: XSOL::MINT,
      user_stablecoin_ata: pda::hyusd_ata(user),
      user_lp_token_ata: pda::shyusd_ata(user),
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      lp_token_auth: *pda::SHYUSD_AUTH,
      lp_token_mint: SHYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: hylo_exchange::ID,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: hylo_stability_pool::ID,
    };
    let args = args::UserDeposit {
      amount_stablecoin: amount.bits,
    };
    let ata = vec![user_ata_instruction(&user, &SHYUSD::MINT)];
    let program = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let instructions = [ata, program].concat();
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
    let accounts = accounts::UserWithdraw {
      user,
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD::MINT,
      user_stablecoin_ata: pda::hyusd_ata(user),
      fee_auth: pda::fee_auth(HYUSD::MINT),
      fee_vault: pda::fee_vault(HYUSD::MINT),
      user_lp_token_ata: pda::shyusd_ata(user),
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_mint: XSOL::MINT,
      levercoin_pool: *pda::XSOL_POOL,
      user_levercoin_ata: pda::xsol_ata(user),
      lp_token_auth: *pda::SHYUSD_AUTH,
      lp_token_mint: SHYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: hylo_exchange::ID,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: hylo_stability_pool::ID,
    };
    let args = args::UserWithdraw {
      amount_lp_token: amount.bits,
    };
    let ata = vec![
      user_ata_instruction(&user, &HYUSD::MINT),
      user_ata_instruction(&user, &XSOL::MINT),
    ];
    let program = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let instructions = [ata, program].concat();
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
impl BuildTransactionData<XSOL, SHYUSD> for StabilityPoolClient {
  type Inputs = (ExchangeClient, SwapArgs);

  /// Builds a composite transaction that swaps xSOL to hyUSD on the exchange
  /// program, then deposits the resulting hyUSD into the stability pool to mint
  /// sHYUSD.
  async fn build(
    &self,
    (exchange, SwapArgs { amount, user }): (ExchangeClient, SwapArgs),
  ) -> Result<VersionedTransactionData> {
    // First, figure out how much hyUSD the swap will mint so we can deposit
    // exactly that amount of hyUSD into the pool.
    let swap_args = exchange
      .build_transaction_data::<XSOL, HYUSD>(SwapArgs { amount, user })
      .await?;
    let swap_tx = exchange
      .build_simulation_transaction(&user, &swap_args)
      .await?;
    let swap_event = exchange
      .simulate_transaction_event::<SwapLeverToStableEventV1>(&swap_tx)
      .await?;
    let hyusd_out = UFix64::new(swap_event.stablecoin_minted_user.bits);
    if hyusd_out.bits == 0 {
      return Err(anyhow!("Swap produced zero hyUSD to deposit"));
    }
    // With the minted hyUSD known, build the stability-pool deposit leg.
    let deposit_args = self
      .build_transaction_data::<HYUSD, SHYUSD>(StabilityPoolArgs {
        amount: hyusd_out,
        user,
      })
      .await?;
    let VersionedTransactionData {
      mut instructions,
      mut lookup_tables,
    } = swap_args;
    instructions.extend(deposit_args.instructions);
    lookup_tables.extend(deposit_args.lookup_tables);
    let lookup_tables = dedup_lookup_tables(lookup_tables);
    Ok(VersionedTransactionData {
      instructions,
      lookup_tables,
    })
  }
}

#[async_trait::async_trait]
impl SimulatePriceWithEnv<XSOL, SHYUSD> for StabilityPoolClient {
  type OutExp = N6;
  type Env = ExchangeClient;

  /// Quotes the composite xSOL→sHYUSD flow by simulating the swap+deposit
  /// transaction with a reference wallet.
  async fn simulate_with_env(
    &self,
    exchange: ExchangeClient,
  ) -> Result<UFix64<N6>> {
    let args = self
      .build_transaction_data::<XSOL, SHYUSD>((
        exchange,
        SwapArgs::quote_input(REFERENCE_WALLET),
      ))
      .await?;
    let tx = self
      .build_simulation_transaction(&REFERENCE_WALLET, &args)
      .await?;
    let deposit = self
      .simulate_transaction_event::<UserDepositEvent>(&tx)
      .await?;
    Ok(UFix64::new(deposit.lp_token_minted.bits))
  }
}

#[async_trait::async_trait]
impl BuildTransactionData<SHYUSD, XSOL> for StabilityPoolClient {
  type Inputs = (ExchangeClient, StabilityPoolArgs);

  /// Builds a composite transaction that withdraws sHYUSD liquidity and swaps
  /// any resulting hyUSD into xSOL. Direct xSOL withdrawals from the pool are
  /// already handled by the base withdraw instruction.
  async fn build(
    &self,
    (exchange, StabilityPoolArgs { amount, user }): (
      ExchangeClient,
      StabilityPoolArgs,
    ),
  ) -> Result<VersionedTransactionData> {
    let withdraw_args = self
      .build_transaction_data::<SHYUSD, HYUSD>(StabilityPoolArgs {
        amount,
        user,
      })
      .await?;
    let withdraw_tx = self
      .build_simulation_transaction(&user, &withdraw_args)
      .await?;
    let withdraw_event = self
      .simulate_transaction_event::<UserWithdrawEventV1>(&withdraw_tx)
      .await?;
    let VersionedTransactionData {
      mut instructions,
      mut lookup_tables,
    } = withdraw_args;
    if withdraw_event.stablecoin_withdrawn.bits > 0 {
      // Swap any hyUSD we withdrew into xSOL for the user.
      let swap_args = exchange
        .build_transaction_data::<HYUSD, XSOL>(SwapArgs {
          amount: UFix64::new(withdraw_event.stablecoin_withdrawn.bits),
          user,
        })
        .await?;
      instructions.extend(swap_args.instructions);
      lookup_tables.extend(swap_args.lookup_tables);
    }
    let lookup_tables = dedup_lookup_tables(lookup_tables);
    Ok(VersionedTransactionData {
      instructions,
      lookup_tables,
    })
  }
}

#[async_trait::async_trait]
impl<OUT: LST> BuildTransactionData<SHYUSD, OUT> for StabilityPoolClient {
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
    let mut instructions = vec![user_ata_instruction(&user, &OUT::MINT)];
    instructions.extend(redeem_shyusd_args.instructions);

    // If simulated transaction yields hyUSD, redeem it to jitoSOL
    if redeem_shyusd_sim.stablecoin_withdrawn.bits > 0 {
      let redeem_hyusd_args = exchange
        .build_transaction_data::<HYUSD, OUT>(RedeemArgs {
          amount: UFix64::new(redeem_shyusd_sim.stablecoin_withdrawn.bits),
          user,
          slippage_config: None,
        })
        .await?;
      instructions.extend(vec![user_ata_instruction(&user, &HYUSD::MINT)]);
      instructions.extend(redeem_hyusd_args.instructions);
    }

    // If simulated transaction yields xSOL, redeem it to jitoSOL
    if redeem_shyusd_sim.levercoin_withdrawn.bits > 0 {
      let redeem_xsol_args = exchange
        .build_transaction_data::<XSOL, OUT>(RedeemArgs {
          amount: UFix64::new(redeem_shyusd_sim.levercoin_withdrawn.bits),
          user,
          slippage_config: None,
        })
        .await?;
      instructions.extend(vec![user_ata_instruction(&user, &XSOL::MINT)]);
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
impl<OUT: LST> SimulatePriceWithEnv<SHYUSD, OUT> for StabilityPoolClient {
  type OutExp = N9;
  type Env = ExchangeClient;
  async fn simulate_with_env(
    &self,
    exchange: ExchangeClient,
  ) -> Result<UFix64<N9>> {
    let args = self
      .build_transaction_data::<SHYUSD, OUT>((
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
impl SimulatePriceWithEnv<SHYUSD, XSOL> for StabilityPoolClient {
  type OutExp = N6;
  type Env = ExchangeClient;

  /// Quotes the sHYUSD→xSOL flow (withdraw then swap) using the reference
  /// wallet, capturing both direct xSOL withdrawals and xSOL minted via swapping
  /// hyUSD.
  async fn simulate_with_env(
    &self,
    exchange: ExchangeClient,
  ) -> Result<UFix64<N6>> {
    let args = self
      .build_transaction_data::<SHYUSD, XSOL>((
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
    let withdraw = parse_event::<UserWithdrawEventV1>(&sim_result)?;
    let swap = parse_event::<SwapStableToLeverEventV1>(&sim_result).ok();
    let swap_minted = swap
      .map(|event| UFix64::new(event.levercoin_minted.bits))
      .unwrap_or_else(UFix64::zero);
    let total_out = UFix64::new(withdraw.levercoin_withdrawn.bits)
      .checked_add(&swap_minted)
      .ok_or(anyhow!("total_out overflow"))?;
    Ok(total_out)
  }
}

/// Deduplicates lookup table accounts so the same table isn't included multiple
/// times in a composed transaction.
fn dedup_lookup_tables(
  tables: Vec<AddressLookupTableAccount>,
) -> Vec<AddressLookupTableAccount> {
  let mut deduped: Vec<AddressLookupTableAccount> = Vec::new();
  for table in tables {
    if deduped
      .iter()
      .any(|existing: &AddressLookupTableAccount| existing.key == table.key)
    {
      continue;
    }
    deduped.push(table);
  }
  deduped
}

#[async_trait::async_trait]
impl TransactionSyntax for StabilityPoolClient {}
